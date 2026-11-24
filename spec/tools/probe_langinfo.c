/* The PROBE of spec/tools/langinfo_probe.py (D-7.16): black-box nl_langinfo_l items of one locale.
 *
 *   cc -O1 -o target/probe/probe_langinfo spec/tools/probe_langinfo.c
 *   target/probe/probe_langinfo LOCALE CAT:ITEM:LEN...
 *
 * One line per CAT:ITEM:LEN: `CAT<TAB>ITEM<TAB>W<TAB><decimal>` for a word item (the value
 * nl_langinfo_l returns is not a readable address: its low 32 bits), else
 * `CAT<TAB>ITEM<TAB>P<TAB><hex>` with LEN bytes from the returned address (fewer if the
 * memory after it ends). Readability is tested with write(2) to a pipe, never by a fault.
 * Nothing is printed when the locale does not open. */
#define _GNU_SOURCE
#include <langinfo.h>
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static int pfd[2];

static int readable(const char *p) {
    char c;
    if ((unsigned long)p < 65536UL) return 0;
    if (write(pfd[1], p, 1) != 1) return 0;
    return read(pfd[0], &c, 1) == 1;
}

int main(int argc, char **argv) {
    if (argc < 2 || pipe(pfd) != 0) return 2;
    locale_t l = newlocale(LC_ALL_MASK, argv[1], (locale_t)0);
    if (!l) return 1;
    for (int a = 2; a < argc; a++) {
        int cat, item, len;
        if (sscanf(argv[a], "%d:%d:%d", &cat, &item, &len) != 3) return 2;
        const char *p = nl_langinfo_l((nl_item)((cat << 16) | item), l);
        if (!readable(p)) {
            printf("%d\t%d\tW\t%u\n", cat, item, (unsigned)(unsigned long)p);
            continue;
        }
        printf("%d\t%d\tP\t", cat, item);
        for (int k = 0; k < len && readable(p + k); k++) printf("%02x", (unsigned char)p[k]);
        putchar('\n');
    }
    freelocale(l);
    return 0;
}
