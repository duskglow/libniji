/* No spec clause beyond RFC 959's names (HEADERS-STAGE2-GAPS-H: inetutils' ftpd/ftpd.c defines FTP_NAMES and
 * prints these tables in its replies): a program that defines FTP_NAMES before <arpa/ftp.h> gets typenames,
 * strunames, modenames and formnames, each indexed by its RFC 959 code (TYPE_A .. TYPE_L, STRU_F .. STRU_P,
 * MODE_S .. MODE_C, FORM_N .. FORM_C; entry 0 is "0") with exactly these strings and no terminating null pointer.
 * The header_check unit "FTP_NAMES arpa/ftp.h" covers the declarations' form; this program covers the contents,
 * which only a run can read. Defects: an entry misspelt, swapped or missing, a table not indexed by its code.
 * Prints each mismatch, exits 1 if any. */
#define FTP_NAMES
#include <arpa/ftp.h>
#include <stdio.h>
#include <string.h>

static int bad;

static void check(const char *table, char **got, size_t n, const char *const *want, size_t wn)
{
  size_t i;
  if (n != wn) {
    printf("%s: %zu entries, want %zu\n", table, n, wn);
    bad = 1;
    return;
  }
  for (i = 0; i < n; i++)
    if (got[i] == NULL || strcmp(got[i], want[i]) != 0) {
      printf("%s[%zu]: \"%s\", want \"%s\"\n", table, i, got[i] ? got[i] : "(null)", want[i]);
      bad = 1;
    }
}

#define CHECK(t, ...) do { static const char *const w[] = { __VA_ARGS__ }; \
  check(#t, t, sizeof t / sizeof t[0], w, sizeof w / sizeof w[0]); } while (0)

int main(void)
{
  CHECK(typenames, "0", "ASCII", "EBCDIC", "Image", "Local");
  CHECK(strunames, "0", "File", "Record", "Page");
  CHECK(modenames, "0", "Stream", "Block", "Compressed");
  CHECK(formnames, "0", "Nonprint", "Telnet", "Carriage-control");
  if (strcmp(typenames[TYPE_I], "Image") || strcmp(strunames[STRU_R], "Record") || strcmp(modenames[MODE_B], "Block")
      || strcmp(formnames[FORM_C], "Carriage-control")) {
    puts("a table is not indexed by its code");
    bad = 1;
  }
  return bad;
}
