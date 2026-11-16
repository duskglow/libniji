/* Name lookups when no DNS server answers (spec §16.1, §16.2, §16.4; getaddrinfo(3), RFC 1035;
 * GitHub issue #8): the server's network is unreachable, the connection is refused, every server
 * times out, the server says SERVFAIL, the reply is truncated and TCP is refused, resolv.conf is
 * empty or missing, and /etc/hosts or nsswitch.conf is missing. Built against the host C library
 * and against libniji's products, the outputs must match line for line (gai_down_products.rs).
 * Black-box: every line is the result of a public function.
 *
 * Usage: gai_down DIR              write DIR/<scenario>/etc for every scenario and list them;
 *        gai_down DIR SCENARIO     with DIR/SCENARIO/etc bind-mounted over /etc in private user,
 *                                  mount and network namespaces where only lo is up
 *                                  (gai_down_run.sh): start the scenario's server on
 *                                  127.0.0.1:53, if any, and run the lookups. */
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

static char dir[512];

enum server { NONE, SILENT, SERVFAIL, TRUNC, ANSWER };

#define FAST "options timeout:1 attempts:1\n"
#define HOSTS "127.0.0.1 localhost\n192.0.2.200 local.test\n"

/* resolv, nsswitch, hosts: NULL = no such file. */
static const struct {
    const char *name, *resolv, *nsswitch, *hosts;
    enum server server;
} SCENARIOS[] = {
    {"unreach", "nameserver 192.0.2.1\n" FAST, "hosts: files dns\n", HOSTS, NONE},
    {"unreach_nohosts", "nameserver 192.0.2.1\n" FAST, "hosts: files dns\n", NULL, NONE},
    {"unreach_bare", "nameserver 192.0.2.1\n" FAST, NULL, NULL, NONE},
    {"unreach_v6", "nameserver 2001:db8::1\n" FAST, "hosts: files dns\n", HOSTS, NONE},
    {"refused", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", HOSTS, NONE},
    {"refused_nohosts", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", NULL, NONE},
    {"refused_unavail", "nameserver 127.0.0.1\n" FAST, "hosts: dns [!UNAVAIL=return] files\n", HOSTS, NONE},
    {"two_down", "nameserver 192.0.2.1\nnameserver 127.0.0.1\n" FAST, "hosts: files dns\n", HOSTS, NONE},
    {"timeout", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", HOSTS, SILENT},
    {"timeout_nohosts", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", NULL, SILENT},
    {"servfail", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", HOSTS, SERVFAIL},
    {"trunc_notcp", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", HOSTS, TRUNC},
    {"empty_resolv", "", "hosts: files dns\n", HOSTS, NONE},
    {"no_resolv", NULL, "hosts: files dns\n", HOSTS, NONE},
    {"no_resolv_nohosts", NULL, "hosts: files dns\n", NULL, NONE},
    /* The server comes up between two rounds: a failure must not be kept. */
    {"recover", "nameserver 127.0.0.1\n" FAST, "hosts: files dns\n", HOSTS, ANSWER},
    {"recover_unavail", "nameserver 127.0.0.1\n" FAST, "hosts: dns [!UNAVAIL=return] files\n", HOSTS, ANSWER},
};
static const int NSCEN = sizeof SCENARIOS / sizeof SCENARIOS[0];

static void put(const char *path, const char *text)
{
    FILE *f = fopen(path, "w");
    if (!f) {
        perror(path);
        exit(1);
    }
    fputs(text, f);
    fclose(f);
}

static void fixtures(void)
{
    for (int i = 0; i < NSCEN; i++) {
        char p[1024];
        snprintf(p, sizeof p, "%s/%s", dir, SCENARIOS[i].name);
        mkdir(p, 0755);
        snprintf(p, sizeof p, "%s/%s/etc", dir, SCENARIOS[i].name);
        mkdir(p, 0755);
        snprintf(p, sizeof p, "%s/%s/etc/services", dir, SCENARIOS[i].name);
        put(p, "http 80/tcp www\nhttps 443/tcp\n");
        if (SCENARIOS[i].resolv) {
            snprintf(p, sizeof p, "%s/%s/etc/resolv.conf", dir, SCENARIOS[i].name);
            put(p, SCENARIOS[i].resolv);
        }
        if (SCENARIOS[i].nsswitch) {
            snprintf(p, sizeof p, "%s/%s/etc/nsswitch.conf", dir, SCENARIOS[i].name);
            put(p, SCENARIOS[i].nsswitch);
        }
        if (SCENARIOS[i].hosts) {
            snprintf(p, sizeof p, "%s/%s/etc/hosts", dir, SCENARIOS[i].name);
            put(p, SCENARIOS[i].hosts);
        }
        printf("%s\n", SCENARIOS[i].name);
    }
}

/* ------------------------------------------------------------------ the server */

/* One response to `q` (`n` bytes) in `r`: 0 = none. */
static int respond(enum server mode, const unsigned char *q, int n, unsigned char *r)
{
    if (n < 12 || mode == SILENT)
        return 0;
    int at = 12;
    while (at < n && q[at])
        at += q[at] + 1;
    at += 5; /* the root label, type and class */
    if (at > n)
        return 0;
    int qtype = q[at - 4] << 8 | q[at - 3];
    memcpy(r, q, at);
    r[2] = 0x84 | (q[2] & 0x01);
    r[3] = 0x80;
    r[4] = 0, r[5] = 1;
    r[6] = r[7] = r[8] = r[9] = r[10] = r[11] = 0;
    if (mode == SERVFAIL) {
        r[3] = 0x82;
        return at;
    }
    if (mode == TRUNC) {
        r[2] |= 0x02;
        return at;
    }
    /* ANSWER: a.test is 192.0.2.10; 10.2.0.192.in-addr.arpa is a.test; nothing else exists. */
    static const unsigned char a_test[] = "\1a\4test";
    static const unsigned char ptr[] = "\00210\0012\0010\003192\7in-addr\4arpa";
    if (n - 12 == (int)sizeof a_test + 4 && memcmp(q + 12, a_test, sizeof a_test) == 0) {
        if (qtype == 1) {
            static const unsigned char rr[] = {0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 192, 0, 2, 10};
            memcpy(r + at, rr, sizeof rr);
            r[7] = 1;
            return at + (int)sizeof rr;
        }
        return at; /* NOERROR, no data */
    }
    if (n - 12 == (int)sizeof ptr + 4 && memcmp(q + 12, ptr, sizeof ptr) == 0 && qtype == 12) {
        static const unsigned char rr[] = {0xc0, 12, 0, 12, 0, 1, 0, 0, 0, 60, 0, 8, 1, 'a', 4, 't', 'e', 's', 't', 0};
        memcpy(r + at, rr, sizeof rr);
        r[7] = 1;
        return at + (int)sizeof rr;
    }
    r[3] = 0x83; /* NXDOMAIN */
    return at;
}

static void serve(enum server mode, int ready)
{
    int u = socket(AF_INET, SOCK_DGRAM, 0), t = -1;
    struct sockaddr_in a;
    memset(&a, 0, sizeof a);
    a.sin_family = AF_INET;
    a.sin_port = htons(53);
    inet_pton(AF_INET, "127.0.0.1", &a.sin_addr);
    if (bind(u, (struct sockaddr *)&a, sizeof a)) {
        perror("server bind");
        _exit(1);
    }
    if (mode == ANSWER) {
        int one = 1;
        t = socket(AF_INET, SOCK_STREAM, 0);
        setsockopt(t, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
        if (bind(t, (struct sockaddr *)&a, sizeof a) || listen(t, 8)) {
            perror("server listen");
            _exit(1);
        }
    }
    (void)!write(ready, "r", 1);
    close(ready);
    unsigned char q[4096], r[4096];
    for (;;) {
        struct pollfd p[2] = {{u, POLLIN, 0}, {t, POLLIN, 0}};
        if (poll(p, t < 0 ? 1 : 2, -1) < 0)
            continue;
        if (p[0].revents & POLLIN) {
            struct sockaddr_in from;
            socklen_t fl = sizeof from;
            int n = (int)recvfrom(u, q, sizeof q, 0, (struct sockaddr *)&from, &fl);
            int rl = respond(mode, q, n, r);
            if (rl > 0)
                sendto(u, r, rl, 0, (struct sockaddr *)&from, fl);
        }
        if (t >= 0 && (p[1].revents & POLLIN)) {
            int c = accept(t, NULL, NULL);
            unsigned char lb[2];
            while (recv(c, lb, 2, MSG_WAITALL) == 2) {
                int n = lb[0] << 8 | lb[1];
                if (recv(c, q, n, MSG_WAITALL) != n)
                    break;
                int rl = respond(mode, q, n, r);
                unsigned char hdr[2] = {(unsigned char)(rl >> 8), (unsigned char)rl};
                send(c, hdr, 2, 0);
                send(c, r, rl, 0);
            }
            close(c);
        }
    }
}

static pid_t start(enum server mode)
{
    int p[2];
    if (pipe(p))
        exit(1);
    fflush(stdout);
    pid_t pid = fork();
    if (pid == 0) {
        close(p[0]);
        serve(mode, p[1]);
        _exit(0);
    }
    close(p[1]);
    char c;
    if (read(p[0], &c, 1) != 1)
        exit(1);
    close(p[0]);
    return pid;
}

static void stop(pid_t pid)
{
    int st;
    kill(pid, SIGKILL);
    syscall(SYS_wait4, pid, &st, 0, NULL);
}

/* ------------------------------------------------------------------ lookups */

static const char *ename(int e)
{
    switch (e) {
    case 0: return "0";
    case ENOENT: return "ENOENT";
    case ECONNREFUSED: return "ECONNREFUSED";
    case ETIMEDOUT: return "ETIMEDOUT";
    case ENETUNREACH: return "ENETUNREACH";
    case EHOSTUNREACH: return "EHOSTUNREACH";
    case EAGAIN: return "EAGAIN";
    case EBUSY: return "EBUSY";
    case EADDRNOTAVAIL: return "EADDRNOTAVAIL";
    default: {
        static char b[16];
        snprintf(b, sizeof b, "%d", e);
        return b;
    }
    }
}

static void gai(const char *node, const char *serv, int family)
{
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = family;
    hints.ai_socktype = SOCK_STREAM;
    errno = 0;
    h_errno = 0;
    int r = getaddrinfo(node, serv, family < 0 ? NULL : &hints, &res);
    int e = errno;
    printf("getaddrinfo %s %s %s: %d (%s)", node, serv, family < 0 ? "NULL" : family == AF_INET ? "inet" : "unspec", r,
           r ? gai_strerror(r) : "ok");
    if (r == EAI_SYSTEM)
        printf(" errno=%s", ename(e));
    for (struct addrinfo *a = res; a; a = a->ai_next) {
        char t[64];
        if (a->ai_family == AF_INET)
            inet_ntop(AF_INET, &((struct sockaddr_in *)a->ai_addr)->sin_addr, t, sizeof t);
        else
            inet_ntop(AF_INET6, &((struct sockaddr_in6 *)a->ai_addr)->sin6_addr, t, sizeof t);
        printf(" %s", t);
    }
    if (res)
        freeaddrinfo(res);
    printf("\n");
}

static void host(const char *what, const char *arg, struct hostent *h, int e)
{
    if (!h) {
        printf("%s %s: NULL h_errno=%d errno=%s\n", what, arg, h_errno, ename(e));
        return;
    }
    char t[64];
    printf("%s %s: [%s] %s\n", what, arg, h->h_name, h->h_addr_list[0] ? inet_ntop(h->h_addrtype, h->h_addr_list[0], t, sizeof t) : "-");
}

static void byname(const char *name, int family)
{
    errno = 0;
    h_errno = 0;
    struct hostent *h = family == AF_INET ? gethostbyname(name) : gethostbyname2(name, family);
    host(family == AF_INET ? "gethostbyname" : "gethostbyname2 inet6", name, h, errno);
}

static void nameinfo(const char *ip, int flags)
{
    struct sockaddr_in s;
    memset(&s, 0, sizeof s);
    s.sin_family = AF_INET;
    s.sin_port = htons(80);
    inet_pton(AF_INET, ip, &s.sin_addr);
    char h[NI_MAXHOST], sv[NI_MAXSERV];
    errno = 0;
    int r = getnameinfo((struct sockaddr *)&s, sizeof s, h, sizeof h, sv, sizeof sv, flags);
    int e = errno;
    printf("getnameinfo %s %s: %d", ip, flags ? "NAMEREQD" : "0", r);
    if (r == 0)
        printf(" %s %s", h, sv);
    else
        printf(" (%s)", gai_strerror(r));
    if (r == EAI_SYSTEM)
        printf(" errno=%s", ename(e));
    printf("\n");
}

static void round_of_lookups(void)
{
    gai("www.gnu.org", "http", -1);
    gai("www.ibm.com", "https", -1);
    gai("localhost", "http", -1);
    gai("a.test", "80", AF_INET);
    gai("local.test", "80", AF_UNSPEC);
    byname("www.gnu.org", AF_INET);
    byname("localhost", AF_INET);
    byname("a.test", AF_INET6);
    struct in_addr a4;
    inet_pton(AF_INET, "192.0.2.10", &a4);
    errno = 0;
    h_errno = 0;
    struct hostent *h = gethostbyaddr(&a4, 4, AF_INET);
    host("gethostbyaddr", "192.0.2.10", h, errno);
    nameinfo("192.0.2.10", 0);
    nameinfo("192.0.2.10", NI_NAMEREQD);
}

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    snprintf(dir, sizeof dir, "%s", argc > 1 ? argv[1] : "/tmp");
    if (argc < 3) {
        fixtures();
        return 0;
    }
    int i = 0;
    while (i < NSCEN && strcmp(SCENARIOS[i].name, argv[2]) != 0)
        i++;
    if (i == NSCEN)
        return 2;
    printf("== scenario %s\n", argv[2]);
    enum server mode = SCENARIOS[i].server;
    if (mode == ANSWER) {
        /* Round 1 with nothing on port 53, round 2 with the server up. */
        printf("-- down\n");
        round_of_lookups();
        pid_t s = start(ANSWER);
        printf("-- up\n");
        round_of_lookups();
        stop(s);
    } else {
        pid_t s = mode == NONE ? 0 : start(mode);
        round_of_lookups();
        if (s)
            stop(s);
    }
    printf("done\n");
    return 0;
}
