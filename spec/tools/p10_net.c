/* P10 of slice 12 (plan/headers.md section 3): differential evaluation of the C.9 macros of the network headers.
   `header_check.py --p10` builds this program twice, once on the build host's own headers (the reference) and once on
   libniji's headers, runs both and compares the output byte for byte.
   Covers: IN6_IS_ADDR_* over crafted and generated addresses, IN6_ARE_ADDR_EQUAL, CMSG_ALIGN/SPACE/LEN for 0..80 and
   the FIRSTHDR/NXTHDR/DATA walk over control buffers (offsets, never addresses), htons/ntohs/htonl/ntohl.
   No spec clause beyond plan/headers.md section 3, P10 and cmsg(3), RFC 3493 section 6.1. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

static void addr_row(const struct in6_addr *a) {
    printf("in6");
    for (int i = 0; i < 16; i++)
        printf(" %02x", a->s6_addr[i]);
    printf(": %d%d%d%d%d%d%d%d%d%d%d%d\n", !!IN6_IS_ADDR_UNSPECIFIED(a), !!IN6_IS_ADDR_LOOPBACK(a),
           !!IN6_IS_ADDR_MULTICAST(a), !!IN6_IS_ADDR_LINKLOCAL(a), !!IN6_IS_ADDR_SITELOCAL(a),
           !!IN6_IS_ADDR_V4MAPPED(a), !!IN6_IS_ADDR_V4COMPAT(a), !!IN6_IS_ADDR_MC_NODELOCAL(a),
           !!IN6_IS_ADDR_MC_LINKLOCAL(a), !!IN6_IS_ADDR_MC_SITELOCAL(a), !!IN6_IS_ADDR_MC_ORGLOCAL(a),
           !!IN6_IS_ADDR_MC_GLOBAL(a));
}

static void walk(unsigned char *buf, size_t n, const size_t *lens, int count) {
    struct msghdr m;
    memset(&m, 0, sizeof m);
    memset(buf, 0, n);
    size_t off = 0;
    for (int i = 0; i < count; i++) {
        struct cmsghdr *c = (struct cmsghdr *) (buf + off);
        c->cmsg_len = CMSG_LEN(lens[i]);
        c->cmsg_level = i;
        c->cmsg_type = 100 + i;
        off += CMSG_SPACE(lens[i]);
    }
    for (size_t cl = 0; cl <= off + 1; cl += (cl < 40 ? 1 : 8)) {
        m.msg_control = buf;
        m.msg_controllen = cl;
        printf("walk n=%d cl=%zu:", count, cl);
        for (struct cmsghdr *c = CMSG_FIRSTHDR(&m); c; c = CMSG_NXTHDR(&m, c))
            printf(" [%td %td %d %d]", (unsigned char *) c - buf, CMSG_DATA(c) - buf, c->cmsg_level, c->cmsg_type);
        printf("\n");
    }
}

int main(void) {
    struct in6_addr a;
    unsigned seed = 12345;
    /* The structured addresses: prefixes and scopes of RFC 4291, then random ones. */
    static const unsigned char p0[] = {0xff, 0x01}, p1[] = {0xff, 0x02}, p2[] = {0xff, 0x05}, p3[] = {0xff, 0x08},
                               p4[] = {0xff, 0x0e}, p5[] = {0xfe, 0x80}, p6[] = {0xfe, 0xc0}, p7[] = {0xfe, 0xbf},
                               p8[] = {0xfe, 0x7f}, p9[] = {0xff, 0xf1}, p10[] = {0xff, 0x00};
    const unsigned char *pref[] = {p0, p1, p2, p3, p4, p5, p6, p7, p8, p9, p10};
    for (int i = 0; i < 11; i++) {
        memset(&a, 0, sizeof a);
        a.s6_addr[0] = pref[i][0];
        a.s6_addr[1] = pref[i][1];
        addr_row(&a);
        a.s6_addr[15] = 1;
        addr_row(&a);
    }
    memset(&a, 0, sizeof a);
    addr_row(&a);
    a.s6_addr[15] = 1;
    addr_row(&a);
    a.s6_addr[15] = 2;
    addr_row(&a);
    memset(&a, 0, sizeof a);
    a.s6_addr[10] = a.s6_addr[11] = 0xff;
    addr_row(&a);
    a.s6_addr[12] = 10;
    addr_row(&a);
    memset(&a, 0, sizeof a);
    a.s6_addr[15] = 0;
    a.s6_addr[14] = 1;
    addr_row(&a);
    for (int k = 0; k < 400; k++) {
        for (int i = 0; i < 16; i++) {
            seed = seed * 1103515245u + 12345u;
            a.s6_addr[i] = (k % 3 == 0 && i > 1) ? 0 : (unsigned char) (seed >> 16);
        }
        addr_row(&a);
        struct in6_addr b = a;
        if (k % 2)
            b.s6_addr[k % 16] ^= 1;
        printf("eq %d\n", !!IN6_ARE_ADDR_EQUAL(&a, &b));
    }
    for (size_t l = 0; l <= 80; l++)
        printf("cmsg %zu: align %zu space %zu len %zu\n", l, (size_t) CMSG_ALIGN(l), (size_t) CMSG_SPACE(l), (size_t) CMSG_LEN(l));
    static unsigned char buf[1024] __attribute__((aligned(16)));
    static const size_t l1[] = {4}, l2[] = {0, 1, 8, 17}, l3[] = {16, 3, 24, 0, 7};
    walk(buf, sizeof buf, l1, 1);
    walk(buf, sizeof buf, l2, 4);
    walk(buf, sizeof buf, l3, 5);
    static const uint32_t v32[] = {0, 1, 0x12345678u, 0xdeadbeefu, 0xffffffffu, 0x80000000u};
    for (int i = 0; i < 6; i++)
        printf("swap32 %08x %08x %08x\n", (unsigned) v32[i], (unsigned) htonl(v32[i]), (unsigned) ntohl(v32[i]));
    for (unsigned v = 0; v < 65536; v += 257)
        printf("swap16 %04x %04x %04x\n", v, (unsigned) htons((uint16_t) v), (unsigned) ntohs((uint16_t) v));
    return 0;
}
