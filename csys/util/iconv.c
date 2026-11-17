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
 * still an error. `-s` changes nothing the probes could see. `-l` lists the built-in names and the
 * installed charmaps that iconv_open accepts.
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

#define CHARMAP_DIR "/usr/share/i18n/charmaps"

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

/* `-l`: the built-in names and every installed charmap iconv_open accepts, sorted, each once, as `NAME//`
 * (a name that holds a `/` ends in one `/`, as the reference lists `ISO-10646/UCS4/`). */
static _Noreturn void list(void) {
    size_t n = 0, cap = 512;
    const char **names = malloc(cap * sizeof *names);
    if (names == NULL)
        fail_memory();
    for (size_t k = 0; k < sizeof BUILTIN_NAMES / sizeof BUILTIN_NAMES[0]; k++)
        names[n++] = BUILTIN_NAMES[k];
    DIR *dir = opendir(CHARMAP_DIR);
    for (struct dirent *e; dir != NULL && (e = readdir(dir)) != NULL;) {
        size_t len = strlen(e->d_name);
        if (len > 3 && strcmp(e->d_name + len - 3, ".gz") == 0)
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
        if (n == cap) {
            cap *= 2;
            const char **p = realloc(names, cap * sizeof *names);
            if (p == NULL)
                fail_memory();
            names = p;
        }
        names[n++] = name;
    }
    if (dir != NULL)
        closedir(dir);
    qsort(names, n, sizeof *names, by_name);
    for (size_t k = 0; k < n; k++) {
        if (k > 0 && strcasecmp(names[k], names[k - 1]) == 0)
            continue;
        printf("%s%s\n", names[k], strchr(names[k], '/') != NULL ? "/" : "//");
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
