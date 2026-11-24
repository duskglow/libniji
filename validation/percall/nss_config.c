// Per-call counts of the lookups that read the switch configuration and gai.conf's policy
// (CONFIG-TABLES-GROWTH): each lookup decodes its nsswitch.conf line, and getaddrinfo with
// two answers sorts them by the gai.conf tables.
#include "percall.h"
#include <netdb.h>
#include <pwd.h>
#include <string.h>
#include <sys/socket.h>

CASE(getaddrinfo_localhost, 2000, 1) {
    struct addrinfo h;
    memset(&h, 0, sizeof h);
    h.ai_socktype = SOCK_STREAM;
    for (long i = 0; i < n; i++) {
        struct addrinfo *r = 0;
        if (getaddrinfo("localhost", "80", &h, &r) == 0) {
            pc_sink += (long)r->ai_family;
            freeaddrinfo(r);
        }
    }
    PC_CHECKING {
        struct addrinfo *r = 0;
        int e = getaddrinfo("localhost", "80", &h, &r);
        int k = 0;
        for (struct addrinfo *p = r; p; p = p->ai_next) k++;
        pc_check("%d %d", e, k);
        if (r) freeaddrinfo(r);
    }
}

CASE(getpwnam_r_root, 2000, 1) {
    struct passwd pw, *res;
    char buf[1024];
    for (long i = 0; i < n; i++) {
        res = 0;
        getpwnam_r("root", &pw, buf, sizeof buf, &res);
        pc_sink += (long)(res != 0);
    }
    PC_CHECKING {
        res = 0;
        getpwnam_r("root", &pw, buf, sizeof buf, &res);
        pc_check("%s", res ? res->pw_dir : "(none)");
    }
}
