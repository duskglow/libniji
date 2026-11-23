/* iconv: convert text between character sets (spec §31.6, plan/utilities.md UTIL-7).
 *
 * A thin front end over the C library's iconv(3): every conversion is what any program gets from
 * libniji's iconv_open/iconv (the built-in codecs and the charmaps of /usr/share/i18n/charmaps, D-6.8);
 * the command adds the options, the files and the messages. Built by `cargo xtask so` into
 * target/niji/iconv, a dynamically linked program shipped in libc-bin as /usr/bin/iconv.
 *
 * Behaviour (black-box probes of the reference, recorded as transcripts under validation/transcripts/iconv/):
 * `iconv [OPTION...] [FILE...]`; no FILE or `-` reads standard input. Each file is read whole and converted
 * on its own (a character split across two files is incomplete in the first). The output is collected and
 * written when the command ends, so `-o` may name an input file and an empty output creates no file.
 * Exit 0, or 1 after a conversion error (the output up to the error is written), an unknown charset, an
 * input file that cannot be opened (the other files are still converted) or a write error; 64 for an option
 * error. `-c` omits what cannot be converted (exit 0); an incomplete sequence at the end of a file is
 * still an error. `-s` changes nothing the probes could see. `-l` lists the built-in names, the
 * installed charmaps that iconv_open accepts and the names their headers give (plain or gzip).
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <iconv.h>
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <unistd.h>

#include "iconv-names.h"
#include "iconv-aliases.h"

#define CHARMAP_DIR "/usr/share/i18n/charmaps"

/* The charmap header reader and its inflater, shared with locale -m. */
#include "charmap-header.h"

static const char *prog_full = "iconv";
static const char *prog_short = "iconv";

static const char *from_code = "";
static const char *to_code = "";
static const char *output_file = NULL;
static int omit_invalid = 0;
static int list_names = 0;
static int verbose = 0;

static const char HELP[] =
    "Usage: iconv [OPTION...] [FILE...]\n"
    "Convert encoding of given files from one encoding to another.\n"
    "\n"
    " Input/Output format specification:\n"
    "  -f, --from-code=NAME       encoding of original text\n"
    "  -t, --to-code=NAME         encoding for output\n"
    "\n"
    " Information:\n"
    "  -l, --list                 list all known coded character sets\n"
    "\n"
    " Output control:\n"
    "  -c                         omit invalid characters from output\n"
    "  -o, --output=FILE          output file\n"
    "  -s, --silent               suppress warnings\n"
    "      --verbose              print progress information\n"
    "\n"
    "  -?, --help                 Give this help list\n"
    "      --usage                Give a short usage message\n"
    "  -V, --version              Print program version\n"
    "\n"
    "Mandatory or optional arguments to long options are also mandatory or optional\n"
    "for any corresponding short options.\n";

static const char USAGE[] =
    "Usage: iconv [-lcs?V] [-f NAME] [-t NAME] [-o FILE] [--from-code=NAME]\n"
    "            [--to-code=NAME] [--list] [--output=FILE] [--silent] [--verbose]\n"
    "            [--help] [--usage] [--version] [FILE...]\n";

/* Identity string (spec §31.1): libniji's own, the ABI level last. */
static const char VERSION[] = "iconv (libniji) 2.39\n";

/* The converted bytes, written when the command ends. */
static char *out_buf;
static size_t out_len, out_cap;

static _Noreturn void fail_memory(void) {
    fprintf(stderr, "%s: %s\n", prog_full, strerror(ENOMEM));
    exit(1);
}

/* Room for at least `more` further bytes after out_len. */
static void reserve(size_t more) {
    if (out_cap - out_len >= more)
        return;
    size_t cap = out_cap ? out_cap : 4096;
    while (cap - out_len < more) {
        if (cap > ((size_t)-1) / 2)
            fail_memory();
        cap *= 2;
    }
    char *p = realloc(out_buf, cap);
    if (p == NULL)
        fail_memory();
    out_buf = p;
    out_cap = cap;
}

/* Write the collected output (stdout, or the -o file only when there is output) and exit with `status`. */
static _Noreturn void finish(int status) {
    FILE *f = stdout;
    if (output_file != NULL) {
        if (out_len == 0)
            exit(status);
        f = fopen(output_file, "w");
        if (f == NULL) {
            fprintf(stderr, "%s: cannot open output file: %s\n", prog_full, strerror(errno));
            exit(1);
        }
    }
    if ((out_len != 0 && fwrite(out_buf, 1, out_len, f) != out_len) || fclose(f) != 0) {
        fprintf(stderr, "%s: error while closing output file: %s\n", prog_full, strerror(errno));
        exit(1);
    }
    exit(status);
}

static void try_line(FILE *f) {
    fprintf(f, "Try `%s --help' or `%s --usage' for more information.\n", prog_short, prog_short);
}

static _Noreturn void option_error(void) {
    try_line(stderr);
    exit(64);
}

/* The long options, in the order an ambiguity message lists them. */
struct longopt {
    const char *name;
    int has_arg;
    int code;
};
static const struct longopt LONGS[] = {
    {"from-code", 1, 'f'}, {"to-code", 1, 't'}, {"list", 0, 'l'},  {"output", 1, 'o'},  {"silent", 0, 's'},
    {"verbose", 0, 'v'},   {"help", 0, '?'},    {"usage", 0, 'u'}, {"version", 0, 'V'},
};
#define NLONGS (sizeof LONGS / sizeof LONGS[0])

static void act(int code, const char *arg) {
    switch (code) {
    case 'f':
        from_code = arg;
        break;
    case 't':
        to_code = arg;
        break;
    case 'l':
        list_names = 1;
        break;
    case 'o':
        output_file = arg;
        break;
    case 'c':
        omit_invalid = 1;
        break;
    case 'v':
        verbose = 1;
        break;
    case '?':
        fputs(HELP, stdout);
        exit(0);
    case 'u':
        fputs(USAGE, stdout);
        exit(0);
    case 'V':
        fputs(VERSION, stdout);
        exit(0);
    default: /* 's' */
        break;
    }
}

/* One `--name[=value]` argument at argv[*i]; advances *i past any separate value. */
static void long_option(int argc, char **argv, int *i) {
    const char *text = argv[*i] + 2;
    const char *eq = strchr(text, '=');
    size_t len = eq ? (size_t)(eq - text) : strlen(text);
    const struct longopt *hit = NULL;
    int matches = 0;
    for (size_t k = 0; k < NLONGS; k++) {
        if (strncmp(LONGS[k].name, text, len) != 0)
            continue;
        if (strlen(LONGS[k].name) == len) {
            hit = &LONGS[k];
            matches = 1;
            break;
        }
        if (matches++ == 0)
            hit = &LONGS[k];
    }
    if (matches == 0) {
        fprintf(stderr, "%s: unrecognized option '%s'\n", prog_full, argv[*i]);
        option_error();
    }
    if (matches > 1) {
        fprintf(stderr, "%s: option '%.*s' is ambiguous; possibilities:", prog_full, (int)len + 2, argv[*i]);
        for (size_t k = 0; k < NLONGS; k++)
            if (strncmp(LONGS[k].name, text, len) == 0)
                fprintf(stderr, " '--%s'", LONGS[k].name);
        fputc('\n', stderr);
        option_error();
    }
    const char *value = NULL;
    if (hit->has_arg) {
        if (eq != NULL) {
            value = eq + 1;
        } else if (*i + 1 < argc) {
            value = argv[++*i];
        } else {
            fprintf(stderr, "%s: option '--%s' requires an argument\n", prog_full, hit->name);
            option_error();
        }
    } else if (eq != NULL) {
        fprintf(stderr, "%s: option '--%s' doesn't allow an argument\n", prog_full, hit->name);
        option_error();
    }
    act(hit->code, value);
}

/* A cluster of short options at argv[*i]; advances *i past any separate value. */
static void short_options(int argc, char **argv, int *i) {
    for (const char *p = argv[*i] + 1; *p != '\0'; p++) {
        switch (*p) {
        case 'l':
        case 'c':
        case 's':
        case '?':
        case 'V':
            act(*p, NULL);
            break;
        case 'f':
        case 't':
        case 'o':
            if (p[1] != '\0') {
                act(*p, p + 1);
            } else if (*i + 1 < argc) {
                act(*p, argv[++*i]);
            } else {
                fprintf(stderr, "%s: option requires an argument -- '%c'\n", prog_full, *p);
                option_error();
            }
            return;
        default:
            fprintf(stderr, "%s: invalid option -- '%c'\n", prog_full, *p);
            option_error();
        }
    }
}

static int by_name(const void *a, const void *b) {
    return strcasecmp(*(const char *const *)a, *(const char *const *)b);
}

/* The names `-l` collects (unsorted, may repeat). */
struct listing {
    const char **v;
    size_t n, cap;
};

static void keep(struct listing *l, const char *name) {
    if (l->n == l->cap) {
        l->cap = l->cap ? l->cap * 2 : 512;
        const char **p = realloc(l->v, l->cap * sizeof *p);
        if (p == NULL)
            fail_memory();
        l->v = p;
    }
    l->v[l->n++] = name;
}

/* How much of each charmap file iconv_open's header scan reads when no loaded charmap and no file carries a
 * name (crates/legacy/src/global_locale.rs `find_charmap`): the file's first 4096 bytes, and of a gzip file at
 * most 8192 bytes of its text. `-l` reads the same, so a name it lists opens in a fresh process too. */
#define SCAN_FILE 4096
#define SCAN_TEXT 8192

static int name_byte(unsigned char c) {
    return (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == ',' || c == '-' ||
           c == '.' || c == '/' || c == ':' || c == '_';
}

static int space_byte(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\f' || c == '\r';
}

/* Keep one header name, spelled as iconv_open matches names (spec/encoding/iconv-names.md steps 2 and 3: only
 * letters, digits and `, - . / : _` count, letters uppercased), when iconv_open accepts that spelling. A name
 * with more than one `/` or ending in `/` is left out: `-l` prints it with a trailing `/`, which would read as
 * suffixes. */
static void keep_header_name(struct listing *l, const unsigned char *s, size_t len) {
    char name[129];
    size_t k = 0, slashes = 0;
    for (size_t i = 0; i < len; i++) {
        if (!name_byte(s[i]))
            continue;
        if (k == sizeof name - 1)
            return;
        slashes += s[i] == '/';
        name[k++] = (char)(s[i] >= 'a' && s[i] <= 'z' ? s[i] - 'a' + 'A' : s[i]);
    }
    name[k] = '\0';
    if (k == 0 || slashes > 1 || name[k - 1] == '/')
        return;
    iconv_t cd = iconv_open("UTF-8", name);
    if (cd == (iconv_t)-1)
        return;
    iconv_close(cd);
    char *copy = strdup(name);
    if (copy == NULL)
        fail_memory();
    keep(l, copy);
}

/* The names a charmap's header gives, as iconv_open reads them: the `<code_set_name>` and each comment line
 * `alias NAME` (the comment character is `<comment_char>`'s, `#` until one is given), each line trimmed of
 * ASCII white space, up to the `CHARMAP` line. Kept in l, or with l NULL only looked for. Returns 1 when a
 * whole `CHARMAP` line (its newline included) ends the header within text. */
static int header_names(struct listing *l, const unsigned char *text, size_t len) {
    unsigned char comment = '#';
    for (size_t at = 0; at < len;) {
        const unsigned char *nl = memchr(text + at, '\n', len - at);
        size_t end = nl != NULL ? (size_t)(nl - text) : len, b = at, e = end;
        at = end + 1;
        while (b < e && space_byte(text[b]))
            b++;
        while (e > b && space_byte(text[e - 1]))
            e--;
        const unsigned char *s = text + b;
        size_t n = e - b;
        if (n == 7 && memcmp(s, "CHARMAP", 7) == 0)
            return nl != NULL;
        if (l == NULL)
            continue;
        if (n >= 15 && memcmp(s, "<code_set_name>", 15) == 0) {
            keep_header_name(l, s + 15, n - 15);
        } else if (n >= 14 && memcmp(s, "<comment_char>", 14) == 0) {
            size_t c = 14;
            while (c < n && space_byte(s[c]))
                c++;
            if (c < n)
                comment = s[c];
        } else if (n > 0 && s[0] == comment) {
            size_t c = 1;
            while (c < n && space_byte(s[c]))
                c++;
            if (n - c >= 5 && memcmp(s + c, "alias", 5) == 0)
                keep_header_name(l, s + c + 5, n - c - 5);
        }
    }
    return 0;
}

/* The header text of a charmap file as iconv_open's scan reads it, into text (HEADER_MAX bytes). A gzip file is
 * decoded to its first SCAN_HEAD bytes, and on to SCAN_TEXT only when its `CHARMAP` line is not among them: no
 * name comes after that line, and most headers end well before SCAN_HEAD. */
#define SCAN_HEAD 2048
static size_t scan_header(const char *file, int gz, unsigned char *text) {
    if (!gz)
        return charmap_header(file, gz, text, SCAN_FILE, SCAN_FILE);
    size_t n = charmap_header(file, gz, text, SCAN_FILE, SCAN_HEAD);
    if (n < SCAN_HEAD || header_names(NULL, text, n))
        return n;
    return charmap_header(file, gz, text, SCAN_FILE, SCAN_TEXT);
}

/* `-l`: the built-in names, every installed charmap iconv_open accepts with the names its header gives, and
 * the listed names that stand for one of those charmaps, sorted, each once, as `NAME//`
 * (a name that holds a `/` ends in one `/`, as the reference lists `ISO-10646/UCS4/`). */
static _Noreturn void list(void) {
    struct listing l = {0};
    for (size_t k = 0; k < sizeof BUILTIN_NAMES / sizeof BUILTIN_NAMES[0]; k++)
        keep(&l, BUILTIN_NAMES[k]);
    static unsigned char text[HEADER_MAX];
    DIR *dir = opendir(CHARMAP_DIR);
    for (struct dirent *e; dir != NULL && (e = readdir(dir)) != NULL;) {
        size_t len = strlen(e->d_name);
        int gz = len > 3 && strcmp(e->d_name + len - 3, ".gz") == 0;
        if (gz)
            len -= 3;
        if (len == 0 || e->d_name[0] == '.')
            continue;
        char *name = strndup(e->d_name, len);
        if (name == NULL)
            fail_memory();
        iconv_t cd = iconv_open("UTF-8", name);
        if (cd == (iconv_t)-1) {
            free(name);
            continue;
        }
        iconv_close(cd);
        keep(&l, name);
        /* The charmap is loaded now, so iconv_open finds its header names without a scan. */
        header_names(&l, text, scan_header(e->d_name, gz, text));
    }
    if (dir != NULL)
        closedir(dir);
    /* The listed names that stand for a charmap (iconv-aliases.h): each one whose charmap file
     * opened above, or whose charmap iconv_open accepts under another file's name (CP874 as an
     * alias in IBM874's header), and that iconv_open then accepts; such a charmap name is listed
     * too. A charmap that is no installed file is tried once (a failed open scans every charmap's
     * header), a row whose charmap does not open is never opened. */
    enum { ROWS = sizeof ALIAS_ROWS / sizeof ALIAS_ROWS[0] };
    static const char *tried[ROWS];
    static int opened[ROWS];
    size_t stems = l.n, ntried = 0;
    for (size_t k = 0; k < ROWS; k++) {
        const char *target = ALIAS_ROWS[k][1];
        int installed = 0;
        for (size_t s = sizeof BUILTIN_NAMES / sizeof BUILTIN_NAMES[0]; s < stems && !installed; s++)
            installed = strcasecmp(l.v[s], target) == 0;
        for (size_t t = 0; t < ntried && !installed; t++)
            if (strcmp(tried[t], target) == 0)
                installed = opened[t] ? 1 : -1;
        if (installed == 0) {
            iconv_t tcd = iconv_open("UTF-8", target);
            installed = tcd == (iconv_t)-1 ? -1 : 1;
            if (installed > 0)
                iconv_close(tcd);
            tried[ntried] = target;
            opened[ntried++] = installed > 0;
            if (installed > 0)
                keep(&l, target);
        }
        if (installed < 0)
            continue;
        iconv_t cd = iconv_open("UTF-8", ALIAS_ROWS[k][0]);
        if (cd == (iconv_t)-1)
            continue;
        iconv_close(cd);
        keep(&l, ALIAS_ROWS[k][0]);
    }
    qsort(l.v, l.n, sizeof *l.v, by_name);
    for (size_t k = 0; k < l.n; k++) {
        if (k > 0 && strcasecmp(l.v[k], l.v[k - 1]) == 0)
            continue;
        printf("%s%s\n", l.v[k], strchr(l.v[k], '/') != NULL ? "/" : "//");
    }
    if (fclose(stdout) != 0)
        exit(1);
    exit(0);
}

/* Open the descriptor; an unknown charset is reported against the name the user gave. */
static iconv_t open_conversion(void) {
    const char *to = to_code;
    char *with_ignore = NULL;
    if (omit_invalid) {
        /* `-c`: the library skips what it cannot convert (`//IGNORE`, appended as one more suffix). */
        const char *sep = strstr(to_code, "//") != NULL ? "," : "//";
        if (asprintf(&with_ignore, "%s%sIGNORE", to_code, sep) < 0)
            fail_memory();
        to = with_ignore;
    }
    iconv_t cd = iconv_open(to, from_code);
    if (cd != (iconv_t)-1) {
        free(with_ignore);
        return cd;
    }
    int err = errno;
    if (err == EINVAL) {
        iconv_t t;
        int from_ok = (t = iconv_open("UTF-8", from_code)) != (iconv_t)-1;
        if (from_ok)
            iconv_close(t);
        int to_ok = (t = iconv_open(to, "UTF-8")) != (iconv_t)-1;
        if (to_ok)
            iconv_close(t);
        if (!from_ok && !to_ok)
            fprintf(stderr, "%s: conversions from `%s' and to `%s' are not supported\n", prog_full, from_code, to_code);
        else if (!from_ok)
            fprintf(stderr, "%s: conversion from `%s' is not supported\n", prog_full, from_code);
        else if (!to_ok)
            fprintf(stderr, "%s: conversion to `%s' is not supported\n", prog_full, to_code);
        else
            fprintf(stderr, "%s: conversion from `%s' to `%s' is not supported\n", prog_full, from_code, to_code);
        try_line(stderr);
    } else {
        fprintf(stderr, "%s: failed to start conversion processing: %s\n", prog_full, strerror(err));
    }
    exit(1);
}

/* Convert one whole input; returns 0, or 1 after reporting an error (the caller stops). */
static int convert(iconv_t cd, char *in, size_t len) {
    char *ip = in;
    size_t left = len;
    for (;;) {
        reserve(left < ((size_t)-1 - 64) / 8 ? left * 8 + 64 : left);
        char *op = out_buf + out_len;
        size_t room = out_cap - out_len;
        char *before = ip;
        size_t r = iconv(cd, &ip, &left, &op, &room);
        out_len = (size_t)(op - out_buf);
        if (r != (size_t)-1)
            break;
        if (errno == E2BIG)
            continue;
        if (errno == EILSEQ && omit_invalid && (left == 0 || ip != before))
            continue;
        if (errno == EINVAL) {
            fprintf(stderr, "%s: incomplete character or shift sequence at end of buffer\n", prog_full);
            return 1;
        }
        if (errno == EILSEQ) {
            fprintf(stderr, "%s: illegal input sequence at position %lld\n", prog_full, (long long)(ip - in));
            return 1;
        }
        fprintf(stderr, "%s: %s\n", prog_full, strerror(errno));
        return 1;
    }
    /* The end of this input: return the target to its initial shift state. */
    for (;;) {
        reserve(64);
        char *op = out_buf + out_len;
        size_t room = out_cap - out_len;
        size_t r = iconv(cd, NULL, NULL, &op, &room);
        out_len = (size_t)(op - out_buf);
        if (r != (size_t)-1 || errno != E2BIG)
            return 0;
    }
}

/* Read all of `fd`; NULL (errno set) on a read error. */
static char *read_all(int fd, size_t *len) {
    size_t cap = 65536, n = 0;
    char *buf = malloc(cap);
    if (buf == NULL)
        fail_memory();
    for (;;) {
        if (n == cap) {
            if (cap > ((size_t)-1) / 2)
                fail_memory();
            char *p = realloc(buf, cap *= 2);
            if (p == NULL)
                fail_memory();
            buf = p;
        }
        ssize_t r = read(fd, buf + n, cap - n);
        if (r == 0)
            break;
        if (r < 0) {
            if (errno == EINTR)
                continue;
            int err = errno;
            free(buf);
            errno = err;
            return NULL;
        }
        n += (size_t)r;
    }
    *len = n;
    return buf;
}

/* Convert the file `name` (`-`: standard input; `named`: an operand, which --verbose names on stderr);
 * 0, 1 to go on with status 1, 2 to stop. */
static int process(iconv_t cd, const char *name, int named) {
    int fd = 0;
    if (strcmp(name, "-") != 0) {
        fd = open(name, O_RDONLY | O_CLOEXEC);
        if (fd < 0) {
            fprintf(stderr, "%s: cannot open input file `%s': %s\n", prog_full, name, strerror(errno));
            return 1;
        }
    }
    if (verbose && named)
        fprintf(stderr, "%s:\n", name);
    size_t len = 0;
    char *data = read_all(fd, &len);
    if (fd != 0)
        close(fd);
    if (data == NULL) {
        fprintf(stderr, "%s: error while reading the input: %s\n", prog_full, strerror(errno));
        return 2;
    }
    int bad = convert(cd, data, len);
    free(data);
    return bad ? 2 : 0;
}

int main(int argc, char **argv) {
    if (argc > 0 && argv[0] != NULL) {
        prog_full = argv[0];
        const char *slash = strrchr(argv[0], '/');
        prog_short = slash != NULL ? slash + 1 : argv[0];
    }
    setlocale(LC_ALL, "");
    /* Options anywhere before `--` (the operands keep their order). */
    char **operands = calloc((size_t)argc + 1, sizeof *operands);
    if (operands == NULL)
        fail_memory();
    int n = 0;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (strcmp(a, "--") == 0) {
            while (++i < argc)
                operands[n++] = argv[i];
            break;
        }
        if (a[0] == '-' && a[1] == '-')
            long_option(argc, argv, &i);
        else if (a[0] == '-' && a[1] != '\0')
            short_options(argc, argv, &i);
        else
            operands[n++] = argv[i];
    }
    if (list_names)
        list();
    iconv_t cd = open_conversion();
    int status = 0;
    int named = n != 0;
    if (n == 0)
        operands[n++] = "-";
    for (int k = 0; k < n; k++) {
        int r = process(cd, operands[k], named);
        if (r != 0)
            status = 1;
        if (r == 2)
            break;
    }
    finish(status);
}
