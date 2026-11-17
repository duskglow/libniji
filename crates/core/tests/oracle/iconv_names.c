/* For each charset name on the command line: whether iconv_open accepts it in both directions
 * against UTF-32LE, then every single byte decoded (its code points, or the error) and each
 * decoded code point encoded back (its bytes, or the error). One line per name, so a name the
 * library does not know, or knows as another conversion, shows as a differing line.
 * Used by crates/core/tests/iconv_charmaps_products.rs. */
#include <errno.h>
#include <iconv.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    for (int a = 1; a < argc; a++) {
        const char *cs = argv[a];
        iconv_t dec = iconv_open("UTF-32LE", cs);
        iconv_t enc = iconv_open(cs, "UTF-32LE");
        printf("%s open %d %d", cs, dec != (iconv_t)-1, enc != (iconv_t)-1);
        if (dec != (iconv_t)-1 && enc != (iconv_t)-1) {
            for (int b = 0; b < 256; b++) {
                unsigned char in[1] = {(unsigned char)b};
                unsigned int out[4];
                char *ip = (char *)in, *op = (char *)out;
                size_t il = 1, ol = sizeof out;
                iconv(dec, NULL, NULL, NULL, NULL);
                size_t r = iconv(dec, &ip, &il, &op, &ol);
                if (r == (size_t)-1) {
                    printf(" %02X:e%d", b, errno);
                    continue;
                }
                size_t n = (size_t)(op - (char *)out) / 4;
                for (size_t i = 0; i < n; i++) {
                    unsigned char back[8];
                    char *bp = (char *)&out[i], *bo = (char *)back;
                    size_t bl = 4, bol = sizeof back;
                    iconv(enc, NULL, NULL, NULL, NULL);
                    size_t r2 = iconv(enc, &bp, &bl, &bo, &bol);
                    printf(" %02X:%X>", b, out[i]);
                    if (r2 == (size_t)-1)
                        printf("e%d", errno);
                    for (unsigned char *p = back; (char *)p < bo; p++) printf("%02X", *p);
                }
            }
        }
        printf("\n");
        if (dec != (iconv_t)-1) iconv_close(dec);
        if (enc != (iconv_t)-1) iconv_close(enc);
    }
    return 0;
}
