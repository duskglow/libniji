/* gencat_products.rs: open the catalog named by argv[1] (a path with '/', so no NLSPATH search) and, for every
 * "set msg" line on stdin, print "set msg " and the text catgets returns, bytes below 0x20 and above 0x7e as \ooo,
 * or "(default)" when catgets returns the default string. Built against the reference and statically against
 * libniji's products. */
#include <nl_types.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    nl_catd cd = catopen(argv[1], 0);
    if (cd == (nl_catd)-1) {
        perror("catopen");
        return 1;
    }
    static const char dflt[] = "\x01default";
    int set, msg;
    while (scanf("%d %d", &set, &msg) == 2) {
        const char *s = catgets(cd, set, msg, dflt);
        printf("%d %d ", set, msg);
        if (s == dflt) {
            fputs("(default)\n", stdout);
            continue;
        }
        for (const unsigned char *p = (const unsigned char *)s; *p; p++)
            if (*p < 0x20 || *p > 0x7e || *p == '\\')
                printf("\\%03o", *p);
            else
                putchar(*p);
        putchar('\n');
    }
    catclose(cd);
    return 0;
}
