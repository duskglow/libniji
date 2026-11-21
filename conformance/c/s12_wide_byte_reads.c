/* Spec §12.1, §12.5 and §12.10, D-12.27, D-12.28, D-12.29: byte operations on a wide-oriented stream work on
 * the stream's bytes in order. The wide layer stops before an invalid sequence (WEOF, EILSEQ, the
 * error indicator) and before a character cut short at end of file (WEOF, the end-of-file
 * indicator); a byte read then returns its first byte and wide reads go on after it, as a program
 * that passes undecodable bytes through does (getwchar, then getchar on EILSEQ). The stream stays
 * wide-oriented. The streams are left open: the program ends with _exit. */
#define _DEFAULT_SOURCE
#include <errno.h>
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
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

/* One line per kind of invalid sequence (D-12.27, D-12.28): a lone continuation byte, an overlong
 * form, 0xff, a 3-byte sequence cut short by an ASCII byte, a 2-byte lead byte followed by an ASCII
 * byte, and a sequence cut short at end of file. */
static const char invalid[] = "p\x80q\n\xc0\xafr\n\xffs\n\xe2\x82t\n\xc3(u\nend\xe2\x82";

static FILE *open_mode(const char *s, size_t n, const char *mode)
{
    char path[] = "/tmp/s12wbrXXXXXX";
    int fd = mkstemp(path);
    if (fd < 0)
        return NULL;
    int ok = write(fd, s, n) == (ssize_t)n;
    close(fd);
    FILE *f = ok ? fopen(path, mode) : NULL;
    unlink(path);
    return f;
}

/* util-linux col's loop: wide reads, a byte read for each invalid sequence, then the bytes left at
 * end of file. Every input byte comes back once, in order: 1 when the output equals the input. */
static int pass_through(FILE *f, const char *s, size_t n)
{
    char out[64];
    size_t k = 0;
    mbstate_t st;
    memset(&st, 0, sizeof st);
    for (;;) {
        errno = 0;
        wint_t c = fgetwc(f);
        if (c != WEOF) {
            size_t m = wcrtomb(out + k, (wchar_t)c, &st);
            if (m == (size_t)-1 || (k += m) > sizeof out - 8)
                return 0;
            continue;
        }
        if (errno != EILSEQ)
            break;
        int b = getc(f);
        if (b == EOF || k >= sizeof out)
            return 0;
        out[k++] = (char)b;
    }
    if (!feof(f))
        return 0;
    for (int b; (b = getc(f)) != EOF && k < sizeof out;)
        out[k++] = (char)b;
    return k == n && memcmp(out, s, n) == 0 && fwide(f, 0) > 0;
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

    /* Every kind of invalid sequence keeps its bytes for the byte reads, in the locale's encoding
     * and in a ccs= one. */
    CHECK(26, (f = open_mode(invalid, sizeof invalid - 1, "r")) != NULL && pass_through(f, invalid, sizeof invalid - 1));
    CHECK(27, (f = open_mode(invalid, sizeof invalid - 1, "r,ccs=UTF-8")) != NULL && pass_through(f, invalid, sizeof invalid - 1));

    /* Cut short by an ASCII byte: the sequence's first byte is next (the position before it); a
     * wide read then meets the rest as invalid (the position before it), then a byte given back is
     * read again, then the ASCII byte. */
    static const char cut[] = "\xe2\x82t\n";
    CHECK(28, (f = open_mode(cut, sizeof cut - 1, "r")) != NULL);
    errno = 0;
    CHECK(29, fgetwc(f) == WEOF && errno == EILSEQ && ferror(f) && ftell(f) == 0);
    errno = 0;
    CHECK(30, getc(f) == 0xe2 && fgetwc(f) == WEOF && errno == EILSEQ && ftell(f) == 1);
    CHECK(31, getc(f) == 0x82 && ungetc(0x82, f) == 0x82 && getc(f) == 0x82 && fgetwc(f) == L't' && fgetwc(f) == L'\n');
    static const char lead[] = "\xc3(u";
    CHECK(33, (f = open_mode(lead, sizeof lead - 1, "r,ccs=UTF-8")) != NULL);
    CHECK(34, fgetwc(f) == WEOF && getc(f) == 0xc3 && ungetc(0xc3, f) == 0xc3 && getc(f) == 0xc3 && fgetwc(f) == L'(');

    /* Cut short at end of file: WEOF with the end-of-file indicator, the position before the
     * sequence, then its bytes. */
    static const char end[] = "e\xe2\x82";
    CHECK(35, (f = open_mode(end, sizeof end - 1, "r")) != NULL && fgetwc(f) == L'e');
    CHECK(36, fgetwc(f) == WEOF && feof(f) && !ferror(f) && ftell(f) == 1);
    CHECK(37, getc(f) == 0xe2 && getc(f) == 0x82 && getc(f) == EOF && feof(f));

    /* Cut short at end of file with nothing before it in the input read (D-12.29): EILSEQ with the
     * error indicator and not the end-of-file one, the bytes consumed (the position after them),
     * then a plain end of input. In the locale's encoding and a ccs= one. */
    static const char alone[] = "\xe3\x81";
    for (int k = 0; k < 2; k++) {
        CHECK(38, (f = open_mode(alone, sizeof alone - 1, k ? "r,ccs=UTF-8" : "r")) != NULL);
        errno = 0;
        CHECK(39, fgetwc(f) == WEOF && errno == EILSEQ && ferror(f) && !feof(f) && ftell(f) == 2);
        errno = 0;
        CHECK(40, fgetwc(f) == WEOF && errno != EILSEQ && feof(f) && ftell(f) == 2);
    }
    return 0;
}

int main(void)
{
    if (!setlocale(LC_ALL, "C.UTF-8"))
        _exit(1);
    int rc = run();
    _exit(rc);
}
