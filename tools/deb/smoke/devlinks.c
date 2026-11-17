/* Smoke program for libc6-dev's development links and archives (SYS-DEV-LINKS; tools/deb/check-system.sh,
   tools/deb/root-system.sh): linked with -lanl -lBrokenLocale -lmcheck -lg, it prints MB_CUR_MAX in
   C.UTF-8, which libBrokenLocale's __ctype_get_mb_cur_max caps at 4 (the C library's own value is 6),
   so the line shows whether the link took libBrokenLocale. */
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    if (setlocale(LC_CTYPE, "C.UTF-8") == NULL) {
        printf("devlinks: no C.UTF-8\n");
        return 1;
    }
    printf("devlinks: MB_CUR_MAX %zu\n", (size_t) MB_CUR_MAX);
    return 0;
}
