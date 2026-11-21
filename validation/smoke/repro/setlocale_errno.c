/* validation/smoke/repro/setlocale_errno.c: AQOURS-SMOKE finding 3 (spec section 7, setlocale), from bash's
   test run-glob-test: bash reports a failed setlocale with strerror(errno) when errno is not 0 (it does not
   clear errno first). tests/glob2.sub sets LC_ALL to a locale that is not installed twice in one shell (lines
   44 and 65), and a bash started with that LC_ALL calls setlocale with it more than once, so bash's warning
   shows errno after a REPEATED failure.

   Asks twice for a locale that is not installed, clearing errno before each call, and prints setlocale's
   result and errno for each call.

   Build and run against both C libraries as validation/smoke/repro/regcomp_nesting.c says.

   Observed (2026-11-14, host C library glibc 2.39, three build hosts; names zh_HK.big5hkscs and xx_YY.UTF-8):
   the first call gives NULL and errno ENOENT on both; the second call gives NULL with errno ENOENT on libniji
   and errno unchanged (0) on the reference. This is the recorded difference "errno after setlocale returns NULL" in
   docs/differences-from-glibc.md (ruling #5480, no compatibility option). */
#include <errno.h>
#include <locale.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    const char *name = argc > 1 ? argv[1] : "zh_HK.big5hkscs";
    for (int call = 1; call <= 2; call++) {
        errno = 0;
        char *r = setlocale(LC_ALL, name);
        int err = errno;
        printf("call %d: setlocale(LC_ALL, \"%s\") %s errno %d (%s)\n", call, name, r ? "ok" : "NULL", err,
               err ? strerror(err) : "none");
    }
    return 0;
}
