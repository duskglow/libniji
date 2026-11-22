/* LG6-CTYPE-OBJECTS (plan row LG-6; spec §5.1 ⚑ COMPAT, clause s0_6_c0_6_0; spec/behaviour/ctype-objects.md):
 * an old binary's view of the legacy data objects. The program binds __ctype_b, __ctype_tolower,
 * __ctype_toupper, the __ctype32_* names and _obstack at GLIBC_2.2.5 (as a binary linked before the
 * __ctype_*_loc accessors did: COPY relocations of the 8-byte pointers), then, in the "C" locale, checks every
 * byte against the library's own isalpha/isupper/isdigit/isspace/ispunct/tolower/toupper and the POSIX "C"
 * rules, the entries -128..-1 (no class; c + 256, EOF to itself), the 32-bit class table as the 16-bit one's
 * masks byte-reversed, and that _obstack is NULL. Run on the reference and on libniji alike.
 * Each failed check prints FAIL and the line; the exit status is the number of failures (capped). */
/* LINK-HELPERS: dlh_add */
#define _GNU_SOURCE
#include <ctype.h>
#include <stdint.h>
#include <stdio.h>

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

int main(void)
{
    CHECK(old_b == *__ctype_b_loc(), 0);
    CHECK(old_tolower == *__ctype_tolower_loc(), 0);
    CHECK(old_toupper == *__ctype_toupper_loc(), 0);
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
    return fails > 100 ? 100 : fails;
}
