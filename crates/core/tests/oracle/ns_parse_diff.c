/* ns_parse_diff.c: the DNS message parsers of libresolv (ns_initparse, ns_parserr, ns_skiprr,
 * ns_msg_getflag, dn_expand, dn_skipname, ns_name_unpack, ns_name_uncompress, ns_name_skip,
 * ns_get16/ns_get32, ns_put16/ns_put32) on a fixed corpus of well-formed and malformed messages
 * (RFC 1035 sections 4.1 and 4.1.4, RFC 2181, RFC 3596). Offline: no network, no configuration.
 * Black-box: every line is a result (return value, errno, the fields the parsers fill, offsets
 * relative to the message), so the reference and libniji must print the same text
 * (ns_parse_products.rs). Built three ways: against the host C library with -lresolv, against
 * libniji dynamically with -lresolv (libresolv.so.2), and statically against libniji's libc.a. */
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <arpa/nameser.h>
#include <resolv.h>

static unsigned char m[2048];
static int n;

static void b(int v) { m[n++] = (unsigned char) v; }
static void u16(unsigned v) { b(v >> 8); b(v & 0xff); }
static void u32(unsigned long v) { u16((v >> 16) & 0xffff); u16(v & 0xffff); }
static void hdr(unsigned id, unsigned flags, int qd, int an, int ns, int ar)
{
    n = 0;
    u16(id); u16(flags); u16(qd); u16(an); u16(ns); u16(ar);
}
/* A name in labels: dotted text, then a terminating root label unless `ptr` >= 0, in which case
 * the name ends with a compression pointer to `ptr`. "" is the root name (or the bare pointer). */
static void name(const char *s, int ptr)
{
    while (*s) {
        const char *d = strchr(s, '.');
        int l = d ? (int) (d - s) : (int) strlen(s);
        b(l);
        memcpy(m + n, s, l);
        n += l;
        s += l;
        if (d) s++;
    }
    if (ptr >= 0) u16(0xc000 | ptr); else b(0);
}
static void label(int len, int c) { b(len); memset(m + n, c, len); n += len; }
static void q(const char *nm, int ptr, unsigned type) { name(nm, ptr); u16(type); u16(1); }
/* An RR's fixed part; returns where its RDLENGTH is, for rdend(). */
static int rrh(const char *nm, int ptr, unsigned type, unsigned cls, unsigned long ttl)
{
    name(nm, ptr); u16(type); u16(cls); u32(ttl);
    int at = n;
    u16(0);
    return at;
}
static void rdend(int at) { int l = n - at - 2; m[at] = l >> 8; m[at + 1] = l & 0xff; }

static void show_name_fns(int at)
{
    char text[NS_MAXDNAME];
    unsigned char wire[NS_MAXCDNAME];
    const unsigned char *eom = m + n, *p = m + at;
    errno = 0;
    int r = dn_expand(m, eom, m + at, text, sizeof text);
    printf("  dn_expand@%d: %d", at, r);
    if (r < 0) printf(" errno=%d", errno); else printf(" [%s]", text);
    errno = 0;
    r = dn_expand(m, eom, m + at, text, 8);
    printf(" small: %d", r);
    if (r < 0) printf(" errno=%d", errno); else printf(" [%s]", text);
    errno = 0;
    r = dn_skipname(m + at, eom);
    printf(" dn_skipname: %d", r);
    if (r < 0) printf(" errno=%d", errno);
    printf("\n");
    errno = 0;
    r = ns_name_unpack(m, eom, m + at, wire, sizeof wire);
    printf("  ns_name_unpack@%d: %d", at, r);
    if (r < 0) {
        printf(" errno=%d", errno);
    } else {
        errno = 0;
        int t = ns_name_ntop(wire, text, sizeof text);
        printf(" ntop %d", t);
        if (t < 0) printf(" errno=%d", errno); else printf(" [%s]", text);
    }
    errno = 0;
    r = ns_name_uncompress(m, eom, m + at, text, sizeof text);
    printf(" uncompress: %d", r);
    if (r < 0) printf(" errno=%d", errno); else printf(" [%s]", text);
    errno = 0;
    r = ns_name_skip(&p, eom);
    printf(" skip: %d", r);
    if (r < 0) printf(" errno=%d", errno); else printf(" ->%d", (int) (p - m));
    printf("\n");
}

static void show_rr(ns_msg *h, int sect, int i)
{
    ns_rr rr;
    memset(&rr, 0, sizeof rr);
    errno = 0;
    int r = ns_parserr(h, (ns_sect) sect, i, &rr);
    printf("  parserr s%d #%d: %d", sect, i, r);
    if (r < 0) {
        printf(" errno=%d\n", errno);
        return;
    }
    long off = rr.rdata ? (long) (rr.rdata - m) : -1;
    printf(" [%s] type=%u class=%u ttl=%lu rdlen=%u rdata@%ld\n", rr.name, (unsigned) ns_rr_type(rr), (unsigned) ns_rr_class(rr),
           (unsigned long) ns_rr_ttl(rr), (unsigned) ns_rr_rdlen(rr), off);
}

static void parse(const char *what, int len)
{
    ns_msg h;
    printf("== %s (%d bytes)\n", what, len);
    n = len;
    errno = 0;
    int r = ns_initparse(m, len, &h);
    printf("  initparse: %d", r);
    if (r < 0) {
        printf(" errno=%d\n", errno);
    } else {
        printf(" id=%u size=%d counts=%d,%d,%d,%d flags", (unsigned) ns_msg_id(h), (int) ns_msg_size(h), ns_msg_count(h, ns_s_qd), ns_msg_count(h, ns_s_an),
               ns_msg_count(h, ns_s_ns), ns_msg_count(h, ns_s_ar));
        for (int f = ns_f_qr; f < ns_f_max; f++) printf(" %d", ns_msg_getflag(h, f));
        printf("\n");
        /* In order, one past the end of each section. */
        for (int s = ns_s_qd; s < ns_s_max; s++)
            for (int i = 0; i <= ns_msg_count(h, s) && i < 4; i++) show_rr(&h, s, i);
        /* Out of order: backwards in the answers, a section again after a later one, bad numbers. */
        show_rr(&h, ns_s_an, 1);
        show_rr(&h, ns_s_an, 0);
        show_rr(&h, ns_s_qd, 0);
        show_rr(&h, ns_s_ar, 0);
        /* rrnum -1: after another section, then again in the same section; other negatives. */
        show_rr(&h, ns_s_an, -1);
        show_rr(&h, ns_s_an, -1);
        show_rr(&h, ns_s_an, -2);
        show_rr(&h, ns_s_qd, -1);
        show_rr(&h, ns_s_qd, -1);
        show_rr(&h, ns_s_max, 0);
        show_rr(&h, -1, 0);
    }
    /* ns_skiprr over each section from the end of the header, as counted by the header. */
    if (len >= NS_HFIXEDSZ) {
        const unsigned char *p = m + NS_HFIXEDSZ, *eom = m + len;
        printf("  skiprr:");
        for (int s = 0; s < 4; s++) {
            int c = (m[4 + 2 * s] << 8) | m[5 + 2 * s];
            errno = 0;
            int k = ns_skiprr(p, eom, (ns_sect) s, c);
            printf(" s%d(%d)=%d", s, c, k);
            if (k < 0) {
                printf(" errno=%d", errno);
                break;
            }
            p += k;
        }
        printf(" end@%d\n", (int) (p - m));
        if (len > NS_HFIXEDSZ) show_name_fns(NS_HFIXEDSZ);
    }
}

int main(void)
{
    int at;

    /* Well-formed responses. */
    hdr(0x1234, 0x8180, 1, 2, 0, 0);
    q("www.example.com", -1, ns_t_a);
    /* The CNAME's data (web.example.com: a label, then a pointer to "example.com" at 16) is at 45:
     * header 12, question 17 + 4, the answer's owner pointer 2 and fixed part 10. The A record's
     * owner points at it. */
    at = rrh("", 12, ns_t_cname, 1, 300); name("web", 16); rdend(at);
    at = rrh("", 45, ns_t_a, 1, 3600); u32(0x5db8d822); rdend(at);
    parse("A via CNAME, compressed", n);

    hdr(0xbeef, 0x8583, 1, 0, 1, 2);
    q("nonexistent.example", -1, ns_t_aaaa);
    at = rrh("", 24, ns_t_soa, 1, 900); name("ns1", 24); name("hostmaster", 24); u32(2024010101); u32(7200); u32(3600); u32(1209600); u32(300); rdend(at);
    at = rrh("ns1", 24, ns_t_aaaa, 1, 60); u32(0x20010db8); u32(0); u32(0); u32(1); rdend(at);
    at = rrh("", -1, ns_t_opt, 1232, 0); rdend(at);
    parse("NXDOMAIN with SOA, AAAA glue and OPT", n);

    hdr(7, 0x8400, 1, 3, 0, 0);
    q("Example.ORG", -1, ns_t_mx);
    at = rrh("", 12, ns_t_mx, 1, 86400); u16(10); name("mail", 12); rdend(at);
    at = rrh("", 12, ns_t_txt, 1, 86400); b(5); memcpy(m + n, "v=spf", 5); n += 5; b(0); rdend(at);
    at = rrh("", 12, ns_t_ns, 1, 86400); name("a.iana-servers.net", -1); rdend(at);
    parse("MX, TXT, NS; mixed case", n);

    hdr(1, 0x0100, 1, 0, 0, 0);
    q("", -1, ns_t_ns);
    parse("query for the root's NS", n);

    hdr(0, 0, 0, 0, 0, 0);
    parse("header only, no records", n);
    parse("one byte past a 12-byte header is cut", 11);
    parse("empty message", 0);

    /* Every flag set. */
    hdr(0xffff, 0xffff, 0, 0, 0, 0);
    parse("all flag bits set", n);

    /* Truncations. */
    hdr(2, 0x8180, 1, 0, 0, 0);
    parse("question counted, none present", n);
    hdr(2, 0x8180, 1, 0, 0, 0);
    name("a.example", -1);
    parse("question without type and class", n);
    hdr(2, 0x8180, 1, 0, 0, 0);
    name("a.example", -1); u16(1);
    parse("question with type only", n);
    hdr(3, 0x8180, 1, 1, 0, 0);
    q("a.example", -1, ns_t_a);
    name("", 12); u16(1); u16(1); u16(0);
    parse("answer cut inside its fixed part", n);
    hdr(3, 0x8180, 1, 1, 0, 0);
    q("a.example", -1, ns_t_a);
    at = rrh("", 12, ns_t_a, 1, 1); u32(0x7f000001); rdend(at);
    m[at + 1] = 5;
    parse("RDLENGTH one past the end", n);
    hdr(3, 0x8180, 1, 1, 0, 0);
    q("a.example", -1, ns_t_a);
    at = rrh("", 12, ns_t_a, 1, 1); u32(0x7f000001); rdend(at);
    m[at] = 0xff; m[at + 1] = 0xff;
    parse("RDLENGTH 65535", n);
    hdr(3, 0x8180, 1, 3, 0, 0);
    q("a.example", -1, ns_t_a);
    at = rrh("", 12, ns_t_a, 1, 1); u32(0x7f000001); rdend(at);
    parse("three answers counted, one present", n);
    hdr(3, 0x8180, 1, 0xffff, 0, 0);
    q("a.example", -1, ns_t_a);
    parse("65535 answers counted, none present", n);
    hdr(3, 0x8180, 1, 1, 0, 0);
    q("a.example", -1, ns_t_a);
    at = rrh("", 12, ns_t_a, 1, 1); u32(0x7f000001); rdend(at);
    b(0xde); b(0xad); b(0xbe);
    parse("bytes after the last record", n);

    /* Names. */
    hdr(4, 0x8180, 1, 0, 0, 0);
    u16(0xc000 | 12); u16(1); u16(1);
    parse("pointer to itself", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    u16(0xc000 | 14); u16(0xc000 | 12); u16(1); u16(1);
    parse("two pointers in a loop", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    b(1); b('a'); u16(0xc000 | 12); u16(1); u16(1);
    parse("a label then a pointer back to it", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    u16(0xc000 | 0x3ff); u16(1); u16(1);
    parse("pointer past the end", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    u16(0xc000 | 2); u16(1); u16(1);
    parse("pointer into the header", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    u16(0xc000 | 16); u16(1); u16(1); name("fwd.example", -1);
    parse("pointer forward to a later name", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    b(30); b('a'); b('b');
    parse("label longer than the message", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    b(0x41); b('a'); b(0); u16(1); u16(1);
    parse("label type 01 (extended)", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    b(0x81); b('a'); b(0); u16(1); u16(1);
    parse("label type 10 (reserved)", n);
    hdr(4, 0x8180, 1, 0, 0, 0);
    b(0xc0);
    parse("pointer cut after its first byte", n);
    hdr(5, 0x8180, 1, 0, 0, 0);
    for (int i = 0; i < 4; i++) label(63, 'a' + i);
    b(0); u16(1); u16(1);
    parse("name of 257 bytes", n);
    hdr(5, 0x8180, 1, 0, 0, 0);
    for (int i = 0; i < 3; i++) label(63, 'a' + i);
    label(61, 'z');
    b(0); u16(1); u16(1);
    parse("name of 255 bytes", n);
    hdr(5, 0x8180, 1, 0, 0, 0);
    for (int i = 0; i < 3; i++) label(63, 'a' + i);
    label(62, 'z');
    b(0); u16(1); u16(1);
    parse("name of 256 bytes", n);
    hdr(5, 0x8180, 1, 0, 0, 0);
    label(63, 'x'); label(63, 'y'); b(0); u16(1); u16(1);
    at = n; label(63, 'p'); label(63, 'q'); u16(0xc000 | 12);
    parse("compressed name over 255 bytes in total", n);
    /* The name after the question, at `at`, is 128 + 129 = 257 bytes long once expanded. */
    printf("  (the name at %d)\n", at);
    show_name_fns(at);
    hdr(6, 0x8180, 1, 0, 0, 0);
    b(3); b('a'); b('.'); b('b'); b(3); b('x'); b(0); b('y'); b(2); b('\\'); b(0xe9); b(0); u16(1); u16(1);
    parse("labels holding a dot, a zero byte, a backslash and a high byte", n);

    /* ns_get16, ns_get32, ns_put16, ns_put32. */
    unsigned char w[4] = {0xfe, 0xdc, 0xba, 0x98};
    printf("== get/put\n  ns_get16 %u ns_get32 %lu", ns_get16(w), (unsigned long) ns_get32(w));
    ns_put16(0x1234, w);
    printf(" put16 %02x%02x%02x%02x", w[0], w[1], w[2], w[3]);
    ns_put32(0x89abcdefUL, w);
    printf(" put32 %02x%02x%02x%02x\n", w[0], w[1], w[2], w[3]);
    return 0;
}
