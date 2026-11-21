/* WIDE-READ-FAST: per-call instructions of the wide reads against the byte reads (wl_percall --spec).
 * fgetwc, getwc, fgetwc_unlocked, getwchar, fgetws and ungetwc on fully buffered (8192-byte) streams over files of
 * one character repeated: NUL (/dev/zero), ASCII 'a', U+00E9 (2 bytes), U+3042 (3 bytes: 8192 is not a multiple,
 * so a character cut by the buffer end and its refill are part of the per-call cost), U+1F600 (4 bytes); in
 * C.UTF-8 and in the single-byte "C" locale. The byte reads (fgetc, getc, getc_unlocked on /dev/zero) are the
 * "byte reads must not get slower" rows. The harness makes 20000 calls; every file holds 65536 characters. */
#define _GNU_SOURCE
#include "percall.h"
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <wchar.h>

static char sc_rbuf[8192];

static FILE *sc_buffered(FILE *f)
{
    if (!f) abort();
    setvbuf(f, sc_rbuf, _IOFBF, sizeof sc_rbuf);
    return f;
}

static FILE *sc_zero(void) { return sc_buffered(fopen("/dev/zero", "r")); }

/* A file of 65536 copies of the `len`-byte sequence `seq` (a line feed every 32 characters when `lines`). */
static const char *sc_file(const char *seq, int len, int lines)
{
    static char paths[8][32];
    static int used;
    char *p = paths[used++ & 7];
    snprintf(p, 32, "/tmp/pc_wideXXXXXX");
    int fd = mkstemp(p);
    if (fd < 0) abort();
    static char blk[65536];
    size_t at = 0;
    for (int c = 0; c < 65536; c++) {
        if (lines && c % 32 == 31) {
            blk[at++] = '\n';
        } else {
            for (int k = 0; k < len; k++) blk[at++] = seq[k];
        }
        if (at > sizeof blk - 8) {
            if (write(fd, blk, at) != (ssize_t)at) abort();
            at = 0;
        }
    }
    if (write(fd, blk, at) != (ssize_t)at) abort();
    close(fd);
    return p;
}

static FILE *sc_open(const char *seq, int len) { return sc_buffered(fopen(sc_file(seq, len, 0), "r")); }
static FILE *sc_ascii(void) { return sc_open("a", 1); }
static FILE *sc_two(void) { return sc_open("\xc3\xa9", 2); }
static FILE *sc_three(void) { return sc_open("\xe3\x81\x82", 3); }
static FILE *sc_four(void) { return sc_open("\xf0\x9f\x98\x80", 4); }

#define READ_CASE(nm, call) \
    CASE_LOCALE(nm, 1000000, 1, "C.UTF-8") { \
        FILE *f = sc_zero(); \
        for (long i = 0; i < n; i++) pc_sink += call(f); \
        PC_CHECKING pc_check("%d", call(f)); \
        fclose(f); \
    }
READ_CASE(fgetc, fgetc)
READ_CASE(getc, getc)
READ_CASE(getc_unlocked, getc_unlocked)

#define WIDE_CASE(nm, loc, open, call) \
    CASE_LOCALE(nm, 1000000, 1, loc) { \
        FILE *f = open(); \
        for (long i = 0; i < n; i++) pc_sink += call(f); \
        PC_CHECKING pc_check("%u", (unsigned)call(f)); \
        fclose(f); \
    }
#define WIDE_WIDTHS(fn) \
    WIDE_CASE(fn##_nul, "C.UTF-8", sc_zero, fn) \
    WIDE_CASE(fn##_ascii, "C.UTF-8", sc_ascii, fn) \
    WIDE_CASE(fn##_2byte, "C.UTF-8", sc_two, fn) \
    WIDE_CASE(fn##_3byte, "C.UTF-8", sc_three, fn) \
    WIDE_CASE(fn##_4byte, "C.UTF-8", sc_four, fn) \
    WIDE_CASE(fn##_c_ascii, "C", sc_ascii, fn)
WIDE_WIDTHS(fgetwc)
WIDE_WIDTHS(getwc)
WIDE_WIDTHS(fgetwc_unlocked)

/* getwchar: stdin reopened on the file. */
#define STDIN_CASE(nm, loc, seq, len) \
    CASE_LOCALE(nm, 1000000, 1, loc) { \
        if (!freopen(sc_file(seq, len, 0), "r", stdin)) abort(); \
        sc_buffered(stdin); \
        for (long i = 0; i < n; i++) pc_sink += getwchar(); \
        PC_CHECKING pc_check("%u", (unsigned)getwchar()); \
    }
STDIN_CASE(getwchar_ascii, "C.UTF-8", "a", 1)
STDIN_CASE(getwchar_2byte, "C.UTF-8", "\xc3\xa9", 2)
STDIN_CASE(getwchar_3byte, "C.UTF-8", "\xe3\x81\x82", 3)
STDIN_CASE(getwchar_c_ascii, "C", "a", 1)

/* fgetws: one 32-character line (31 characters and the line feed) per call. */
#define LINE_CASE(nm, loc, seq, len) \
    CASE_LOCALE(nm, 100000, 1, loc) { \
        FILE *f = sc_buffered(fopen(sc_file(seq, len, 1), "r")); \
        wchar_t line[64]; \
        for (long i = 0; i < n; i++) pc_sink += (fgetws(line, 64, f) != NULL) + line[0]; \
        PC_CHECKING pc_check("%d %u", fgetws(line, 64, f) != NULL, (unsigned)line[0]); \
        fclose(f); \
    }
LINE_CASE(fgetws_ascii, "C.UTF-8", "a", 1)
LINE_CASE(fgetws_2byte, "C.UTF-8", "\xc3\xa9", 2)
LINE_CASE(fgetws_3byte, "C.UTF-8", "\xe3\x81\x82", 3)
LINE_CASE(fgetws_c_ascii, "C", "a", 1)

/* ungetwc: one character given back and read again per iteration (the pair's cost). */
#define UNGET_CASE(nm, loc, open) \
    CASE_LOCALE(nm, 100000, 1, loc) { \
        FILE *f = open(); \
        wint_t c = fgetwc(f); \
        for (long i = 0; i < n; i++) { ungetwc(c, f); pc_sink += fgetwc(f); } \
        PC_CHECKING { wint_t u = ungetwc(c, f); pc_check("%u %u", (unsigned)u, (unsigned)fgetwc(f)); } \
        fclose(f); \
    }
UNGET_CASE(ungetwc_fgetwc_2byte, "C.UTF-8", sc_two)
UNGET_CASE(ungetwc_fgetwc_c_ascii, "C", sc_ascii)
