/* getent: entries from the name service databases (spec §31, plan/utilities.md UTIL-6, D-31.4).
 *
 * A thin front end over the C library's public name service functions: every answer is what any
 * program gets from libniji's NSS (nsswitch.conf, the built-in `files` and `dns` services, the
 * cache); getent adds only the formatting. Built by `cargo xtask so` into target/niji/getent, a
 * dynamically linked program shipped in libc-bin as /usr/bin/getent.
 *
 * Behaviour (black-box probes of the reference, recorded as transcripts under
 * validation/transcripts/getent/): `getent [OPTION...] database [key ...]`. Exit 0 when every key
 * is found (or the enumeration ran), 1 for a usage error or an unknown database, 2 when a key is
 * not found, 3 when the database cannot be enumerated, 64 for an option error.
 */
#define _GNU_SOURCE
#include <aliases.h>
#include <arpa/inet.h>
#include <errno.h>
#include <grp.h>
#include <gshadow.h>
#include <limits.h>
#include <netdb.h>
#include <netinet/ether.h>
#include <netinet/in.h>
#include <pwd.h>
#include <rpc/netdb.h>
#include <shadow.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>

int __nss_configure_lookup(const char *db, const char *service_line);

static const char *prog_full = "getent";
static const char *prog_short = "getent";
static int addrconfig = 1;
static int idn = 1;

static const char HELP[] =
    "Usage: getent [OPTION...] database [key ...]\n"
    "Get entries from administrative database.\n"
    "\n"
    "  -A, --no-addrconfig        do not filter out unsupported IPv4/IPv6 addresses\n"
    "                             (with ahosts*)\n"
    "  -i, --no-idn               disable IDN encoding\n"
    "  -s, --service=CONFIG       Service configuration to be used\n"
    "  -?, --help                 Give this help list\n"
    "      --usage                Give a short usage message\n"
    "  -V, --version              Print program version\n"
    "\n"
    "Mandatory or optional arguments to long options are also mandatory or optional\n"
    "for any corresponding short options.\n"
    "\n"
    "Supported databases:\n"
    "ahosts ahostsv4 ahostsv6 aliases ethers group gshadow hosts initgroups\n"
    "netgroup networks passwd protocols rpc services shadow\n";

static const char USAGE[] =
    "Usage: getent [-Ai?V] [-s CONFIG] [--no-addrconfig] [--no-idn]\n"
    "            [--service=CONFIG] [--help] [--usage] [--version]\n"
    "            database [key ...]\n";

/* Identity string (D-31.4): libniji's own, the ABI level last. */
static const char VERSION[] = "getent (libniji) 2.39\n";

static void try_line(FILE *f) {
    fprintf(f, "Try `%s --help' or `%s --usage' for more information.\n", prog_short, prog_short);
}

static _Noreturn void option_error(void) {
    try_line(stderr);
    exit(64);
}

/* NSS database names `-s CONFIG` without a `db:` part configures. */
static const char *const NSS_DBS[] = {"aliases", "ethers", "group", "gshadow", "hosts", "initgroups", "netgroup",
                                      "networks", "passwd", "protocols", "rpc", "services", "shadow"};

static void service(const char *config) {
    const char *colon = strchr(config, ':');
    if (colon == NULL) {
        for (size_t i = 0; i < sizeof NSS_DBS / sizeof NSS_DBS[0]; i++)
            (void)__nss_configure_lookup(NSS_DBS[i], config);
        return;
    }
    size_t n = (size_t)(colon - config);
    char *db = malloc(n + 1);
    if (db == NULL) {
        fprintf(stderr, "%s: %s\n", prog_full, strerror(ENOMEM));
        exit(1);
    }
    memcpy(db, config, n);
    db[n] = '\0';
    int rc = __nss_configure_lookup(db, colon + 1);
    free(db);
    if (rc != 0) {
        fprintf(stderr, "%s: Unknown database name\n", prog_full);
        exit(1);
    }
}

/* The long options, in the order an ambiguity message lists them. */
struct longopt {
    const char *name;
    int has_arg;
    int code;
};
static const struct longopt LONGS[] = {
    {"service", 1, 's'}, {"no-idn", 0, 'i'}, {"no-addrconfig", 0, 'A'},
    {"help", 0, '?'},    {"usage", 0, 'u'},  {"version", 0, 'V'},
};
#define NLONGS (sizeof LONGS / sizeof LONGS[0])

static void act(int code, const char *arg) {
    switch (code) {
    case 'A':
        addrconfig = 0;
        break;
    case 'i':
        idn = 0;
        break;
    case 's':
        service(arg);
        break;
    case '?':
        fputs(HELP, stdout);
        exit(0);
    case 'u':
        fputs(USAGE, stdout);
        exit(0);
    case 'V':
        fputs(VERSION, stdout);
        exit(0);
    default:
        break;
    }
}

/* One `--name[=value]` argument at argv[*i]; advances *i past any separate value. */
static void long_option(int argc, char **argv, int *i) {
    const char *text = argv[*i] + 2;
    const char *eq = strchr(text, '=');
    size_t len = eq ? (size_t)(eq - text) : strlen(text);
    const struct longopt *hit = NULL;
    int matches = 0;
    for (size_t k = 0; k < NLONGS; k++) {
        if (strncmp(LONGS[k].name, text, len) != 0)
            continue;
        if (strlen(LONGS[k].name) == len) {
            hit = &LONGS[k];
            matches = 1;
            break;
        }
        if (matches++ == 0)
            hit = &LONGS[k];
    }
    if (matches == 0) {
        fprintf(stderr, "%s: unrecognized option '%s'\n", prog_full, argv[*i]);
        option_error();
    }
    if (matches > 1) {
        fprintf(stderr, "%s: option '%.*s' is ambiguous; possibilities:", prog_full, (int)len + 2, argv[*i]);
        for (size_t k = 0; k < NLONGS; k++)
            if (strncmp(LONGS[k].name, text, len) == 0)
                fprintf(stderr, " '--%s'", LONGS[k].name);
        fputc('\n', stderr);
        option_error();
    }
    const char *value = NULL;
    if (hit->has_arg) {
        if (eq != NULL) {
            value = eq + 1;
        } else if (*i + 1 < argc) {
            value = argv[++*i];
        } else {
            fprintf(stderr, "%s: option '--%s' requires an argument\n", prog_full, hit->name);
            option_error();
        }
    } else if (eq != NULL) {
        fprintf(stderr, "%s: option '--%s' doesn't allow an argument\n", prog_full, hit->name);
        option_error();
    }
    act(hit->code, value);
}

/* A cluster of short options at argv[*i]; advances *i past any separate value. */
static void short_options(int argc, char **argv, int *i) {
    for (const char *p = argv[*i] + 1; *p != '\0'; p++) {
        switch (*p) {
        case 'A':
        case 'i':
        case '?':
        case 'V':
            act(*p, NULL);
            break;
        case 's':
            if (p[1] != '\0') {
                act('s', p + 1);
            } else if (*i + 1 < argc) {
                act('s', argv[++*i]);
            } else {
                fprintf(stderr, "%s: option requires an argument -- '%c'\n", prog_full, *p);
                option_error();
            }
            return;
        default:
            fprintf(stderr, "%s: invalid option -- '%c'\n", prog_full, *p);
            option_error();
        }
    }
}

/* A decimal number filling the whole key (the probed rule: `0x10` and `10x` are names). */
static int number(const char *key, unsigned long *out) {
    char *end;
    if (*key == '\0')
        return 0;
    errno = 0;
    unsigned long v = strtoul(key, &end, 10);
    if (*end != '\0' || errno != 0)
        return 0;
    *out = v;
    return 1;
}

static void aliases_list(char **a) {
    for (; a != NULL && *a != NULL; a++)
        printf(" %s", *a);
}

/* ---- passwd, group, shadow, gshadow ------------------------------------------------------- */

static int db_passwd(int n, char **keys) {
    struct passwd *pw;
    if (n == 0) {
        setpwent();
        while ((pw = getpwent()) != NULL)
            putpwent(pw, stdout);
        endpwent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        unsigned long id;
        pw = number(keys[i], &id) ? getpwuid((uid_t)id) : getpwnam(keys[i]);
        if (pw == NULL)
            rc = 2;
        else
            putpwent(pw, stdout);
    }
    return rc;
}

static int db_group(int n, char **keys) {
    struct group *gr;
    if (n == 0) {
        setgrent();
        while ((gr = getgrent()) != NULL)
            putgrent(gr, stdout);
        endgrent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        unsigned long id;
        gr = number(keys[i], &id) ? getgrgid((gid_t)id) : getgrnam(keys[i]);
        if (gr == NULL)
            rc = 2;
        else
            putgrent(gr, stdout);
    }
    return rc;
}

static int db_shadow(int n, char **keys) {
    struct spwd *sp;
    if (n == 0) {
        setspent();
        while ((sp = getspent()) != NULL)
            putspent(sp, stdout);
        endspent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        sp = getspnam(keys[i]);
        if (sp == NULL)
            rc = 2;
        else
            putspent(sp, stdout);
    }
    return rc;
}

static int db_gshadow(int n, char **keys) {
    struct sgrp *sg;
    if (n == 0) {
        setsgent();
        while ((sg = getsgent()) != NULL)
            putsgent(sg, stdout);
        endsgent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        sg = getsgnam(keys[i]);
        if (sg == NULL)
            rc = 2;
        else
            putsgent(sg, stdout);
    }
    return rc;
}

static int db_initgroups(int n, char **keys) {
    if (n == 0) {
        fputs("Enumeration not supported on initgroups\n", stderr);
        return 3;
    }
    for (int i = 0; i < n; i++) {
        int count = 16;
        gid_t *list = NULL;
        int got;
        /* No base group: (gid_t)-1 is passed and left out of the listing. */
        for (;;) {
            gid_t *grown = realloc(list, (size_t)count * sizeof *list);
            if (grown == NULL) {
                free(list);
                fprintf(stderr, "%s: %s\n", prog_full, strerror(ENOMEM));
                exit(1);
            }
            list = grown;
            int want = count;
            got = getgrouplist(keys[i], (gid_t)-1, list, &want);
            if (got >= 0)
                break;
            count = want > count ? want : count * 2;
        }
        printf("%-21s", keys[i]);
        for (int k = 0; k < got; k++)
            if (list[k] != (gid_t)-1)
                printf(" %ld", (long)list[k]);
        putchar('\n');
        free(list);
    }
    return 0;
}

/* ---- hosts ------------------------------------------------------------------------------ */

static void print_host(const struct hostent *h) {
    char text[INET6_ADDRSTRLEN];
    for (char **a = h->h_addr_list; a != NULL && *a != NULL; a++) {
        if (inet_ntop(h->h_addrtype, *a, text, sizeof text) == NULL)
            continue;
        printf("%-15s %s", text, h->h_name);
        aliases_list(h->h_aliases);
        putchar('\n');
    }
}

static int hosts_enumerate(void) {
    struct hostent *h;
    sethostent(0);
    while ((h = gethostent()) != NULL)
        print_host(h);
    endhostent();
    return 0;
}

static int db_hosts(int n, char **keys) {
    if (n == 0)
        return hosts_enumerate();
    int rc = 0;
    for (int i = 0; i < n; i++) {
        unsigned char addr[sizeof(struct in6_addr)];
        struct hostent *h;
        if (inet_pton(AF_INET6, keys[i], addr) == 1)
            h = gethostbyaddr(addr, sizeof(struct in6_addr), AF_INET6);
        else if (inet_pton(AF_INET, keys[i], addr) == 1)
            h = gethostbyaddr(addr, sizeof(struct in_addr), AF_INET);
        else if ((h = gethostbyname2(keys[i], AF_INET6)) == NULL)
            h = gethostbyname2(keys[i], AF_INET);
        if (h == NULL)
            rc = 2;
        else
            print_host(h);
    }
    return rc;
}

static int ahosts(int n, char **keys, int family) {
    if (n == 0)
        return hosts_enumerate();
    int rc = 0;
    for (int i = 0; i < n; i++) {
        struct addrinfo hints, *res;
        memset(&hints, 0, sizeof hints);
        hints.ai_family = family;
        hints.ai_flags = AI_CANONNAME | (addrconfig ? AI_ADDRCONFIG : 0) | (idn ? AI_IDN : 0) |
                         (family == AF_INET6 ? AI_V4MAPPED : 0);
        if (getaddrinfo(keys[i], NULL, &hints, &res) != 0) {
            rc = 2;
            continue;
        }
        for (struct addrinfo *a = res; a != NULL; a = a->ai_next) {
            char text[INET6_ADDRSTRLEN];
            const void *p = a->ai_family == AF_INET6 ? (const void *)&((const struct sockaddr_in6 *)a->ai_addr)->sin6_addr
                                                     : (const void *)&((const struct sockaddr_in *)a->ai_addr)->sin_addr;
            if (inet_ntop(a->ai_family, p, text, sizeof text) == NULL)
                continue;
            const char *type;
            char num[16];
            switch (a->ai_socktype) {
            case SOCK_STREAM:
                type = "STREAM";
                break;
            case SOCK_DGRAM:
                type = "DGRAM";
                break;
            case SOCK_RAW:
                type = "RAW";
                break;
            default:
                snprintf(num, sizeof num, "%d", a->ai_socktype);
                type = num;
                break;
            }
            printf("%-15s %-6s %s\n", text, type, a->ai_canonname != NULL ? a->ai_canonname : "");
        }
        freeaddrinfo(res);
    }
    return rc;
}

static int db_ahosts(int n, char **keys) { return ahosts(n, keys, AF_UNSPEC); }
static int db_ahostsv4(int n, char **keys) { return ahosts(n, keys, AF_INET); }
static int db_ahostsv6(int n, char **keys) { return ahosts(n, keys, AF_INET6); }

/* ---- services, protocols, networks, rpc, ethers -------------------------------------------- */

static void print_service(const struct servent *s) {
    printf("%-21s %d/%s", s->s_name, ntohs((uint16_t)s->s_port), s->s_proto);
    aliases_list(s->s_aliases);
    putchar('\n');
}

static int db_services(int n, char **keys) {
    struct servent *s;
    if (n == 0) {
        setservent(0);
        while ((s = getservent()) != NULL)
            print_service(s);
        endservent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        char *name = strdup(keys[i]);
        if (name == NULL) {
            fprintf(stderr, "%s: %s\n", prog_full, strerror(ENOMEM));
            exit(1);
        }
        char *proto = strchr(name, '/');
        if (proto != NULL)
            *proto++ = '\0';
        unsigned long port;
        s = number(name, &port) ? getservbyport((int)htons((uint16_t)port), proto) : getservbyname(name, proto);
        if (s == NULL)
            rc = 2;
        else
            print_service(s);
        free(name);
    }
    return rc;
}

static void print_proto(const struct protoent *p) {
    printf("%-21s %d", p->p_name, p->p_proto);
    aliases_list(p->p_aliases);
    putchar('\n');
}

static int db_protocols(int n, char **keys) {
    struct protoent *p;
    if (n == 0) {
        setprotoent(0);
        while ((p = getprotoent()) != NULL)
            print_proto(p);
        endprotoent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        unsigned long v;
        p = number(keys[i], &v) ? getprotobynumber((int)v) : getprotobyname(keys[i]);
        if (p == NULL)
            rc = 2;
        else
            print_proto(p);
    }
    return rc;
}

static void print_net(const struct netent *e) {
    struct in_addr a;
    a.s_addr = htonl(e->n_net);
    printf("%-21s %s", e->n_name, inet_ntoa(a));
    aliases_list(e->n_aliases);
    putchar('\n');
}

static int db_networks(int n, char **keys) {
    struct netent *e;
    if (n == 0) {
        setnetent(0);
        while ((e = getnetent()) != NULL)
            print_net(e);
        endnetent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        in_addr_t net = inet_network(keys[i]);
        e = net != INADDR_NONE ? getnetbyaddr(net, AF_INET) : getnetbyname(keys[i]);
        if (e == NULL)
            rc = 2;
        else
            print_net(e);
    }
    return rc;
}

static void print_rpc(const struct rpcent *r) {
    printf("%-15s %d", r->r_name, r->r_number);
    if (r->r_aliases != NULL && r->r_aliases[0] != NULL)
        putchar(' ');
    aliases_list(r->r_aliases);
    putchar('\n');
}

static int db_rpc(int n, char **keys) {
    struct rpcent *r;
    if (n == 0) {
        setrpcent(0);
        while ((r = getrpcent()) != NULL)
            print_rpc(r);
        endrpcent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        unsigned long v;
        r = number(keys[i], &v) ? getrpcbynumber((int)v) : getrpcbyname(keys[i]);
        if (r == NULL)
            rc = 2;
        else
            print_rpc(r);
    }
    return rc;
}

static int db_ethers(int n, char **keys) {
    if (n == 0) {
        fputs("Enumeration not supported on ethers\n", stderr);
        return 3;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        struct ether_addr addr;
        char host[1025];
        struct ether_addr *parsed = ether_aton(keys[i]);
        if (parsed != NULL) {
            addr = *parsed;
            if (ether_ntohost(host, &addr) != 0) {
                rc = 2;
                continue;
            }
            printf("%s %s\n", ether_ntoa(&addr), host);
        } else if (ether_hostton(keys[i], &addr) == 0) {
            printf("%s %s\n", ether_ntoa(&addr), keys[i]);
        } else {
            rc = 2;
        }
    }
    return rc;
}

/* ---- aliases, netgroup ------------------------------------------------------------------- */

static void print_alias(const struct aliasent *a) {
    int w = printf("%s:", a->alias_name);
    printf("%*s ", w < 15 ? 15 - w : 0, "");
    for (size_t k = 0; k < a->alias_members_len; k++)
        printf("%s%s", k == 0 ? "" : ", ", a->alias_members[k]);
    putchar('\n');
}

static int db_aliases(int n, char **keys) {
    struct aliasent *a;
    if (n == 0) {
        setaliasent();
        while ((a = getaliasent()) != NULL)
            print_alias(a);
        endaliasent();
        return 0;
    }
    int rc = 0;
    for (int i = 0; i < n; i++) {
        a = getaliasbyname(keys[i]);
        if (a == NULL)
            rc = 2;
        else
            print_alias(a);
    }
    return rc;
}

static int db_netgroup(int n, char **keys) {
    if (n == 0) {
        fputs("Enumeration not supported on netgroup\n", stderr);
        return 3;
    }
    if (n == 1) {
        char *host, *user, *domain;
        if (setnetgrent(keys[0]) == 0) {
            endnetgrent();
            return 2;
        }
        int any = 0;
        while (getnetgrent(&host, &user, &domain) == 1) {
            if (!any)
                printf("%-21s", keys[0]);
            any = 1;
            printf(" (%s,%s,%s)", host != NULL ? host : " ", user != NULL ? user : "", domain != NULL ? domain : "");
        }
        endnetgrent();
        if (!any)
            return 2;
        putchar('\n');
        return 0;
    }
    if (n == 4) {
        int r = innetgr(keys[0], keys[1], keys[2], keys[3]);
        printf("%-21s (%s,%s,%s) = %d\n", keys[0], keys[1], keys[2], keys[3], r);
    }
    return 0;
}

/* ---- main ------------------------------------------------------------------------------- */

struct database {
    const char *name;
    int (*run)(int n, char **keys);
};
static const struct database DATABASES[] = {
    {"ahosts", db_ahosts},       {"ahostsv4", db_ahostsv4},     {"ahostsv6", db_ahostsv6}, {"aliases", db_aliases},
    {"ethers", db_ethers},       {"group", db_group},           {"gshadow", db_gshadow},   {"hosts", db_hosts},
    {"initgroups", db_initgroups}, {"netgroup", db_netgroup},   {"networks", db_networks}, {"passwd", db_passwd},
    {"protocols", db_protocols}, {"rpc", db_rpc},               {"services", db_services}, {"shadow", db_shadow},
};

int main(int argc, char **argv) {
    if (argc > 0 && argv[0] != NULL) {
        prog_full = argv[0];
        const char *slash = strrchr(argv[0], '/');
        prog_short = slash != NULL ? slash + 1 : argv[0];
    }
    /* Options anywhere before `--` (the operands keep their order). */
    char **operands = calloc((size_t)argc + 1, sizeof *operands);
    if (operands == NULL) {
        fprintf(stderr, "%s: %s\n", prog_full, strerror(ENOMEM));
        return 1;
    }
    int count = 0;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (strcmp(a, "--") == 0) {
            while (++i < argc)
                operands[count++] = argv[i];
            break;
        }
        if (a[0] == '-' && a[1] == '-')
            long_option(argc, argv, &i);
        else if (a[0] == '-' && a[1] != '\0')
            short_options(argc, argv, &i);
        else
            operands[count++] = argv[i];
    }
    if (count == 0) {
        fprintf(stderr, "%s: wrong number of arguments\n", prog_full);
        try_line(stdout);
        return 1;
    }
    for (size_t k = 0; k < sizeof DATABASES / sizeof DATABASES[0]; k++)
        if (strcmp(operands[0], DATABASES[k].name) == 0)
            return DATABASES[k].run(count - 1, operands + 1);
    fprintf(stderr, "Unknown database: %s\n", operands[0]);
    try_line(stdout);
    return 1;
}
