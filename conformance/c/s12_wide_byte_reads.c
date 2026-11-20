/* Spec §12.1, §12.5 and §12.10, D-12.27: byte operations on a wide-oriented stream work on the
 * stream's bytes in order. The wide layer stops before an invalid sequence (WEOF, EILSEQ, the error
 * indicator); a byte read then returns its first byte and wide reads go on after it, as a program
 * that passes undecodable bytes through does (getwchar, then getchar on EILSEQ). The stream stays
 * wide-oriented. The streams are left open: the program ends with _exit. */
#define _DEFAULT_SOURCE
#include <errno.h>
#include <locale.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>

#define CHECK(n, cond) do { if (!(cond)) return n; } while (0)

static const char line[] = "a\xfc" "cd\nxy\n";

static FILE *open_line(void)
{
    /* Written through the descriptor: a byte operation on the stream would orient it. */
    FILE *f = tmpfile();
    if (!f || write(fileno(f), line, sizeof line - 1) != (ssize_t)(sizeof line - 1) || lseek(fileno(f), 0, SEEK_SET) != 0)
        return NULL;
    return f;
}

/* Read 'a', then the invalid byte: WEOF with EILSEQ and the error indicator. */
static int to_invalid(FILE *f)
{
    if (fgetwc(f) != L'a')
        return 0;
    errno = 0;
    return fgetwc(f) == WEOF && errno == EILSEQ && ferror(f) && !feof(f) && fwide(f, 0) > 0;
}

static int run(void)
{
    FILE *f;
    char buf[16];

    /* The bytes after the last character delivered: no wide read reached 0xfc yet. */
    CHECK(2, (f = open_line()) != NULL);
    CHECK(3, fgetwc(f) == L'a' && fgetc(f) == 0xfc && !ferror(f) && !feof(f));
    CHECK(4, fgetwc(f) == L'c' && fgetwc(f) == L'd' && fwide(f, 0) > 0);

    /* The pass-through loop: EILSEQ, then the byte, then wide reads go on (the indicator stays). */
    CHECK(5, (f = open_line()) != NULL && to_invalid(f));
    CHECK(6, getc(f) == 0xfc && ferror(f) && !feof(f));
    CHECK(7, fgetwc(f) == L'c' && fgetwc(f) == L'd' && fgetwc(f) == L'\n' && fwide(f, 0) > 0);

    /* clearerr, then the byte: the indicator stays clear. */
    CHECK(8, (f = open_line()) != NULL && to_invalid(f));
    clearerr(f);
    CHECK(9, getc_unlocked(f) == 0xfc && fgetwc(f) == L'c' && !ferror(f));

    /* Two byte reads take the invalid byte and the next one. */
    CHECK(10, (f = open_line()) != NULL && to_invalid(f));
    CHECK(11, fgetc(f) == 0xfc && fgetc(f) == 'c' && fgetwc(f) == L'd');

    /* fread and fgets take bytes from the same place. */
    CHECK(12, (f = open_line()) != NULL && to_invalid(f));
    memset(buf, 0, sizeof buf);
    CHECK(13, fread(buf, 1, 3, f) == 3 && memcmp(buf, "\xfc" "cd", 3) == 0);
    CHECK(14, fgetwc(f) == L'\n' && fgetwc(f) == L'x');
    CHECK(15, (f = open_line()) != NULL && to_invalid(f));
    CHECK(16, fgets(buf, sizeof buf, f) == buf && strcmp(buf, "\xfc" "cd\n") == 0);
    CHECK(17, fgetwc(f) == L'x' && fwide(f, 0) > 0);

    /* A byte given back is read again. */
    CHECK(18, (f = open_line()) != NULL && to_invalid(f));
    CHECK(19, getc(f) == 0xfc && ungetc(0xfc, f) == 0xfc && getc(f) == 0xfc && fgetwc(f) == L'c');

    /* A wide character given back stays for the wide reads; the byte read takes the next byte. */
    CHECK(20, (f = open_line()) != NULL && to_invalid(f));
    CHECK(21, ungetwc(L'Q', f) == L'Q' && getc(f) == 0xfc && fgetwc(f) == L'Q' && fgetwc(f) == L'c');

    /* A byte written between flushed wide output lands in order. */
    CHECK(22, (f = tmpfile()) != NULL);
    CHECK(23, fputwc(L'w', f) == L'w' && fflush(f) == 0 && putc('p', f) == 'p' && fwide(f, 0) > 0);
    CHECK(24, fputwc(L'x', f) == L'x' && fflush(f) == 0);
    int fd = fileno(f);
    char out[4] = {0};
    CHECK(25, pread(fd, out, sizeof out, 0) == 3 && memcmp(out, "wpx", 3) == 0);
    return 0;
}

int main(void)
{
    if (!setlocale(LC_ALL, "C.UTF-8"))
        _exit(1);
    int rc = run();
    _exit(rc);
}
