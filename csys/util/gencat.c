/* gencat (UTIL-14, spec §31.12, plan/utilities.md): compiles X/Open message-catalog sources (POSIX gencat) into
 * the Appendix D.2.2 catalog that catopen and catgets read, linked dynamically like getent.
 *
 * Every rule below that the spec and POSIX leave open was settled by black-box transcripts of the reference's
 * program (handoff/UTIL-14.md lists them): the plane size search, the string pool order (sets in the reverse of
 * the order they were first named, the default set 1 counting as named first; messages ascending), the merge with an existing catalog after the sources,
 * the -H header layout, octal escapes, the message texts of the diagnostics and the exit codes. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <wchar.h>

#define MAGIC 0x960408deu

struct msg {
    uint32_t num;
    char *name; /* symbolic identifier, or NULL */
    char *text;
    size_t len;
    const char *file;
    size_t line;
};

struct set {
    uint32_t num;
    char *name; /* symbolic identifier, or NULL */
    const char *file;
    size_t line;
    int explicit; /* named by a $set line (the default set 1 is not until then) */
    struct msg *m;
    size_t n, cap;
};

static struct set *sets; /* in the order they were first named */
static size_t nsets, capsets;
static int failed;

static void *xrealloc(void *p, size_t n) {
    void *q = realloc(p, n ? n : 1);
    if (!q) {
        fputs("gencat: memory exhausted\n", stderr);
        exit(1);
    }
    return q;
}

static char *xstrndup(const char *s, size_t n) {
    char *d = xrealloc(NULL, n + 1);
    memcpy(d, s, n);
    d[n] = 0;
    return d;
}

static void diag(const char *file, size_t line, const char *what) {
    fprintf(stderr, "%s:%zu: %s\n", file, line, what);
    failed = 1;
}

static struct set *set_by_num(uint32_t num) {
    for (size_t i = 0; i < nsets; i++)
        if (sets[i].num == num) return &sets[i];
    return NULL;
}

/* A new set goes last in the naming order, except the default set 1 (NL_SETD), which always counts as named first
 * whenever it is first used: the pool and the -H header list it last (probe UTIL-14 setorder). Callers hold no set
 * pointer across this call (the array moves). */
static struct set *new_set(uint32_t num, const char *file, size_t line) {
    if (nsets == capsets) sets = xrealloc(sets, (capsets = capsets ? 2 * capsets : 8) * sizeof *sets);
    size_t at = num == 1 ? 0 : nsets;
    memmove(&sets[at + 1], &sets[at], (nsets - at) * sizeof *sets);
    nsets++;
    struct set *s = &sets[at];
    *s = (struct set){.num = num, .file = file, .line = line};
    return s;
}

static struct msg *msg_by_num(struct set *s, uint32_t num) {
    for (size_t i = 0; i < s->n; i++)
        if (s->m[i].num == num) return &s->m[i];
    return NULL;
}

static void add_msg(struct set *s, struct msg m) {
    if (s->n == s->cap) s->m = xrealloc(s->m, (s->cap = s->cap ? 2 * s->cap : 8) * sizeof *s->m);
    s->m[s->n++] = m;
}

static int blank(char c) { return c == ' ' || c == '\t'; }
static int ident_start(char c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'; }

/* A decimal token's leading digits, modulo 2^32 (as the reference stores them); trailing bytes are ignored. */
static uint32_t number(const char *t, size_t n) {
    uint32_t v = 0;
    for (size_t i = 0; i < n && t[i] >= '0' && t[i] <= '9'; i++) v = v * 10 + (uint32_t)(t[i] - '0');
    return v;
}

/* The text of a message line after its separator: escapes decoded, the quote character honoured. Returns 0 when
 * the message is dropped (a character invalid in the locale's encoding). */
static int message_text(const char *p, const char *end, int quote, const char *file, size_t line, char **out, size_t *outlen) {
    char *buf = xrealloc(NULL, (size_t)(end - p) + 1);
    size_t n = 0;
    int quoted = quote && p < end && (unsigned char)*p == quote, closed = 0;
    if (quoted) p++;
    mbstate_t st = {0};
    while (p < end) {
        unsigned char c = (unsigned char)*p;
        if (quoted && c == quote) {
            closed = 1;
            break;
        }
        if (c == '\\') {
            p++;
            if (p == end) break; /* a backslash at the end of the input */
            char e = *p;
            if (e >= '0' && e <= '7') {
                unsigned v = 0;
                while (p < end && *p >= '0' && *p <= '7' && v * 8 + (unsigned)(*p - '0') <= 0377) v = v * 8 + (unsigned)(*p++ - '0');
                if (v > 0x7f)
                    diag(file, line, "invalid escape sequence");
                else
                    buf[n++] = (char)v;
                continue;
            }
            const char *from = "ntvbrf", *to = "\n\t\v\b\r\f";
            const char *k = strchr(from, e);
            buf[n++] = e && k ? to[k - from] : e; /* an unknown escape is the character itself */
            p++;
            continue;
        }
        size_t r = mbrtowc(NULL, p, (size_t)(end - p), &st);
        if (r == (size_t)-1 || r == (size_t)-2) {
            diag(file, line, "invalid character: message ignored");
            free(buf);
            return 0;
        }
        if (r == 0) r = 1;
        memcpy(buf + n, p, r);
        n += r;
        p += r;
    }
    if (quoted && !closed) diag(file, line, "unterminated message");
    buf[n] = 0;
    *out = buf;
    *outlen = n;
    return 1;
}

static struct set *current;
static int quote;

static void directive(const char *p, const char *end, const char *file, size_t line) {
    const char *w = p + 1, *we = w;
    while (we < end && !blank(*we)) we++;
    const char *a = we;
    while (a < end && blank(*a)) a++;
    const char *ae = a;
    while (ae < end && !blank(*ae)) ae++;
    size_t wl = (size_t)(we - w);
    if (wl == 3 && memcmp(w, "set", 3) == 0) {
        if (a < end && *a >= '0' && *a <= '9') {
            uint32_t num = number(a, (size_t)(ae - a));
            if (num == 0) return;
            struct set *s = set_by_num(num);
            if (s) {
                free(s->name), s->name = NULL; /* named again by number: the symbol is dropped */
                current = s;
            } else {
                current = new_set(num, file, line);
            }
            current->explicit = 1;
        } else if (a < end && ident_start(*a)) {
            for (size_t i = 0; i < nsets; i++)
                if (sets[i].name && strlen(sets[i].name) == (size_t)(ae - a) && memcmp(sets[i].name, a, (size_t)(ae - a)) == 0) {
                    diag(file, line, "duplicate set definition");
                    fprintf(stderr, "%s:%zu: this is the first definition\n", sets[i].file, sets[i].line);
                    current = &sets[i];
                    return;
                }
            /* The next number after the largest of the sets named by $set lines, compared as signed 32-bit
             * numbers from 0 (probe UTIL-14 edge); it can be the default set's 1, which then takes the name. */
            int32_t max = 0;
            for (size_t i = 0; i < nsets; i++)
                if (sets[i].explicit && (int32_t)sets[i].num > max) max = (int32_t)sets[i].num;
            uint32_t num = (uint32_t)max + 1;
            current = set_by_num(num);
            if (current)
                current->file = file, current->line = line;
            else
                current = new_set(num, file, line);
            current->explicit = 1;
            current->name = xstrndup(a, (size_t)(ae - a));
        } else {
            diag(file, line, "illegal set number");
        }
    } else if (wl == 6 && memcmp(w, "delset", 6) == 0) {
        /* Accepted and ignored: the reference deletes nothing (spec/behaviour/catalogs.md). */
    } else if (wl == 5 && memcmp(w, "quote", 5) == 0) {
        quote = 0;
        if (a < end) {
            if ((unsigned char)*a < 0x80)
                quote = (unsigned char)*a;
            else
                diag(file, line, "invalid quote character");
        }
    } else {
        fprintf(stderr, "%s:%zu: unknown directive `%.*s': line ignored\n", file, line, (int)wl, w);
        failed = 1;
    }
}

static void message(const char *p, const char *end, const char *file, size_t line) {
    const char *te = p;
    while (te < end && !blank(*te)) te++;
    int symbolic = ident_start(*p);
    uint32_t num = symbolic ? 0 : number(p, (size_t)(te - p));
    if (!symbolic && num == 0) return; /* message number 0: dropped without a diagnostic */
    const char *t = te < end ? te + 1 : te; /* one blank separates the number from the text */
    char *text;
    size_t len;
    if (!message_text(t, end, quote, file, line, &text, &len)) return;
    if (!current) {
        current = set_by_num(1);
        if (!current) current = new_set(1, file, line);
    }
    struct set *s = current;
    char *name = NULL;
    if (symbolic) {
        size_t nl = (size_t)(te - p);
        for (size_t i = 0; i < s->n; i++)
            if (s->m[i].name && strlen(s->m[i].name) == nl && memcmp(s->m[i].name, p, nl) == 0) {
                diag(file, line, "duplicated message identifier");
                fprintf(stderr, "%s:%zu: this is the first definition\n", s->m[i].file, s->m[i].line);
                free(text);
                return;
            }
        int32_t max = 0; /* the largest number in the set, signed, from 0 (probe UTIL-14 edge) */
        for (size_t i = 0; i < s->n; i++)
            if ((int32_t)s->m[i].num > max) max = (int32_t)s->m[i].num;
        num = (uint32_t)max + 1;
        name = xstrndup(p, nl);
    } else {
        struct msg *old = msg_by_num(s, num);
        if (old && old->name) {
            /* A number a symbolic message took: that message moves to the set's signed maximum + 1 (probe UTIL-14
             * edge2: `M a`, `N b`, `1 c` gives c 1, N 2, M 3). */
            int32_t max = 0;
            for (size_t i = 0; i < s->n; i++)
                if ((int32_t)s->m[i].num > max) max = (int32_t)s->m[i].num;
            old->num = (uint32_t)max + 1;
        } else if (old) {
            diag(file, line, "duplicated message number");
            fprintf(stderr, "%s:%zu: this is the first definition\n", old->file, old->line);
            free(text);
            return;
        }
    }
    add_msg(s, (struct msg){.num = num, .name = name, .text = text, .len = len, .file = file, .line = line});
}

static void source(FILE *f, const char *file) {
    char *buf = NULL, *logical = NULL;
    size_t cap = 0, lcap = 0, line = 0;
    ssize_t r;
    current = NULL;
    while ((r = getline(&buf, &cap, f)) > 0) {
        size_t first = ++line, n = 0;
        for (;;) {
            size_t len = (size_t)r;
            if (len && buf[len - 1] == '\n') len--;
            size_t bs = 0;
            while (bs < len && buf[len - 1 - bs] == '\\') bs++;
            int cont = bs % 2 == 1 && (size_t)r > len; /* an odd run of backslashes before the newline */
            if (cont) len--;
            if (n + len + 1 > lcap) logical = xrealloc(logical, lcap = 2 * (n + len + 1));
            memcpy(logical + n, buf, len);
            n += len;
            if (!cont || (r = getline(&buf, &cap, f)) <= 0) break;
            line++;
        }
        if (!logical) logical = xrealloc(NULL, lcap = 1);
        logical[n] = 0;
        const char *p = logical, *end = logical + n;
        if (n == 0) continue;
        if (*p == '$') {
            if (n > 1 && blank(p[1])) continue; /* a comment */
            directive(p, end, file, first);
        } else if ((*p >= '0' && *p <= '9') || ident_start(*p)) {
            message(p, end, file, first);
        } else {
            diag(file, first, "malformed line ignored");
        }
    }
    free(buf);
    free(logical);
}

/* Messages of an existing catalog at the output path that the sources did not define, read in table order. An
 * unreadable or malformed file is ignored (the reference writes a new catalog). Every offset is checked: the file
 * is untrusted. */
static void merge_existing(const char *path) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return;
    struct stat sb;
    unsigned char *b = NULL;
    size_t size = 0;
    if (fstat(fd, &sb) == 0 && S_ISREG(sb.st_mode) && sb.st_size >= 12 && (uint64_t)sb.st_size <= (1u << 30)) {
        size = (size_t)sb.st_size;
        b = xrealloc(NULL, size);
        size_t got = 0;
        while (got < size) {
            ssize_t r = read(fd, b + got, size - got);
            if (r <= 0) break;
            got += (size_t)r;
        }
        size = got;
    }
    close(fd);
    uint32_t w[3];
    if (!b || size < 12) goto out;
    memcpy(w, b, 12);
    uint64_t slots = (uint64_t)w[1] * w[2];
    if (w[0] != MAGIC || slots == 0 || slots > (size - 12) / 24) goto out;
    size_t pool = 12 + 24 * (size_t)slots;
    for (size_t k = 0; k < slots; k++) {
        uint32_t e[3];
        memcpy(e, b + 12 + 12 * k, 12);
        if (e[0] == 0 || e[2] >= size - pool) continue;
        const unsigned char *s = b + pool + e[2];
        const unsigned char *z = memchr(s, 0, size - pool - e[2]);
        size_t len = z ? (size_t)(z - s) : size - pool - e[2];
        struct set *st = set_by_num(e[0] - 1);
        if (!st) st = new_set(e[0] - 1, path, 0);
        if (msg_by_num(st, e[1])) continue;
        add_msg(st, (struct msg){.num = e[1], .text = xstrndup((const char *)s, len), .len = len, .file = path});
    }
out:
    free(b);
}

static int by_num(const void *a, const void *b) {
    int32_t x = (int32_t)((const struct msg *)a)->num, y = (int32_t)((const struct msg *)b)->num; /* signed order */
    return (x > y) - (x < y);
}

static uint32_t slot_of(uint32_t set, uint32_t msg, uint32_t p) { return (set + 1) * msg % p; }

/* The plane depth needed with plane size p, or 0 once it is known to exceed `limit`. */
static size_t depth(uint32_t p, size_t limit, uint32_t *count) {
    memset(count, 0, p * sizeof *count);
    size_t d = 1;
    for (size_t i = 0; i < nsets; i++)
        for (size_t j = 0; j < sets[i].n; j++) {
            uint32_t c = ++count[slot_of(sets[i].num, sets[i].m[j].num, p)];
            if (c > d && (d = c) > limit) return 0;
        }
    return d;
}

static int write_all(int fd, const void *p, size_t n) {
    size_t done = 0;
    while (done < n) {
        ssize_t r = write(fd, (const char *)p + done, n - done);
        if (r < 0 && errno == EINTR) continue;
        if (r <= 0) {
            const char *e = strerror(r < 0 ? errno : ENOSPC);
            fprintf(stderr, "write of %zu bytes failed after %zu: %s: %s\n", n, done, e, e);
            return -1;
        }
        done += (size_t)r;
    }
    return 0;
}

static int write_catalog(const char *path) {
    size_t total = 0, poolsize = 0;
    for (size_t i = 0; i < nsets; i++) {
        qsort(sets[i].m, sets[i].n, sizeof *sets[i].m, by_num);
        total += sets[i].n;
        for (size_t j = 0; j < sets[i].n; j++) poolsize += sets[i].m[j].len + 1;
    }
    /* The plane size P runs up from total/5 + 1 while P can still beat the best P*D; a tie goes to the larger P. */
    size_t bestp = 0, bestd = 0;
    uint32_t *count = NULL;
    for (size_t p = total / 5 + 1; bestp == 0 || p <= bestp * bestd; p++) {
        if (p > UINT32_MAX) break;
        count = xrealloc(count, p * sizeof *count);
        size_t d = depth((uint32_t)p, bestp ? bestp * bestd / p : SIZE_MAX, count);
        if (d && (bestp == 0 || p * d <= bestp * bestd)) bestp = p, bestd = d;
    }
    free(count);
    size_t slots = bestp * bestd;
    uint32_t *le = xrealloc(NULL, 12 * slots), *be = xrealloc(NULL, 12 * slots);
    memset(le, 0, 12 * slots);
    char *pool = xrealloc(NULL, poolsize);
    size_t off = 0;
    for (size_t i = nsets; i-- > 0;) /* the pool: the last-named set first */
        for (size_t j = 0; j < sets[i].n; j++) {
            struct msg *m = &sets[i].m[j];
            size_t at = slot_of(sets[i].num, m->num, (uint32_t)bestp);
            while (le[3 * at]) at += bestp; /* the first plane where the slot is free */
            le[3 * at] = sets[i].num + 1, le[3 * at + 1] = m->num, le[3 * at + 2] = (uint32_t)off;
            memcpy(pool + off, m->text, m->len + 1);
            off += m->len + 1;
        }
    for (size_t k = 0; k < 3 * slots; k++) be[k] = __builtin_bswap32(le[k]);
    uint32_t head[3] = {MAGIC, (uint32_t)bestp, (uint32_t)bestd};
    int fd = strcmp(path, "-") == 0 ? STDOUT_FILENO : open(path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0666);
    if (fd < 0) {
        fprintf(stderr, "cannot open output file `%s': %s\n", path, strerror(errno));
        return -1;
    }
    int rc = write_all(fd, head, 12) || write_all(fd, le, 12 * slots) || write_all(fd, be, 12 * slots) || write_all(fd, pool, poolsize) ? -1 : 0;
    if (fd != STDOUT_FILENO && close(fd) != 0 && rc == 0) {
        fprintf(stderr, "cannot open output file `%s': %s\n", path, strerror(errno));
        rc = -1;
    }
    free(le), free(be), free(pool);
    return rc;
}

static int write_header(const char *path) {
    FILE *f = strcmp(path, "-") == 0 ? stdout : fopen(path, "w");
    if (!f) {
        fprintf(stderr, "cannot open output file `%s': %s\n", path, strerror(errno));
        return -1;
    }
    for (size_t i = nsets; i-- > 0;) {
        struct set *s = &sets[i];
        char prefix[32];
        if (s->name) {
            if (i != nsets - 1) fputc('\n', f);
            fprintf(f, "#define %sSet %#x\t/* %s:%zu */\n", s->name, s->num, s->file, s->line);
        } else {
            snprintf(prefix, sizeof prefix, "AutomaticSet%u", s->num + 1);
        }
        for (size_t j = 0; j < s->n; j++)
            if (s->m[j].name)
                fprintf(f, "#define %s%s %#x\t/* %s:%zu */\n", s->name ? s->name : prefix, s->m[j].name, s->m[j].num, s->m[j].file, s->m[j].line);
    }
    int bad = f == stdout ? fflush(f) != 0 : fclose(f) != 0;
    if (bad) fprintf(stderr, "cannot open output file `%s': %s\n", path, strerror(errno));
    return bad ? -1 : 0;
}

static const char usage_text[] =
    "Usage: gencat [-?V] [-H NAME] [-o NAME] [--header=NAME] [--new] [--output=NAME]\n"
    "            [--help] [--usage] [--version] -o OUTPUT-FILE [INPUT-FILE]...\n"
    "  or:  gencat [OPTION...] [OUTPUT-FILE [INPUT-FILE]...]\n";

static const char help_text[] =
    "Usage: gencat [OPTION...] -o OUTPUT-FILE [INPUT-FILE]...\n"
    "  or:  gencat [OPTION...] [OUTPUT-FILE [INPUT-FILE]...]\n"
    "Generate message catalog.\n"
    "\n"
    "  -H, --header=NAME          Create C header file NAME containing symbol\n"
    "                             definitions\n"
    "      --new                  Do not use existing catalog, force new output file\n"
    "                            \n"
    "  -o, --output=NAME          Write output to file NAME\n"
    "  -?, --help                 Give this help list\n"
    "      --usage                Give a short usage message\n"
    "  -V, --version              Print program version\n"
    "\n"
    "Mandatory or optional arguments to long options are also mandatory or optional\n"
    "for any corresponding short options.\n"
    "\n"
    "If INPUT-FILE is -, input is read from standard input.  If OUTPUT-FILE\n"
    "is -, output is written to standard output.\n";

static int bad(const char *fmt, const char *what) {
    fputs("gencat: ", stderr);
    fprintf(stderr, fmt, what);
    fputs("\nTry `gencat --help' or `gencat --usage' for more information.\n", stderr);
    return 64;
}

int main(int argc, char **argv) {
    static const struct {
        const char *name;
        char opt;
        int arg;
    } longs[] = {{"header", 'H', 1}, {"new", 'n', 0}, {"output", 'o', 1}, {"help", '?', 0}, {"usage", 'u', 0}, {"version", 'V', 0}};
    enum { NLONG = sizeof longs / sizeof longs[0] };
    const char *output = NULL, *header = NULL;
    int force_new = 0, nops = 0;
    char **ops = xrealloc(NULL, (size_t)argc * sizeof *ops);
    setlocale(LC_ALL, "");
    for (int i = 1; i < argc; i++) {
        char *a = argv[i];
        if (strcmp(a, "--") == 0) {
            while (++i < argc) ops[nops++] = argv[i];
            break;
        }
        if (a[0] != '-' || a[1] == 0) {
            ops[nops++] = a;
            continue;
        }
        if (a[1] == '-') {
            char *name = a + 2, *eq = strchr(name, '=');
            size_t len = eq ? (size_t)(eq - name) : strlen(name);
            int hit = -1, hits = 0;
            for (int j = 0; j < NLONG; j++)
                if (strncmp(longs[j].name, name, len) == 0) {
                    if (strlen(longs[j].name) == len) {
                        hit = j, hits = 1;
                        break;
                    }
                    hit = j, hits++;
                }
            if (hits == 0) return bad("unrecognized option '%s'", a);
            if (hits > 1) {
                fprintf(stderr, "gencat: option '%.*s' is ambiguous; possibilities:", (int)(len + 2), a);
                for (int j = 0; j < NLONG; j++)
                    if (strncmp(longs[j].name, name, len) == 0) fprintf(stderr, " '--%s'", longs[j].name);
                fputs("\nTry `gencat --help' or `gencat --usage' for more information.\n", stderr);
                return 64;
            }
            char *val = NULL;
            char full[16];
            snprintf(full, sizeof full, "--%s", longs[hit].name);
            if (longs[hit].arg) {
                val = eq ? eq + 1 : (i + 1 < argc ? argv[++i] : NULL);
                if (!val) return bad("option '%s' requires an argument", full);
            } else if (eq) {
                return bad("option '%s' doesn't allow an argument", full);
            }
            switch (longs[hit].opt) {
            case 'H': header = val; break;
            case 'o': output = val; break;
            case 'n': force_new = 1; break;
            case '?': fputs(help_text, stdout); return 0;
            case 'u': fputs(usage_text, stdout); return 0;
            default: puts("gencat (libniji) 2.39"); return 0;
            }
            continue;
        }
        for (char *r = a + 1; *r; r++) {
            char c[2] = {*r, 0};
            switch (*r) {
            case 'H':
            case 'o': {
                char *val = r[1] ? r + 1 : (i + 1 < argc ? argv[++i] : NULL);
                if (!val) return bad("option requires an argument -- '%s'", c);
                *(*r == 'H' ? &header : &output) = val;
                r = val + strlen(val) - 1;
                break;
            }
            case '?': fputs(help_text, stdout); return 0;
            case 'V': puts("gencat (libniji) 2.39"); return 0;
            default: return bad("invalid option -- '%s'", c);
            }
        }
    }
    int first = 0;
    if (!output) output = nops ? ops[first++] : "-";
    if (first == nops) ops[nops++] = "-";
    for (int i = first; i < nops; i++) {
        if (strcmp(ops[i], "-") == 0) {
            source(stdin, ops[i]);
            continue;
        }
        FILE *f = fopen(ops[i], "re");
        if (!f) {
            /* No catalog is written (probe gencat-nofile). */
            fprintf(stderr, "cannot open input file `%s': %s\n", ops[i], strerror(errno));
            return 1;
        }
        source(f, ops[i]);
        fclose(f);
    }
    if (!force_new && strcmp(output, "-") != 0) merge_existing(output);
    if (write_catalog(output) != 0) return 1;
    if (header && write_header(header) != 0) return 1;
    return failed;
}
