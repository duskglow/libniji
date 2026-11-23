/* LG6-CTYPE-OBJECTS (plan row LG-6; spec §5.1 ⚑ COMPAT, clause s0_6_c0_6_0; spec/behaviour/ctype-objects.md):
 * an old binary's view of the legacy data objects. The program binds __ctype_b, __ctype_tolower,
 * __ctype_toupper, the __ctype32_* names and _obstack at GLIBC_2.2.5 (as a binary linked before the
 * __ctype_*_loc accessors did: COPY relocations of the 8-byte pointers), then, in the "C" locale, checks every
 * byte against the library's own isalpha/isupper/isdigit/isspace/ispunct/tolower/toupper and the POSIX "C"
 * rules, the entries -128..-1 (no class; c + 256, EOF to itself), the 32-bit class table as the 16-bit one's
 * masks byte-reversed, and that _obstack is NULL. Run on the reference and on libniji alike.
 * CTYPE-OBJECTS-SETLOCALE: the six ctype objects follow the global LC_CTYPE (setlocale LC_ALL, LC_CTYPE, ""),
 * never another category or a thread's uselocale: the 16-bit ones equal *__ctype_*_loc() of the global locale,
 * the 32-bit ones a locale file's CLASS32/TOLOWER32/TOUPPER32 items (code points U+0000..U+00FF), and back in
 * "C" the "C" tables again. The 8-bit locale is de_DE.ISO-8859-1 compiled with the host's localedef into a
 * private LOCPATH directory, removed at exit.
 * Each failed check prints FAIL and the line; the exit status is the number of failures (capped). */
/* LINK-HELPERS: dlh_add */
#define _GNU_SOURCE
#include <ctype.h>
#include <stdint.h>
#include <langinfo.h>
#include <locale.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wctype.h>

#define BIND(local, sym, ver) __asm__(".symver " #local "," #sym "@" #ver)
extern const unsigned short *old_b; BIND(old_b, __ctype_b, GLIBC_2.2.5);
extern const int *old_tolower; BIND(old_tolower, __ctype_tolower, GLIBC_2.2.5);
extern const int *old_toupper; BIND(old_toupper, __ctype_toupper, GLIBC_2.2.5);
extern const uint32_t *old_b32; BIND(old_b32, __ctype32_b, GLIBC_2.2.5);
extern const int *old_tolower32; BIND(old_tolower32, __ctype32_tolower, GLIBC_2.2.5);
extern const int *old_toupper32; BIND(old_toupper32, __ctype32_toupper, GLIBC_2.2.5);
extern void *old_obstack; BIND(old_obstack, _obstack, GLIBC_2.2.5);

static int fails;
#define CHECK(cond, c) do { if (!(cond)) { if (fails < 20) printf("FAIL line %d c=%d\n", __LINE__, (c)); fails++; } } while (0)

/* What <ctype.h>'s macros of an old binary computed: a mask test of the table entry. */
static int has(int c, unsigned short mask) { return (old_b[c] & mask) != 0; }

#define DE "de_DE.ISO-8859-1"
static char dir[256];
static void cleanup(void)
{
    char cmd[300];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", dir);
    if (system(cmd) == -1) printf("FAIL: rm\n");
}

/* The six objects now: the global locale's tables (`global` = 1) in the calling thread, whose own
   locale is the global one. `file` = 1: a locale file's 32-bit items; 0: the "C" tables. */
static void follows(int line, int file)
{
    int ok = old_b == *__ctype_b_loc() && old_tolower == *__ctype_tolower_loc() && old_toupper == *__ctype_toupper_loc();
    if (file) {
        ok = ok && (const void *)old_b32 == (const void *)nl_langinfo(5) && (const void *)old_tolower32 == (const void *)nl_langinfo(16)
            && (const void *)old_toupper32 == (const void *)nl_langinfo(15);
        for (int c = 0; c < 256; c++) {
            ok = ok && (wint_t)old_toupper32[c] == towupper(c) && (wint_t)old_tolower32[c] == towlower(c);
            ok = ok && ((old_b32[c] & __builtin_bswap32(1u << 2)) != 0) == (iswalpha(c) != 0);
        }
    } else {
        ok = ok && old_tolower32 == old_tolower && old_toupper32 == old_toupper && old_b32[0xe9] == 0 && old_toupper[0xe9] == 0xe9;
    }
    if (!ok) { printf("FAIL line %d (file %d)\n", line, file); fails++; }
}

static const unsigned short *seen_b;
static locale_t obj;
static void *in_thread(void *a)
{
    (void)a;
    uselocale(obj);
    if (old_b != seen_b || (*__ctype_b_loc())[0xe9] == 0) { printf("FAIL: uselocale moved the objects\n"); fails++; }
    uselocale(LC_GLOBAL_LOCALE);
    return NULL;
}

static void setlocale_rows(void)
{
    const char *tmp = getenv("TMPDIR");
    snprintf(dir, sizeof dir, "%s/s05-legacy-objects-XXXXXX", tmp && *tmp ? tmp : "/tmp");
    if (!mkdtemp(dir)) { printf("FAIL: mkdtemp\n"); exit(1); }
    atexit(cleanup);
    char cmd[400];
    snprintf(cmd, sizeof cmd, "localedef -i de_DE -f ISO-8859-1 '%s/" DE "' >/dev/null 2>&1; cd '%s' && ln -s " DE " de_DE.iso88591", dir, dir);
    if (system(cmd) == -1 || setenv("LOCPATH", dir, 1) != 0 || !setlocale(LC_ALL, DE)) {
        printf("FAIL: compiling and loading " DE " (localedef from the locales package)\n");
        exit(1);
    }
    follows(__LINE__, 1);
    CHECK(old_b[0xe9] != 0 && old_toupper[0xe9] == 0xc9 && old_toupper32[0xff] == 0x178, 0);
    setlocale(LC_ALL, "C");
    follows(__LINE__, 0);
    if (setlocale(LC_CTYPE, DE)) follows(__LINE__, 1); else CHECK(0, 0);
    setlocale(LC_ALL, "C");
    if (setlocale(LC_NUMERIC, DE)) follows(__LINE__, 0); else CHECK(0, 0);
    setlocale(LC_ALL, "C");
    seen_b = old_b;
    obj = newlocale(LC_ALL_MASK, DE, 0);
    pthread_t t;
    if (!obj || pthread_create(&t, NULL, in_thread, NULL) != 0 || pthread_join(t, NULL) != 0) CHECK(0, 0);
    if (obj) freelocale(obj);
    setenv("LC_ALL", DE, 1);
    if (setlocale(LC_ALL, "")) follows(__LINE__, 1); else CHECK(0, 0);
    setlocale(LC_ALL, "POSIX");
    follows(__LINE__, 0);
}

int main(void)
{
    /* The same contents as the accessors' tables over the whole index range (the addresses
       may differ: libniji renders these tables separately, D-5.13). */
    for (int c = -128; c < 256; c++) {
        CHECK(old_b[c] == (*__ctype_b_loc())[c], c);
        CHECK(old_tolower[c] == (*__ctype_tolower_loc())[c], c);
        CHECK(old_toupper[c] == (*__ctype_toupper_loc())[c], c);
    }
    CHECK(old_tolower32 == old_tolower && old_toupper32 == old_toupper, 0);
    CHECK(old_obstack == NULL, 0);
    for (int c = 0; c < 256; c++) {
        CHECK(has(c, _ISalpha) == (isalpha(c) != 0), c);
        CHECK(has(c, _ISupper) == (isupper(c) != 0), c);
        CHECK(has(c, _ISlower) == (islower(c) != 0), c);
        CHECK(has(c, _ISdigit) == (isdigit(c) != 0), c);
        CHECK(has(c, _ISxdigit) == (isxdigit(c) != 0), c);
        CHECK(has(c, _ISspace) == (isspace(c) != 0), c);
        CHECK(has(c, _ISprint) == (isprint(c) != 0), c);
        CHECK(has(c, _ISgraph) == (isgraph(c) != 0), c);
        CHECK(has(c, _ISblank) == (isblank(c) != 0), c);
        CHECK(has(c, _IScntrl) == (iscntrl(c) != 0), c);
        CHECK(has(c, _ISpunct) == (ispunct(c) != 0), c);
        CHECK(has(c, _ISalnum) == (isalnum(c) != 0), c);
        CHECK(old_tolower[c] == tolower(c) && old_toupper[c] == toupper(c), c);
        /* POSIX "C" locale: letters are A-Z and a-z only, nothing above 127 has a class. */
        CHECK(has(c, _ISalpha) == ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')), c);
        CHECK(c < 128 || old_b[c] == 0, c);
        CHECK(old_toupper[c] == ((c >= 'a' && c <= 'z') ? c - 32 : c), c);
        CHECK(old_tolower[c] == ((c >= 'A' && c <= 'Z') ? c + 32 : c), c);
        CHECK(old_b32[c] == __builtin_bswap32(__builtin_bswap16(old_b[c])), c);
    }
    for (int c = -128; c < 0; c++) {
        CHECK(old_b[c] == 0, c);
        CHECK(old_tolower[c] == (c == EOF ? EOF : c + 256) && old_toupper[c] == old_tolower[c], c);
    }
    setlocale_rows();
    return fails > 100 ? 100 : fails;
}
