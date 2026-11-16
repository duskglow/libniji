/*
 * ldd (UTIL-4, spec §31.3, plan/utilities.md §4, D-31.7): lists the shared objects each operand needs through
 * Liella's loader in trace mode (LD_TRACE_LOADED_OBJECTS, D-27.16), so ldd and the loader cannot disagree. ldd
 * never runs the operand's code or another interpreter: it reads the ELF headers itself, answers static programs
 * and objects with no DT_NEEDED alone, and starts only Liella's loader (LDD_RTLD) on the rest. `-v` reads each
 * listed object's version needs (DT_VERNEED, ELF gABI "Symbol Versioning") itself. `-d`, `-r` and `-u` are
 * refused until the loader reports relocations in trace mode (D-31.7).
 *
 * A static PIE over libc.a like getconf (`cargo xtask so`, run_ldd), so it works whatever state the dynamic
 * libraries are in.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

#ifndef LDD_RTLD
#define LDD_RTLD "/lib64/ld-linux-x86-64.so.2"
#endif
/* The x86-64 psABI program interpreter: on a Liella system, Liella's loader. */
#define PSABI_INTERP "/lib64/ld-linux-x86-64.so.2"

enum { PT_DYNAMIC_ = 2, PT_INTERP_ = 3, PT_LOAD_ = 1, DT_NEEDED_ = 1, DT_STRTAB_ = 5, DT_STRSZ_ = 10,
       DT_VERNEED_ = 0x6ffffffe, DT_VERNEEDNUM_ = 0x6fffffff };

static const char USAGE[] =
    "Usage: ldd [OPTION]... FILE...\n"
    "      --help              print this help and exit\n"
    "      --version           print version information and exit\n"
    "  -d, --data-relocs       process data relocations\n"
    "  -r, --function-relocs   process data and function relocations\n"
    "  -u, --unused            print unused direct dependencies\n"
    "  -v, --verbose           print all information\n";

/* A file mapped read-only, with bounds-checked readers. */
struct image { const uint8_t *b; size_t n; };

static uint64_t rd(const struct image *im, uint64_t at, unsigned w) {
    uint64_t v = 0;
    if (at > im->n || w > im->n - at) return 0;
    for (unsigned i = w; i-- > 0;) v = (v << 8) | im->b[at + i];
    return v;
}

static int map_file(const char *path, struct image *im) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    struct stat st;
    if (fstat(fd, &st) != 0) { int e = errno; close(fd); errno = e; return -1; }
    im->n = (size_t)st.st_size;
    im->b = NULL;
    if (im->n > 0) {
        void *p = mmap(NULL, im->n, PROT_READ, MAP_PRIVATE, fd, 0);
        if (p == MAP_FAILED) { int e = errno; close(fd); errno = e; return -1; }
        im->b = p;
    }
    close(fd);
    return 0;
}

static void unmap_file(struct image *im) {
    if (im->b) munmap((void *)im->b, im->n);
    im->b = NULL;
}

/* What the headers say: the dynamic section's file range, the interpreter, the number of DT_NEEDED entries. */
struct elfinfo { int dynamic; uint64_t dyn_off, dyn_size; const char *interp; size_t interp_len; unsigned needed; };

/* 0: a 64-bit little-endian x86-64 executable or shared object with a PT_DYNAMIC; -1 otherwise. */
static int inspect(const struct image *im, struct elfinfo *e) {
    memset(e, 0, sizeof *e);
    if (im->n < 64 || memcmp(im->b, "\177ELF", 4) != 0 || im->b[4] != 2 || im->b[5] != 1) return -1;
    uint64_t type = rd(im, 16, 2), machine = rd(im, 18, 2);
    if ((type != 2 && type != 3) || machine != 62) return -1;
    uint64_t phoff = rd(im, 32, 8), phent = rd(im, 54, 2), phnum = rd(im, 56, 2);
    if (phent < 56) return -1;
    for (uint64_t i = 0; i < phnum; i++) {
        uint64_t ph = phoff + i * phent;
        if (ph > im->n || im->n - ph < 56) return -1;
        uint32_t pt = (uint32_t)rd(im, ph, 4);
        uint64_t off = rd(im, ph + 8, 8), filesz = rd(im, ph + 32, 8);
        if (off > im->n || filesz > im->n - off) continue;
        if (pt == PT_DYNAMIC_) { e->dynamic = 1; e->dyn_off = off; e->dyn_size = filesz; }
        if (pt == PT_INTERP_ && filesz > 0) {
            e->interp = (const char *)im->b + off;
            e->interp_len = strnlen(e->interp, filesz);
        }
    }
    if (!e->dynamic) return -1;
    for (uint64_t d = e->dyn_off; d + 16 <= e->dyn_off + e->dyn_size; d += 16) {
        uint64_t tag = rd(im, d, 8);
        if (tag == 0) break;
        if (tag == DT_NEEDED_) e->needed++;
    }
    return 0;
}

/* The file offset of virtual address `va` through the PT_LOAD segments, or UINT64_MAX. */
static uint64_t va_to_off(const struct image *im, uint64_t va) {
    uint64_t phoff = rd(im, 32, 8), phent = rd(im, 54, 2), phnum = rd(im, 56, 2);
    for (uint64_t i = 0; i < phnum; i++) {
        uint64_t ph = phoff + i * phent;
        if (rd(im, ph, 4) != PT_LOAD_) continue;
        uint64_t off = rd(im, ph + 8, 8), vaddr = rd(im, ph + 16, 8), filesz = rd(im, ph + 32, 8);
        if (va >= vaddr && va - vaddr < filesz) return off + (va - vaddr);
    }
    return UINT64_MAX;
}

/* The loader's listing of one operand: (name, path) per line, the interpreter's path last. */
struct listed { const char *name; size_t name_len; const char *path; size_t path_len; };

static size_t parse_list(char *out, size_t n, struct listed *l, size_t max) {
    size_t k = 0;
    for (char *line = out; line < out + n && k < max;) {
        char *end = memchr(line, '\n', (size_t)(out + n - line));
        if (!end) end = out + n;
        if (line[0] == '\t') {
            char *s = line + 1, *arrow = NULL, *paren = NULL;
            for (char *c = s; c + 4 <= end; c++)
                if (!arrow && memcmp(c, " => ", 4) == 0) arrow = c;
            for (char *c = end; c-- > s;)
                if (*c == ' ' && c + 1 < end && c[1] == '(') { paren = c; break; }
            if (arrow && paren && paren > arrow + 4) {
                l[k++] = (struct listed){ s, (size_t)(arrow - s), arrow + 4, (size_t)(paren - arrow - 4) };
            } else if (!arrow && paren && s[0] == '/') {
                /* The interpreter: matched by the last component of its path. */
                const char *base = s;
                for (const char *c = s; c < paren; c++)
                    if (*c == '/') base = c + 1;
                l[k++] = (struct listed){ base, (size_t)(paren - base), s, (size_t)(paren - s) };
            }
        }
        line = end + 1;
    }
    return k;
}

/* Whether the object at `path` has a DT_NEEDED entry `name` (`len` bytes). */
static int has_needed(const char *path, const char *name, size_t len) {
    struct image im;
    struct elfinfo e;
    int found = 0;
    if (map_file(path, &im) != 0) return 0;
    if (inspect(&im, &e) == 0) {
        uint64_t strtab = 0, strsz = 0;
        for (uint64_t d = e.dyn_off; d + 16 <= e.dyn_off + e.dyn_size; d += 16) {
            uint64_t tag = rd(&im, d, 8), val = rd(&im, d + 8, 8);
            if (tag == 0) break;
            if (tag == DT_STRTAB_) strtab = val;
            if (tag == DT_STRSZ_) strsz = val;
        }
        uint64_t str = va_to_off(&im, strtab);
        for (uint64_t d = e.dyn_off; str != UINT64_MAX && str <= im.n && strsz <= im.n - str && d + 16 <= e.dyn_off + e.dyn_size; d += 16) {
            uint64_t tag = rd(&im, d, 8), val = rd(&im, d + 8, 8);
            if (tag == 0) break;
            if (tag == DT_NEEDED_ && val < strsz && strsz - val > len && memcmp(im.b + str + val, name, len) == 0 && im.b[str + val + len] == 0)
                found = 1;
        }
    }
    unmap_file(&im);
    return found;
}

/* `-v`: one object's version needs, `\t<shown>:` then `\t\t<file> (<version>) => <path>` each. */
static void print_needs(const char *path, const char *shown, const struct listed *l, size_t nl, int *header) {
    struct image im;
    struct elfinfo e;
    if (map_file(path, &im) != 0) return;
    if (inspect(&im, &e) == 0) {
        uint64_t strtab = 0, strsz = 0, verneed = 0, num = 0;
        for (uint64_t d = e.dyn_off; d + 16 <= e.dyn_off + e.dyn_size; d += 16) {
            uint64_t tag = rd(&im, d, 8), val = rd(&im, d + 8, 8);
            if (tag == 0) break;
            if (tag == DT_STRTAB_) strtab = val;
            if (tag == DT_STRSZ_) strsz = val;
            if (tag == DT_VERNEED_) verneed = val;
            if (tag == DT_VERNEEDNUM_) num = val;
        }
        uint64_t str = va_to_off(&im, strtab), vn = va_to_off(&im, verneed);
        if (verneed && num && str != UINT64_MAX && vn != UINT64_MAX && str <= im.n && strsz <= im.n - str) {
            if (!*header) fputs("\n\tVersion information:\n", stdout);
            *header = 1;
            printf("\t%s:\n", shown);
#define STR(o) ((o) < strsz ? (const char *)im.b + str + (o) : "")
#define STROK(o) ((o) < strsz && memchr(im.b + str + (o), 0, strsz - (o)))
            for (uint64_t i = 0; i < num && vn + 16 <= im.n; i++) {
                uint64_t cnt = rd(&im, vn + 2, 2), file = rd(&im, vn + 4, 4), aux = rd(&im, vn + 8, 4), next = rd(&im, vn + 12, 4);
                const char *fname = STROK(file) ? STR(file) : "";
                const char *to = "not found";
                size_t to_len = 9;
                for (size_t j = 0; j < nl; j++)
                    if (l[j].name_len == strlen(fname) && memcmp(l[j].name, fname, l[j].name_len) == 0) {
                        to = l[j].path;
                        to_len = l[j].path_len;
                        break;
                    }
                uint64_t a = vn + aux;
                for (uint64_t c = 0; c < cnt && a + 16 <= im.n; c++) {
                    uint64_t name = rd(&im, a + 8, 4), anext = rd(&im, a + 12, 4);
                    printf("\t\t%s (%s) => %.*s\n", fname, STROK(name) ? STR(name) : "", (int)to_len, to);
                    if (anext == 0) break;
                    a += anext;
                }
                if (next == 0) break;
                vn += next;
            }
#undef STR
#undef STROK
        }
    }
    unmap_file(&im);
}

/* Run Liella's loader in trace mode on `shown`; its listing goes to stdout (and is returned for `-v`). */
static int trace(const char *shown, char **out, size_t *n) {
    int fd[2];
    if (pipe2(fd, O_CLOEXEC) != 0) return -1;
    fflush(stdout);
    pid_t pid = fork();
    if (pid < 0) { close(fd[0]); close(fd[1]); return -1; }
    if (pid == 0) {
        dup2(fd[1], 1);
        setenv("LD_TRACE_LOADED_OBJECTS", "1", 1);
        execl(LDD_RTLD, LDD_RTLD, shown, (char *)NULL);
        fprintf(stderr, "ldd: %s: %s\n", LDD_RTLD, strerror(errno));
        _exit(127);
    }
    close(fd[1]);
    size_t cap = 4096, len = 0;
    char *buf = malloc(cap);
    for (;;) {
        if (buf && len == cap) {
            char *nb = realloc(buf, cap * 2);
            if (!nb) { free(buf); buf = NULL; }
            else { buf = nb; cap *= 2; }
        }
        if (!buf) break;
        ssize_t r = read(fd[0], buf + len, cap - len);
        if (r < 0 && errno == EINTR) continue;
        if (r <= 0) break;
        len += (size_t)r;
    }
    close(fd[0]);
    int st;
    while (waitpid(pid, &st, 0) < 0 && errno == EINTR) {}
    if (!buf) return -1;
    *out = buf;
    *n = len;
    return WIFEXITED(st) && WEXITSTATUS(st) == 0 ? 0 : -1;
}

/* One operand: 0 when listed, 1 on a failure (the reason already printed). */
static int one(const char *file, int verbose) {
    size_t fl = strlen(file);
    char *shown = malloc(fl + 3);
    if (!shown) return 1;
    if (strchr(file, '/')) memcpy(shown, file, fl + 1);
    else { memcpy(shown, "./", 2); memcpy(shown + 2, file, fl + 1); }
    struct stat st;
    int rc = 1;
    struct image im = { NULL, 0 };
    struct elfinfo e;
    if (stat(shown, &st) != 0) {
        fflush(stdout);
        fprintf(stderr, "ldd: %s: %s\n", shown, strerror(errno));
    } else if (!S_ISREG(st.st_mode)) {
        fflush(stdout);
        fprintf(stderr, "ldd: %s: not regular file\n", shown);
    } else if (map_file(shown, &im) != 0) {
        fflush(stdout);
        fprintf(stderr, "ldd: %s: %s\n", shown, strerror(errno));
    } else if (inspect(&im, &e) != 0) {
        fflush(stdout);
        fputs("\tnot a dynamic executable\n", stderr);
    } else if (e.needed == 0) {
        puts("\tstatically linked");
        rc = 0;
    } else {
        if (e.interp && (e.interp_len != strlen(PSABI_INTERP) || memcmp(e.interp, PSABI_INTERP, e.interp_len) != 0)) {
            fflush(stdout);
            fprintf(stderr, "ldd: %s: program interpreter %.*s is not Liella's loader: listed by Liella's loader's rules\n",
                    shown, (int)e.interp_len, e.interp);
        }
        char *out = NULL;
        size_t n = 0;
        rc = trace(shown, &out, &n) == 0 ? 0 : 1;
        static struct listed l[256];
        size_t nl = out ? parse_list(out, n, l, 256) : 0;
        /* The interpreter line (a path with no `=>`, last in the list) is printed only when the program or a
           listed object names the interpreter in DT_NEEDED, as the reference lists it (transcripts). */
        const char *skip = NULL;
        if (nl > 0 && l[nl - 1].path[0] == '/' && l[nl - 1].path + l[nl - 1].path_len - l[nl - 1].name_len == l[nl - 1].name) {
            int needed = has_needed(shown, l[nl - 1].name, l[nl - 1].name_len) || has_needed(shown, l[nl - 1].path, l[nl - 1].path_len);
            for (size_t j = 0; j + 1 < nl && !needed; j++) {
                char p[4096];
                if (l[j].path[0] != '/' || l[j].path_len >= sizeof p) continue;
                memcpy(p, l[j].path, l[j].path_len);
                p[l[j].path_len] = 0;
                needed = has_needed(p, l[nl - 1].name, l[nl - 1].name_len) || has_needed(p, l[nl - 1].path, l[nl - 1].path_len);
            }
            if (!needed) skip = l[nl - 1].path - 1;
        }
        for (char *line = out; out && line < out + n;) {
            char *end = memchr(line, 0x0a, (size_t)(out + n - line));
            end = end ? end + 1 : out + n;
            if (line != skip) fwrite(line, 1, (size_t)(end - line), stdout);
            line = end;
        }
        if (skip) nl--;
        if (rc == 0 && verbose) {
            int header = 0;
            print_needs(shown, shown, l, nl, &header);
            for (size_t j = 0; j < nl; j++) {
                if (l[j].path[0] != '/') continue;
                char p[4096];
                if (l[j].path_len >= sizeof p) continue;
                memcpy(p, l[j].path, l[j].path_len);
                p[l[j].path_len] = 0;
                print_needs(p, p, l, nl, &header);
            }
        }
        free(out);
    }
    unmap_file(&im);
    free(shown);
    return rc;
}

static int usage_error(const char *fmt, const char *arg, int try_line) {
    fprintf(stderr, fmt, arg);
    if (try_line) fputs("Try `ldd --help' for more information.\n", stderr);
    return 1;
}

int main(int argc, char **argv) {
    static const char *const longs[] = { "help", "version", "data-relocs", "function-relocs", "unused", "verbose" };
    static const char shorts[] = { 0, 0, 'd', 'r', 'u', 'v' };
    int verbose = 0, i = 1;
    for (; i < argc && argv[i][0] == '-'; i++) {
        const char *a = argv[i];
        int opt = -1;
        if (strcmp(a, "--") == 0) { i++; break; }
        if (a[1] == '-') {
            size_t len = strlen(a + 2), hits = 0;
            for (int k = 0; k < 6; k++)
                if (strncmp(longs[k], a + 2, len) == 0) {
                    if (longs[k][len] == 0) { opt = k; hits = 1; break; }
                    opt = k;
                    hits++;
                }
            if (hits > 1) return usage_error("ldd: option `%s' is ambiguous\n", a, 0);
            if (hits == 0 || len == 0) opt = -1;
        } else if (a[1] && !a[2]) {
            for (int k = 2; k < 6; k++)
                if (a[1] == shorts[k]) opt = k;
        }
        if (opt < 0) return usage_error("ldd: unrecognized option `%s'\n", a, 1);
        if (opt == 0) { fputs(USAGE, stdout); return 0; }
        if (opt == 1) {
            puts("ldd (Liella) 2.39");
            return 0;
        }
        if (opt == 5) { verbose = 1; continue; }
        fprintf(stderr, "ldd: option `%s' is not supported: the loader does not report relocations in trace mode yet\n", a);
        return 1;
    }
    if (i >= argc) return usage_error("ldd: missing file arguments\n", "", 1);
    int rc = 0, several = argc - i > 1;
    for (; i < argc; i++) {
        if (several) printf("%s:\n", argv[i]);
        rc |= one(argv[i], verbose);
    }
    if (fflush(stdout) != 0) rc = 1;
    return rc;
}
