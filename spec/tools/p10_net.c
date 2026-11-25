/* P10 of slice 12 (plan/headers.md section 3): differential evaluation of the C.9 macros of the network headers.
   `header_check.py --p10` builds this program twice, once on the build host's own headers (the reference) and once on
   libniji's headers, runs both and compares the output byte for byte.
   Covers: IN6_IS_ADDR_* over crafted and generated addresses, IN6_ARE_ADDR_EQUAL, CMSG_ALIGN/SPACE/LEN for 0..80 and
   the FIRSTHDR/NXTHDR/DATA walk over control buffers (offsets, never addresses), htons/ntohs/htonl/ntohl; the IPv4
   class tests IN_CLASSA..IN_BADCLASS over every leading nibble, and the ICMP6_FILTER_* bit operations (HEADERS-NET-
   SYS-NAMES), with their result types and how often they evaluate each argument, and the forms: SETPASS and SETBLOCK in
   a comma expression, SETPASSALL and SETBLOCKALL as a compound statement before `else` (each compiles only in its form).
   No spec clause beyond plan/headers.md section 3, P10 and cmsg(3), RFC 3493 section 6.1, RFC 3542 section 3.2,
   RFC 1112 section 4. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/icmp6.h>
#include <arpa/inet.h>

static int n_arg, n_type, n_filter;
static struct icmp6_filter filt;
static uint32_t arg(uint32_t v) { n_arg++; return v; }
static int type_of(int t) { n_type++; return t; }
static struct icmp6_filter *filter(void) { n_filter++; return &filt; }

static void filter_row(const char *what) {
    const unsigned char *b = (const unsigned char *) &filt;
    printf("filter %s:", what);
    for (unsigned i = 0; i < sizeof filt; i++)
        printf("%02x", b[i]);
    printf("\n");
}

static void filters(int block_first) {
    if (block_first) ICMP6_FILTER_SETBLOCKALL(&filt) else ICMP6_FILTER_SETPASSALL(&filt)
    filter_row(block_first ? "blockall" : "passall");
    static const int types[] = {0, 1, 7, 8, 31, 32, 33, 63, 64, 128, 129, 200, 254, 255};
    for (unsigned i = 0; i < sizeof types / sizeof types[0]; i++) {
        int t = types[i], r = (block_first ? ICMP6_FILTER_SETPASS(t, &filt) : ICMP6_FILTER_SETBLOCK(t, &filt), 1);
        char w[24];
        snprintf(w, sizeof w, "%s %d", block_first ? "pass" : "block", t);
        filter_row(w);
        printf("filter will %d %d %d %d %d\n", r, ICMP6_FILTER_WILLPASS(t, &filt), ICMP6_FILTER_WILLBLOCK(t, &filt),
               ICMP6_FILTER_WILLPASS(t ^ 1, &filt), ICMP6_FILTER_WILLBLOCK(1 ? t ^ 2 : 0, &filt) * 2);
    }
}

static void classes(void) {
    for (uint32_t n = 0; n < 16; n++)
        for (int lo = 0; lo < 3; lo++) {
            uint32_t a = (n << 28) | (lo == 1 ? 0x0fffffffu : lo == 2 ? 0x01000001u : 0);
            printf("class %08x: %d %d %d %d %d %d %d\n", (unsigned) a, IN_CLASSA(a), IN_CLASSB(a), IN_CLASSC(a), IN_CLASSD(a),
                   IN_MULTICAST(a), IN_EXPERIMENTAL(a), IN_BADCLASS(a));
        }
    printf("class signed %d %d %d %d wide %d %d\n", IN_CLASSA(-1), IN_BADCLASS(-1), IN_MULTICAST(-268435456),
           2 * IN_CLASSD(1 ? 0xe0000000u : 0), IN_CLASSA(0x100000000LL), IN_MULTICAST(0x1e0000000LL));
    /* The argument is converted to in_addr_t: a floating value in its range classifies as that integer. */
    printf("class double %d %d %d\n", IN_CLASSA(1.5), IN_CLASSB(3.0e9), IN_MULTICAST(3758096385.0));
    n_arg = 0;
    int s = IN_CLASSA(arg(1)) + IN_CLASSB(arg(1)) + IN_CLASSC(arg(1)) + IN_CLASSD(arg(1)) + IN_MULTICAST(arg(1)) +
            IN_EXPERIMENTAL(arg(1)) + IN_BADCLASS(arg(1));
    printf("class evaluations %d sum %d int %d%d%d%d%d%d%d\n", n_arg, s,
           __builtin_types_compatible_p(__typeof__(IN_CLASSA(1)), int), __builtin_types_compatible_p(__typeof__(IN_CLASSB(1)), int),
           __builtin_types_compatible_p(__typeof__(IN_CLASSC(1)), int), __builtin_types_compatible_p(__typeof__(IN_CLASSD(1)), int),
           __builtin_types_compatible_p(__typeof__(IN_MULTICAST(1)), int),
           __builtin_types_compatible_p(__typeof__(IN_EXPERIMENTAL(1)), int),
           __builtin_types_compatible_p(__typeof__(IN_BADCLASS(1)), int));
}

static void filter_forms(void) {
    filters(1);
    filters(0);
    n_filter = 0;
    ICMP6_FILTER_SETBLOCKALL(filter());
    printf("filter evaluations blockall %d", n_filter);
    n_filter = 0;
    ICMP6_FILTER_SETPASSALL(filter());
    printf(" passall %d", n_filter);
    n_filter = n_type = 0;
    ICMP6_FILTER_SETBLOCK(type_of(40), filter());
    printf(" setblock %d %d", n_type, n_filter);
    n_filter = n_type = 0;
    ICMP6_FILTER_SETPASS(type_of(41), filter());
    printf(" setpass %d %d", n_type, n_filter);
    n_filter = n_type = 0;
    int r = ICMP6_FILTER_WILLPASS(type_of(40), filter());
    printf(" willpass %d %d", n_type, n_filter);
    n_filter = n_type = 0;
    r += ICMP6_FILTER_WILLBLOCK(type_of(40), filter());
    printf(" willblock %d %d (%d)\n", n_type, n_filter, r);
    const struct icmp6_filter *c = &filt;
    printf("filter types %d %d %d %d const %d\n", __builtin_types_compatible_p(__typeof__(ICMP6_FILTER_WILLPASS(1, c)), int),
           __builtin_types_compatible_p(__typeof__(ICMP6_FILTER_WILLBLOCK(1, c)), int),
           __builtin_types_compatible_p(__typeof__(ICMP6_FILTER_SETPASS(1, &filt)), unsigned int),
           __builtin_types_compatible_p(__typeof__(ICMP6_FILTER_SETBLOCK(1, &filt)), unsigned int),
           ICMP6_FILTER_WILLBLOCK(40, c));
}

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
    classes();
    filter_forms();
    return 0;
}
