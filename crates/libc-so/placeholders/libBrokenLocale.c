/* libBrokenLocale.so.1 and libBrokenLocale.a (SL-SONAMES): the program's MB_CUR_MAX
 * (`__ctype_get_mb_cur_max`) replaced by a smaller bound for programs that size buffers by it.
 *
 * The values are probe-settled (black-box, the reference library linked with -lBrokenLocale,
 * every locale code-set width the reference's charmaps have): a locale whose MB_CUR_MAX is
 * 1, 2, 3, 4 or 6 (UTF-8) gives 1, 1, 2, 2 and 4 (BIG5, EUC-KR, GBK, SHIFT_JIS, BIG5-HKSCS: 2 -> 1;
 * EUC-JP, EUC-JP-MS: 3 -> 2; GB18030, EUC-TW: 4 -> 2; UTF-8: 6 -> 4; C, KOI8-R, ISO-8859-1,
 * TIS-620: 1 -> 1). Liella's rule: half the width rounded up, and 4 for UTF-8's 6 (RFC 3629's
 * longest UTF-8 sequence). The width is libc's own (`nl_langinfo(_NL_CTYPE_MB_CUR_MAX)`, a word
 * item, spec D.1.5), so the library never calls the name it replaces. */
#include <langinfo.h>
#include <stddef.h>
#include <stdint.h>

size_t liella_ph___ctype_get_mb_cur_max(void);

size_t liella_ph___ctype_get_mb_cur_max(void)
{
    size_t n = (uint32_t) (uintptr_t) nl_langinfo(_NL_CTYPE_MB_CUR_MAX);
    return n >= 6 ? 4 : (n + 1) / 2;
}
