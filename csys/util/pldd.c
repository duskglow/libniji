/* pldd (UTIL-TZSELECT-PLDD, spec §31.13, D-31.35): lists the shared objects loaded into a running process, those
 * loaded with dlopen included, linked dynamically like zdump.
 *
 * Mechanism (D-31.35), from documented interfaces only: the target's tasks are attached with PTRACE_SEIZE and
 * stopped with PTRACE_INTERRUPT (ptrace(2)), which both checks that the caller may inspect the process (the
 * "cannot attach" error) and keeps the loader from changing its list while it is read; the program's program headers
 * come from AT_PHDR/AT_PHNUM of /proc/PID/auxv (proc(5)), its PT_DYNAMIC holds DT_DEBUG, the address of the
 * debugger rendezvous `struct r_debug` (<link.h>, the System V ABI's DT_DEBUG), whose r_map starts the `link_map`
 * list; every object after the first (the program) is printed by its l_name, in list order. Memory is read with
 * pread on /proc/PID/mem. The target's memory is untrusted: every read may fail, the walk stops after MAX_OBJECTS
 * entries and a name is cut at PATH_MAX bytes, so a corrupt or cyclic list ends the listing instead of the program.
 *
 * Output and errors as probed against the reference (handoff/UTIL-TZSELECT-PLDD.md): `<pid>:\t<program>` then one
 * name per line, exit 0; a process without the rendezvous (a static program) prints nothing, exit 1. */
#define _GNU_SOURCE
#include <dirent.h>
#include <elf.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/wait.h>
#include <unistd.h>

enum { MAX_OBJECTS = 1 << 16, MAX_TASKS = 1 << 16, MAX_SCANS = 64 };

static const char try_line[] = "Try `pldd --help' or `pldd --usage' for more information.\n";
static const char usage_text[] = "Usage: pldd [-?V] [--help] [--usage] [--version] PID\n";
static const char help_text[] =
    "Usage: pldd [OPTION...] PID\n"
    "List dynamic shared objects loaded into process.\n"
    "\n"
    "  -?, --help                 Give this help list\n"
    "      --usage                Give a short usage message\n"
    "  -V, --version              Print program version\n";

static const char *prog = "pldd"; /* messages begin with argv[0]; the Try line names pldd (probed) */
static int memfd = -1;
static int word = 8; /* the target's pointer size: 8 (ELFCLASS64) or 4 (ELFCLASS32) */

/* The attached tasks and the signal each was stopped with (re-delivered at detach). */
static struct task {
    pid_t tid;
    int sig;
} *tasks;
static size_t ntasks;

static int bad(const char *fmt, const char *what) {
    fprintf(stderr, "%s: ", prog);
    fprintf(stderr, fmt, what);
    fputc('\n', stderr);
    fputs(try_line, stderr);
    return 64;
}

static void detach_all(void) {
    for (size_t i = 0; i < ntasks; i++) ptrace(PTRACE_DETACH, tasks[i].tid, 0, (void *)(intptr_t)tasks[i].sig);
    ntasks = 0;
}

static int attached(pid_t tid) {
    for (size_t i = 0; i < ntasks; i++)
        if (tasks[i].tid == tid) return 1;
    return 0;
}

/* Seizes and stops one task. Returns 0, or the errno of the failed PTRACE_SEIZE. */
static int seize(pid_t tid) {
    if (ptrace(PTRACE_SEIZE, tid, 0, 0) != 0) return errno;
    struct task *t = realloc(tasks, (ntasks + 1) * sizeof *tasks);
    if (!t) {
        ptrace(PTRACE_DETACH, tid, 0, 0);
        return ENOMEM;
    }
    tasks = t;
    tasks[ntasks].tid = tid;
    tasks[ntasks].sig = 0;
    ntasks++;
    if (ptrace(PTRACE_INTERRUPT, tid, 0, 0) != 0) return 0; /* the task is exiting */
    int st;
    while (waitpid(tid, &st, __WALL) < 0)
        if (errno != EINTR) return 0;
    /* A signal-delivery stop (no event in the high bits) holds a signal the task has not seen yet. */
    if (WIFSTOPPED(st) && (st >> 16) == 0) tasks[ntasks - 1].sig = WSTOPSIG(st);
    return 0;
}

/* Attaches every task of pid, rescanning /proc/PID/task until a scan finds none new (a task may start a thread
 * while the others are being stopped). Returns 0 or the errno of attaching the process itself. */
static int attach(pid_t pid) {
    int err = seize(pid);
    if (err) return err;
    char path[64];
    snprintf(path, sizeof path, "/proc/%d/task", (int)pid);
    for (int scan = 0; scan < MAX_SCANS; scan++) {
        DIR *d = opendir(path);
        if (!d) return 0;
        int added = 0;
        struct dirent *e;
        while ((e = readdir(d)) && ntasks < MAX_TASKS) {
            char *end;
            long tid = strtol(e->d_name, &end, 10);
            if (*end || tid <= 0 || attached((pid_t)tid)) continue;
            if (seize((pid_t)tid) == 0) added = 1;
        }
        closedir(d);
        if (!added) break;
    }
    return 0;
}

static int rd(uint64_t addr, void *buf, size_t n) {
    if (addr > (uint64_t)INT64_MAX - n) return -1;
    return pread(memfd, buf, n, (off_t)addr) == (ssize_t)n ? 0 : -1;
}

/* One target word at addr (pointer-sized). */
static int rdw(uint64_t addr, uint64_t *v) {
    if (word == 8) return rd(addr, v, 8);
    uint32_t w;
    if (rd(addr, &w, 4) != 0) return -1;
    *v = w;
    return 0;
}

/* The NUL-terminated string at addr, at most PATH_MAX bytes (cut there), into buf. */
static int rdstr(uint64_t addr, char *buf) {
    size_t got = 0;
    while (got < PATH_MAX) {
        size_t chunk = 256 - (size_t)((addr + got) % 256); /* never across a page from a readable byte */
        if (chunk > PATH_MAX - got) chunk = PATH_MAX - got;
        ssize_t r = pread(memfd, buf + got, chunk, (off_t)(addr + got));
        if (r <= 0) return got ? (buf[got] = 0, 0) : -1;
        char *z = memchr(buf + got, 0, (size_t)r);
        if (z) return 0;
        got += (size_t)r;
    }
    buf[PATH_MAX] = 0;
    return 0;
}

/* The program's r_debug address from its auxiliary vector and dynamic section; 0 when it has none. */
static uint64_t rendezvous(pid_t pid) {
    char path[64];
    snprintf(path, sizeof path, "/proc/%d/auxv", (int)pid);
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    unsigned char av[8192];
    ssize_t n = read(fd, av, sizeof av);
    close(fd);
    uint64_t phdr = 0, phnum = 0;
    for (ssize_t off = 0; n > 0 && off + 2 * word <= n; off += 2 * word) {
        uint64_t key = 0, val = 0;
        memcpy(&key, av + off, (size_t)word);
        memcpy(&val, av + off + word, (size_t)word);
        if (key == AT_NULL) break;
        if (key == AT_PHDR) phdr = val;
        if (key == AT_PHNUM) phnum = val;
    }
    if (!phdr || !phnum || phnum > 0xffff) return 0;
    /* Elf64_Phdr: 56 bytes, p_vaddr at 16, p_memsz at 40; Elf32_Phdr: 32 bytes, at 8 and 20. */
    size_t phent = word == 8 ? 56 : 32, vaddr_at = word == 8 ? 16 : 8, memsz_at = word == 8 ? 40 : 20;
    uint64_t bias = 0, dyn = 0, dynsz = 0;
    int have_bias = 0, have_dyn = 0;
    for (uint64_t i = 0; i < phnum; i++) {
        uint32_t type;
        uint64_t vaddr, memsz;
        uint64_t at = phdr + i * phent;
        if (rd(at, &type, 4) != 0 || rdw(at + vaddr_at, &vaddr) != 0 || rdw(at + memsz_at, &memsz) != 0) return 0;
        if (type == PT_PHDR) bias = phdr - vaddr, have_bias = 1;
        if (type == PT_DYNAMIC) dyn = vaddr, dynsz = memsz, have_dyn = 1;
    }
    if (!have_dyn) return 0;
    if (!have_bias) {
        /* No PT_PHDR: the load bias is AT_ENTRY less the program's own e_entry. */
        uint64_t entry = 0;
        for (ssize_t off = 0; n > 0 && off + 2 * word <= n; off += 2 * word) {
            uint64_t key = 0, val = 0;
            memcpy(&key, av + off, (size_t)word);
            memcpy(&val, av + off + word, (size_t)word);
            if (key == AT_NULL) break;
            if (key == AT_ENTRY) entry = val;
        }
        snprintf(path, sizeof path, "/proc/%d/exe", (int)pid);
        fd = open(path, O_RDONLY | O_CLOEXEC);
        if (fd < 0) return 0;
        unsigned char eh[64];
        ssize_t got = pread(fd, eh, sizeof eh, 0);
        close(fd);
        uint64_t e_entry = 0;
        if (got < 32) return 0;
        memcpy(&e_entry, eh + 24, (size_t)word);
        bias = entry - e_entry;
    }
    uint64_t at = bias + dyn, count = dynsz / (2 * (uint64_t)word);
    if (count > 4096) count = 4096;
    for (uint64_t i = 0; i < count; i++, at += 2 * (uint64_t)word) {
        uint64_t tag, val;
        if (rdw(at, &tag) != 0 || rdw(at + (uint64_t)word, &val) != 0 || tag == DT_NULL) return 0;
        if (tag == DT_DEBUG) return val;
    }
    return 0;
}

/* The target's pointer size from its program's ELF identification (8 when it cannot be read). */
static int class_of(pid_t pid) {
    char path[64];
    snprintf(path, sizeof path, "/proc/%d/exe", (int)pid);
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    unsigned char id[EI_NIDENT];
    ssize_t got = fd >= 0 ? pread(fd, id, sizeof id, 0) : -1;
    if (fd >= 0) close(fd);
    return got == EI_NIDENT && memcmp(id, ELFMAG, SELFMAG) == 0 && id[EI_CLASS] == ELFCLASS32 ? 4 : 8;
}

/* Prints the listing; returns the exit status. */
static int list(pid_t pid) {
    word = class_of(pid);
    uint64_t r = rendezvous(pid), map;
    /* struct r_debug: int r_version, then r_map at one word. */
    int32_t version;
    if (!r || rd(r, &version, 4) != 0 || version < 1 || rdw(r + (uint64_t)word, &map) != 0 || !map) return 1;
    char path[64], exe[PATH_MAX + 1];
    snprintf(path, sizeof path, "/proc/%d/exe", (int)pid);
    ssize_t n = readlink(path, exe, PATH_MAX);
    exe[n > 0 ? n : 0] = 0;
    printf("%d:\t%s\n", (int)pid, exe);
    /* struct link_map: l_addr, l_name, l_ld, l_next, l_prev, one word each. The first entry is the program. */
    static char name[PATH_MAX + 1];
    uint64_t next;
    if (rdw(map + 3 * (uint64_t)word, &next) != 0) return 0;
    for (unsigned i = 0; next && i < MAX_OBJECTS; i++) {
        uint64_t l_name;
        if (rdw(next + (uint64_t)word, &l_name) != 0) break;
        if (!l_name || rdstr(l_name, name) != 0) name[0] = 0;
        puts(name);
        if (rdw(next + 3 * (uint64_t)word, &next) != 0) break;
    }
    return 0;
}

int main(int argc, char **argv) {
    static const struct {
        const char *name;
        char opt;
    } longs[] = {{"help", '?'}, {"usage", 'u'}, {"version", 'V'}};
    enum { NLONG = sizeof longs / sizeof longs[0] };
    char *pidarg = NULL;
    if (argc > 0 && argv[0] && *argv[0]) prog = argv[0];
    int nops = 0;
    for (int i = 1; i < argc; i++) {
        char *a = argv[i];
        if (strcmp(a, "--") == 0) {
            while (++i < argc) pidarg = argv[i], nops++;
            break;
        }
        if (a[0] != '-' || a[1] == 0) {
            pidarg = a, nops++;
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
                fprintf(stderr, "%s: option '%.*s' is ambiguous; possibilities:", prog, (int)(len + 2), a);
                for (int j = 0; j < NLONG; j++)
                    if (strncmp(longs[j].name, name, len) == 0) fprintf(stderr, " '--%s'", longs[j].name);
                fputc('\n', stderr);
                fputs(try_line, stderr);
                return 64;
            }
            if (eq) {
                char full[16];
                snprintf(full, sizeof full, "--%s", longs[hit].name);
                return bad("option '%s' doesn't allow an argument", full);
            }
            switch (longs[hit].opt) {
            case '?': fputs(help_text, stdout); return 0;
            case 'u': fputs(usage_text, stdout); return 0;
            default: puts("pldd (libniji) 2.39"); return 0;
            }
        }
        for (char *r = a + 1; *r; r++) {
            char c[2] = {*r, 0};
            switch (*r) {
            case '?': fputs(help_text, stdout); return 0;
            case 'V': puts("pldd (libniji) 2.39"); return 0;
            default: return bad("invalid option -- '%s'", c);
            }
        }
    }
    if (nops != 1) {
        fputs("Exactly one parameter with process ID required.\n", stderr);
        fputs(try_line, stderr);
        return 1;
    }
    /* A decimal process id: blanks and one '+' before the digits, nothing after (probed: "+N", " N", "0N"). */
    const char *p = pidarg;
    while (*p == ' ' || (*p >= '\t' && *p <= '\r')) p++;
    if (*p == '+') p++;
    long long v = 0;
    int digits = 0;
    for (; *p >= '0' && *p <= '9'; p++, digits++)
        if ((v = v * 10 + (*p - '0')) > INT_MAX) break;
    if (!digits || *p) {
        fprintf(stderr, "%s: invalid process ID '%s'\n", prog, pidarg);
        return 1;
    }
    pid_t pid = (pid_t)v;
    char path[64];
    snprintf(path, sizeof path, "/proc/%d", (int)pid);
    int dir = open(path, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (dir < 0) {
        fprintf(stderr, "%s: cannot open %s: %s\n", prog, path, strerror(errno));
        return 1;
    }
    close(dir);
    int err = attach(pid);
    if (err) {
        fprintf(stderr, "%s: cannot attach to process %d: %s\n", prog, (int)pid, strerror(err));
        detach_all();
        return 1;
    }
    snprintf(path, sizeof path, "/proc/%d/mem", (int)pid);
    memfd = open(path, O_RDONLY | O_CLOEXEC);
    int rc = memfd < 0 ? 1 : list(pid);
    fflush(stdout);
    detach_all();
    return rc;
}
