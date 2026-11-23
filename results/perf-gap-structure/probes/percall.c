/* PERF-GAP-STRUCTURE-AUDIT per-call probe. Usage: percall <mode> <n>.
 * Per-call cost = (count(n) - count(0)) / n, so start-up and set-up cancel.
 * Library calls are direct (through the PLT when dynamic), with asm escapes so the
 * compiler cannot fold or remove them; sizes come from volatile variables so
 * memcpy/memcmp/strlen are real library calls on both libraries. */
#define _GNU_SOURCE
#include <ctype.h>
#include <locale.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <wctype.h>

#define ESC(p) __asm__ volatile("" : : "r"(p) : "memory")

static volatile size_t vsz64 = 64, vsz4k = 4096;
static void *noop(void *a) { return a; }

int main(int argc, char **argv)
{
    if (argc < 3)
        return 2;
    const char *m = argv[1];
    long n = atol(argv[2]);
    static char a[256], b[256], buf[256];
    memset(a, 'a', 64);
    memset(b, 'a', 64);
    ESC(a);
    ESC(b);
    int r = 0;
    if (!strcmp(m, "malloc_free")) {
        for (long i = 0; i < n; i++) { void *p = malloc(vsz64); ESC(p); free(p); }
    } else if (!strcmp(m, "malloc_free_4k")) {
        for (long i = 0; i < n; i++) { void *p = malloc(vsz4k); ESC(p); free(p); }
    } else if (!strcmp(m, "snprintf_d")) {
        for (long i = 0; i < n; i++) { r += snprintf(buf, sizeof buf, "%d", (int)i); ESC(buf); }
    } else if (!strcmp(m, "snprintf_mixed")) {
        for (long i = 0; i < n; i++) { r += snprintf(buf, sizeof buf, "%s=%d %x %c", a + 60, (int)i, (unsigned)i, 'q'); ESC(buf); }
    } else if (!strcmp(m, "fprintf_null")) {
        FILE *f = fopen("/dev/null", "w");
        if (!f) return 3;
        for (long i = 0; i < n; i++) r += fprintf(f, "%d\n", (int)i);
        fclose(f);
    } else if (!strcmp(m, "fopen_fclose")) {
        for (long i = 0; i < n; i++) { FILE *f = fopen("/dev/null", "r"); ESC(f); if (f) fclose(f); }
    } else if (!strcmp(m, "strlen64")) {
        for (long i = 0; i < n; i++) { r += (int)strlen(a); ESC(a); }
    } else if (!strcmp(m, "memcpy64")) {
        for (long i = 0; i < n; i++) { memcpy(buf, a, vsz64); ESC(buf); }
    } else if (!strcmp(m, "memcmp64")) {
        for (long i = 0; i < n; i++) { r += memcmp(a, b, vsz64); ESC(a); }
    } else if (!strcmp(m, "strcmp64")) {
        for (long i = 0; i < n; i++) { r += strcmp(a, b); ESC(a); }
    } else if (!strcmp(m, "strchr_miss64")) {
        for (long i = 0; i < n; i++) { r += strchr(a, 'z') != NULL; ESC(a); }
    } else if (!strcmp(m, "strtol")) {
        memcpy(buf, "123456", 7); ESC(buf);
        for (long i = 0; i < n; i++) { r += (int)strtol(buf, NULL, 10); ESC(buf); }
    } else if (!strcmp(m, "toupper_c")) {
        for (long i = 0; i < n; i++) { r += toupper((int)(i & 127)); }
    } else if (!strcmp(m, "iswalpha_loc") || !strcmp(m, "towupper_loc") || !strcmp(m, "strcoll_loc")
               || !strcmp(m, "mbrtowc_loc")) {
        const char *loc = getenv("PGSA_LOCALE");
        if (!setlocale(LC_ALL, loc ? loc : "C.UTF-8")) { fputs("LOCALE-FAIL\n", stderr); return 4; }
        if (m[0] == 'i') {
            for (long i = 0; i < n; i++) r += iswalpha((wint_t)(0x4e00 + (i & 255)));
        } else if (m[0] == 't') {
            for (long i = 0; i < n; i++) r += (int)towupper((wint_t)(0x3b1 + (i & 15)));
        } else if (m[0] == 's') {
            memcpy(a, "Zebra-apple", 12); memcpy(b, "zebra-Apple", 12); ESC(a); ESC(b);
            for (long i = 0; i < n; i++) { r += strcoll(a, b) > 0; ESC(a); }
        } else {
            static const char s[] = "\xe4\xb8\x80"; mbstate_t st; wchar_t w;
            for (long i = 0; i < n; i++) { memset(&st, 0, sizeof st); r += (int)mbrtowc(&w, s, 3, &st); ESC(&w); }
        }
    } else if (!strcmp(m, "pthread")) {
        for (long i = 0; i < n; i++) {
            pthread_t t;
            if (pthread_create(&t, NULL, noop, NULL)) return 5;
            pthread_join(t, NULL);
        }
    } else {
        return 6;
    }
    return r == 12345678 ? 1 : 0;
}
