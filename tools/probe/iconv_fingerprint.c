/* Conversion fingerprint of charset names, black box through iconv(3) (ICONV-NAME-MATCH-2,
 * ICONV-REST). Reads one name per line on stdin; prints "name<TAB>hash" or "name<TAB>NOOPEN".
 * The hash covers decoding every 1-byte and every 2-byte input and encoding every BMP code point
 * plus a stride of supplementary ones, one conversion at a time with a reset between: two names
 * with the same hash convert the same way. Used by tools/probe/iconv_aliases.py.
 * Build: cc -O2 -o iconv_fingerprint tools/probe/iconv_fingerprint.c */
#include <errno.h>
#include <iconv.h>
#include <stdio.h>
#include <string.h>

static unsigned long long h;

static void mix(const void *p, size_t n) {
    const unsigned char *b = p;
    for (size_t i = 0; i < n; i++) {
        h ^= b[i];
        h *= 0x100000001b3ULL;
    }
}

static void one(iconv_t cd, const char *in, size_t n) {
    char out[64], *ip = (char *)in, *op = out;
    size_t il = n, ol = sizeof out;
    iconv(cd, NULL, NULL, NULL, NULL);
    size_t r = iconv(cd, &ip, &il, &op, &ol);
    int e = r == (size_t)-1 ? errno : 0;
    char *op2 = op;
    size_t ol2 = ol;
    iconv(cd, NULL, NULL, &op2, &ol2);
    mix(&e, sizeof e);
    mix(&il, sizeof il);
    mix(out, (size_t)(op2 - out));
    mix("|", 1);
}

static int utf8(unsigned c, char *o) {
    if (c < 0x80) {
        o[0] = (char)c;
        return 1;
    }
    if (c < 0x800) {
        o[0] = (char)(0xC0 | c >> 6);
        o[1] = (char)(0x80 | (c & 63));
        return 2;
    }
    if (c < 0x10000) {
        o[0] = (char)(0xE0 | c >> 12);
        o[1] = (char)(0x80 | (c >> 6 & 63));
        o[2] = (char)(0x80 | (c & 63));
        return 3;
    }
    o[0] = (char)(0xF0 | c >> 18);
    o[1] = (char)(0x80 | (c >> 12 & 63));
    o[2] = (char)(0x80 | (c >> 6 & 63));
    o[3] = (char)(0x80 | (c & 63));
    return 4;
}

int main(void) {
    char name[512];
    while (fgets(name, sizeof name, stdin)) {
        name[strcspn(name, "\n")] = 0;
        iconv_t dec = iconv_open("UTF-8", name), enc = iconv_open(name, "UTF-8");
        if (dec == (iconv_t)-1 || enc == (iconv_t)-1) {
            printf("%s\tNOOPEN\n", name);
            if (dec != (iconv_t)-1)
                iconv_close(dec);
            if (enc != (iconv_t)-1)
                iconv_close(enc);
            continue;
        }
        h = 0xcbf29ce484222325ULL;
        char b[4];
        for (unsigned i = 0; i < 256; i++) {
            b[0] = (char)i;
            one(dec, b, 1);
        }
        for (unsigned i = 0; i < 256; i++)
            for (unsigned j = 0; j < 256; j++) {
                b[0] = (char)i;
                b[1] = (char)j;
                one(dec, b, 2);
            }
        for (unsigned c = 0; c < 0x110000; c += (c < 0x10000 ? 1 : 0x101)) {
            if (c >= 0xD800 && c < 0xE000)
                continue;
            int n = utf8(c, b);
            one(enc, b, (size_t)n);
        }
        iconv_close(dec);
        iconv_close(enc);
        printf("%s\t%016llx\n", name, h);
        fflush(stdout);
    }
    return 0;
}
