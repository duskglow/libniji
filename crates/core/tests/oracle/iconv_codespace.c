/* The whole code space of each charset named on the command line, as iconv converts it, built
 * once against the reference C library and once against libniji (tests/iconv_codespace_products.rs
 * compares the two outputs line by line). Per charset:
 *   D <seq> <cp>...   a complete sequence and the code points it decodes to (pivot UCS-4LE)
 *   I <seq> <used>    EILSEQ for this sequence; <used> is the input consumed before it
 *   T/P <seq>         a sequence still incomplete at 4 bytes / any other outcome
 *   G <seq> <cp>... r=<ret> e=<errno> left=<n>   the same invalid sequence and "A" with //IGNORE
 *   E <cp> <seq>      the encoding of a code point (U+0000..U+10FFFF, no surrogates)
 *   B <block> <hash>  per 256 code points: a hash of each code point's encoding with //TRANSLIT
 *                     and with //IGNORE (output, return value, errno, input left)
 * Sequences grow a byte at a time while iconv reports EINVAL, up to 4 bytes. */
#include <errno.h>
#include <iconv.h>
#include <stdio.h>
#include <string.h>

static iconv_t dec, dec_ign, enc, enc_tr, enc_ign;

static void hex(const unsigned char *s, size_t n) {
    for (size_t i = 0; i < n; i++) printf("%02X", s[i]);
}

static void ignored(const unsigned char *seq, size_t n) {
    unsigned char in[8];
    unsigned int out[16];
    memcpy(in, seq, n);
    in[n] = 'A';
    char *ip = (char *)in, *op = (char *)out;
    size_t il = n + 1, ol = sizeof out;
    iconv(dec_ign, NULL, NULL, NULL, NULL);
    errno = 0;
    size_t r = iconv(dec_ign, &ip, &il, &op, &ol);
    int e = r == (size_t)-1 ? errno : 0;
    printf("G ");
    hex(seq, n);
    for (unsigned int *p = out; (char *)p < op; p++) printf(" %X", *p);
    printf(" r=%ld e=%d left=%zu\n", (long)r, e, il);
}

static void walk(unsigned char *seq, size_t n) {
    for (int b = 0; b < 256; b++) {
        seq[n - 1] = (unsigned char)b;
        unsigned int out[8];
        char *ip = (char *)seq, *op = (char *)out;
        size_t il = n, ol = sizeof out;
        iconv(dec, NULL, NULL, NULL, NULL);
        size_t r = iconv(dec, &ip, &il, &op, &ol);
        if (r != (size_t)-1 && il == 0) {
            printf("D ");
            hex(seq, n);
            for (unsigned int *p = out; (char *)p < op; p++) printf(" %X", *p);
            printf("\n");
        } else if (r == (size_t)-1 && errno == EINVAL) {
            if (n < 4) walk(seq, n + 1);
            else { printf("T "); hex(seq, n); printf("\n"); }
        } else if (r == (size_t)-1 && errno == EILSEQ) {
            printf("I "); hex(seq, n); printf(" %zu\n", n - il);
            ignored(seq, n);
        } else {
            printf("P "); hex(seq, n); printf(" %zu\n", n - il);
        }
    }
}

static unsigned long long fnv(unsigned long long h, const void *p, size_t n) {
    const unsigned char *s = p;
    for (size_t i = 0; i < n; i++) h = (h ^ s[i]) * 1099511628211ULL;
    return h;
}

/* Encode c with cd; fold the outcome into h. Returns whether it succeeded. */
static int encode(iconv_t cd, unsigned int c, unsigned char *out, size_t *len, unsigned long long *h) {
    char *ip = (char *)&c, *op = (char *)out;
    size_t il = 4, ol = 64;
    iconv(cd, NULL, NULL, NULL, NULL);
    errno = 0;
    size_t r = iconv(cd, &ip, &il, &op, &ol);
    int e = r == (size_t)-1 ? errno : 0;
    if (r != (size_t)-1) iconv(cd, NULL, NULL, &op, &ol);
    *len = (size_t)(op - (char *)out);
    if (h) {
        long rr = (long)r;
        *h = fnv(*h, out, *len);
        *h = fnv(*h, &rr, sizeof rr);
        *h = fnv(*h, &e, sizeof e);
        *h = fnv(*h, &il, sizeof il);
    }
    return r != (size_t)-1;
}

int main(int argc, char **argv) {
    for (int a = 1; a < argc; a++) {
        const char *cs = argv[a];
        char tr[96], ig[96];
        snprintf(tr, sizeof tr, "%s//TRANSLIT", cs);
        snprintf(ig, sizeof ig, "%s//IGNORE", cs);
        dec = iconv_open("UCS-4LE", cs);
        dec_ign = iconv_open("UCS-4LE//IGNORE", cs);
        enc = iconv_open(cs, "UCS-4LE");
        enc_tr = iconv_open(tr, "UCS-4LE");
        enc_ign = iconv_open(ig, "UCS-4LE");
        if (dec == (iconv_t)-1 || dec_ign == (iconv_t)-1 || enc == (iconv_t)-1 || enc_tr == (iconv_t)-1 || enc_ign == (iconv_t)-1) {
            printf("charset %s: cannot open\n", cs);
            continue;
        }
        printf("charset %s\n", cs);
        unsigned char seq[4];
        walk(seq, 1);
        for (unsigned int block = 0; block < 0x1100; block++) {
            unsigned long long h = 14695981039346656037ULL;
            for (unsigned int c = block << 8; c < (block + 1) << 8; c++) {
                if (c >= 0xD800 && c < 0xE000) continue;
                unsigned char out[64];
                size_t n;
                if (encode(enc, c, out, &n, NULL)) {
                    printf("E %X ", c);
                    hex(out, n);
                    printf("\n");
                }
                encode(enc_tr, c, out, &n, &h);
                encode(enc_ign, c, out, &n, &h);
            }
            printf("B %X %016llX\n", block, h);
        }
        iconv_close(dec);
        iconv_close(dec_ign);
        iconv_close(enc);
        iconv_close(enc_tr);
        iconv_close(enc_ign);
    }
    return 0;
}
