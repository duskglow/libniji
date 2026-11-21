/* iconv_open name spellings (spec/encoding/iconv-names.md): for each argument, whether
 * iconv_open opens it as a source and as a target, the errno of each, and a fingerprint of the
 * conversion it denotes (every single byte decoded to UTF-8, a few characters encoded), so a
 * spelling that opens a different charset shows. A leading "\xNN" in an argument is that byte.
 * `-n COUNT NAME` instead opens and closes NAME (to and from UTF-8) COUNT times and prints
 * nothing: the loop the instruction counts of the name lookup run. */
#include <errno.h>
#include <iconv.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned long long fnv(unsigned long long h, const void *p, size_t n) {
    const unsigned char *b = p;
    for (size_t i = 0; i < n; i++) {
        h ^= b[i];
        h *= 1099511628211ULL;
    }
    return h;
}

static void unescape(const char *s, char *o) {
    for (; *s; s++) {
        if (s[0] == '\\' && s[1] == 'x' && s[2] && s[3]) {
            char hx[3] = {s[2], s[3], 0};
            *o++ = (char)strtol(hx, NULL, 16);
            s += 3;
        } else {
            *o++ = *s;
        }
    }
    *o = 0;
}

static const char *chars[] = {"A", "\xc3\xa9", "\xe2\x82\xac", "\xe3\x81\x82", "\xd0\x96", "\xc5\x93", "~", "\xc2\xa4"};

static void probe(const char *shown, const char *name) {
    unsigned long long h = 1469598103934665603ULL;
    errno = 0;
    iconv_t d = iconv_open("UTF-8", name);
    int e1 = d == (iconv_t)-1 ? errno : 0;
    if (d != (iconv_t)-1) {
        for (int b = 0; b < 256; b++) {
            char in = (char)b, out[16];
            char *ip = &in, *op = out;
            size_t il = 1, ol = sizeof out;
            iconv(d, NULL, NULL, NULL, NULL);
            int er = iconv(d, &ip, &il, &op, &ol) == (size_t)-1 ? errno : 0;
            h = fnv(h, &er, sizeof er);
            h = fnv(h, out, (size_t)(op - out));
        }
        iconv_close(d);
    }
    errno = 0;
    iconv_t t = iconv_open(name, "UTF-8");
    int e2 = t == (iconv_t)-1 ? errno : 0;
    if (t != (iconv_t)-1) {
        for (size_t c = 0; c < sizeof chars / sizeof *chars; c++) {
            char out[32];
            char *ip = (char *)chars[c], *op = out;
            size_t il = strlen(chars[c]), ol = sizeof out;
            iconv(t, NULL, NULL, NULL, NULL);
            int er = iconv(t, &ip, &il, &op, &ol) == (size_t)-1 ? errno : 0;
            h = fnv(h, &er, sizeof er);
            h = fnv(h, out, (size_t)(op - out));
        }
        iconv_close(t);
    }
    printf("%s\t%d\t%d\t%d\t%d\t%016llx\n", shown, e1 == 0, e2 == 0, e1, e2, (e1 == 0 || e2 == 0) ? h : 0ULL);
}

int main(int argc, char **argv) {
    if (argc == 4 && strcmp(argv[1], "-n") == 0) {
        long n = strtol(argv[2], NULL, 10);
        for (long i = 0; i < n; i++) {
            iconv_t d = iconv_open("UTF-8", argv[3]);
            iconv_t t = iconv_open(argv[3], "UTF-8");
            if (d == (iconv_t)-1 || t == (iconv_t)-1) return 1;
            iconv_close(d);
            iconv_close(t);
        }
        return 0;
    }
    char name[512];
    for (int i = 1; i < argc; i++) {
        if (strlen(argv[i]) >= sizeof name) return 2;
        unescape(argv[i], name);
        probe(argv[i], name);
    }
    return 0;
}
