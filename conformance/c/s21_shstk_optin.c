/* Spec §21.1 Guarantees ("With CET, the shadow stack ...") and §1.8 (tunables); D-21.6, D-22.29:
 * the user shadow stack is opt-in. With no tunable it stays off; `glibc.cpu.hwcaps=SHSTK` turns it
 * on for a process whose every object is marked, locked; `glibc.cpu.x86_shstk=off` vetoes;
 * `=permissive` leaves it unlocked; a later `glibc.cpu.hwcaps` entry replaces an earlier one. A
 * dlopen of an unmarked object once it is on: refused by default (the message names SHSTK, the
 * shadow stack stays on), loaded with it kept under `=on`, loaded after it is turned off under
 * `=permissive`. A thread's shadow stack is sized from the thread's stack, not RLIMIT_STACK.
 *
 * main runs each row as a fresh copy of itself (/proc/self/exe) with GLIBC_TUNABLES set; the copy
 * answers in its exit status: 1 on, 2 locked, 4 the unmarked object loaded, 8 on after the dlopen,
 * 16 the refusal names SHSTK, 32 a 256 KiB thread's shadow stack is under 1 MiB with RLIMIT_STACK
 * at 64 MiB. Only a CPU and kernel with user shadow stacks (`user_shstk` in /proc/cpuinfo) run the
 * rows, and only when this program itself carries the property (the build host's start files
 * decide): elsewhere it reports SKIPPED and exits 0. s21_shstk_optin_static runs the start rows on
 * a static image. */
/* DLOPEN-HELPERS: s21sk_unmarked */
/* HELPER-FLAGS: s21sk_unmarked -fcf-protection=none */
#define _GNU_SOURCE
#include <elf.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#ifndef S21SK_STATIC
#include <dlfcn.h>
#endif

extern char **environ;

/* Whether this image carries GNU_PROPERTY_X86_FEATURE_1_AND with the SHSTK bit (psABI §2.1.7). */
static int self_marked(void)
{
    const Elf64_Phdr *ph = (const Elf64_Phdr *)getauxval(AT_PHDR);
    size_t n = getauxval(AT_PHNUM);
    uintptr_t bias = 0;
    for (size_t i = 0; i < n; i++)
        if (ph[i].p_type == PT_PHDR)
            bias = (uintptr_t)ph - ph[i].p_vaddr;
    for (size_t i = 0; i < n; i++) {
        if (ph[i].p_type != 0x6474e553) /* PT_GNU_PROPERTY */
            continue;
        const unsigned char *p = (const unsigned char *)(bias + ph[i].p_vaddr), *end = p + ph[i].p_memsz;
        while (p + 12 <= end) {
            Elf64_Nhdr h;
            memcpy(&h, p, sizeof h);
            const unsigned char *d = p + 12 + ((h.n_namesz + 7) & ~7u), *dend = d + h.n_descsz;
            if (dend > end)
                return 0;
            if (h.n_type == 5) /* NT_GNU_PROPERTY_TYPE_0 */
                while (d + 8 <= dend) {
                    uint32_t type, size, word = 0;
                    memcpy(&type, d, 4);
                    memcpy(&size, d + 4, 4);
                    if (type == 0xc0000002 && size >= 4) {
                        memcpy(&word, d + 8, 4);
                        return (word & 2) != 0;
                    }
                    d += 8 + ((size + 7) & ~7u);
                }
            p = dend + ((8 - h.n_descsz % 8) % 8);
        }
    }
    return 0;
}

/* Bit 1 if /proc/self/status's x86_Thread_features lists shstk, bit 2 if the locked line does. */
static int status_bits(void)
{
    FILE *f = fopen("/proc/self/status", "r");
    char line[256];
    int bits = 0;
    if (!f)
        return 0;
    while (fgets(line, sizeof line, f)) {
        if (strncmp(line, "x86_Thread_features:", 20) == 0 && strstr(line + 20, "shstk"))
            bits |= 1;
        if (strncmp(line, "x86_Thread_features_locked:", 27) == 0 && strstr(line + 27, "shstk"))
            bits |= 2;
    }
    fclose(f);
    return bits;
}

static int has_user_shstk(void)
{
    FILE *f = fopen("/proc/cpuinfo", "r");
    char line[4096];
    int yes = 0;
    if (!f)
        return 0;
    while (!yes && fgets(line, sizeof line, f))
        yes = strncmp(line, "flags", 5) == 0 && strstr(line, " user_shstk") != 0;
    fclose(f);
    return yes;
}

/* The size of the mapping holding the thread's shadow-stack pointer (0 when it has none). */
static void *thread_shstk_size(void *out)
{
    unsigned long ssp = 0, lo, hi;
    __asm__ volatile("rdsspq %0" : "+r"(ssp));
    FILE *f = fopen("/proc/self/maps", "r");
    char line[512];
    *(unsigned long *)out = 0;
    while (ssp && f && fgets(line, sizeof line, f))
        if (sscanf(line, "%lx-%lx", &lo, &hi) == 2 && lo <= ssp && ssp < hi)
            *(unsigned long *)out = hi - lo;
    if (f)
        fclose(f);
    return 0;
}

/* The copy: mode 's' start state, 'd' also dlopen the unmarked helper, 't' also a thread. */
static int child(char mode)
{
    int bits = status_bits();
    if (mode == 't') {
        struct rlimit r;
        pthread_attr_t a;
        pthread_t t;
        unsigned long size = 0;
        getrlimit(RLIMIT_STACK, &r);
        r.rlim_cur = r.rlim_max == RLIM_INFINITY || r.rlim_max >= (64ul << 20) ? (64ul << 20) : r.rlim_max;
        setrlimit(RLIMIT_STACK, &r);
        pthread_attr_init(&a);
        pthread_attr_setstacksize(&a, 256 << 10);
        if (pthread_create(&t, &a, thread_shstk_size, &size) == 0 && pthread_join(t, 0) == 0)
            printf("thread shadow stack %lu bytes\n", size);
        if (size != 0 && size < (1ul << 20))
            bits |= 32;
    }
#ifndef S21SK_STATIC
    if (mode == 'd') {
        char path[4096];
        const char *dir = getenv("CONF_HELPERS");
        snprintf(path, sizeof path, "%s/libs21sk_unmarked.so", dir ? dir : ".");
        void *h = dlopen(path, RTLD_NOW);
        const char *e = h ? 0 : dlerror();
        printf("dlopen: %s\n", h ? "loaded" : e ? e : "(no message)");
        if (h)
            bits |= 4;
        if (e && strstr(e, "SHSTK"))
            bits |= 16;
        if (status_bits() & 1)
            bits |= 8;
    }
#endif
    fflush(stdout);
    return bits;
}

/* Runs one row: this program as a copy with GLIBC_TUNABLES=tun (none if empty) in mode. */
static int run(const char *tun, char mode)
{
    size_t n = 0;
    while (environ[n])
        n++;
    char **env = calloc(n + 2, sizeof *env), var[256], m[2] = {mode, 0};
    size_t k = 0;
    for (size_t i = 0; i < n; i++)
        if (strncmp(environ[i], "GLIBC_TUNABLES=", 15) != 0)
            env[k++] = environ[i];
    if (*tun) {
        snprintf(var, sizeof var, "GLIBC_TUNABLES=%s", tun);
        env[k++] = var;
    }
    env[k] = 0;
    char *argv[] = {"s21_shstk_optin", "--child", m, 0};
    fflush(stdout);
    pid_t p = fork();
    if (p == 0) {
        execve("/proc/self/exe", argv, env);
        _exit(127);
    }
    int st = 0;
    waitpid(p, &st, 0);
    free(env);
    return WIFEXITED(st) ? WEXITSTATUS(st) : 128 + WTERMSIG(st);
}

static const struct row {
    const char *tun;
    char mode;
    int want;
} rows[] = {
    {"", 's', 0},
    {"glibc.cpu.hwcaps=SHSTK", 's', 3},
    {"glibc.cpu.hwcaps=IBT,SHSTK", 's', 3},
    {"glibc.cpu.hwcaps=-SHSTK,SHSTK", 's', 3},
    {"glibc.cpu.hwcaps=SHSTK,-SHSTK", 's', 0},
    {"glibc.cpu.hwcaps=shstk", 's', 0},
    {"glibc.cpu.hwcaps=SHSTK:glibc.cpu.hwcaps=AVX2", 's', 0},
    {"glibc.cpu.x86_shstk=on", 's', 0},
    {"glibc.cpu.x86_shstk=permissive", 's', 0},
    {"glibc.cpu.x86_shstk=off:glibc.cpu.hwcaps=SHSTK", 's', 0},
    {"glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on", 's', 3},
    {"glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=bogus", 's', 3},
    {"glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=permissive", 's', 1},
    {"glibc.cpu.hwcaps=SHSTK", 't', 3 | 32},
#ifndef S21SK_STATIC
    {"glibc.cpu.hwcaps=SHSTK", 'd', 3 | 8 | 16},
    {"glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on", 'd', 3 | 4 | 8},
    {"glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=permissive", 'd', 1 | 4},
    {"", 'd', 4},
#endif
};

int main(int argc, char **argv)
{
    if (argc == 3 && strcmp(argv[1], "--child") == 0)
        return child(argv[2][0]);
    if (!has_user_shstk()) {
        printf("SKIPPED: no user_shstk in /proc/cpuinfo (CPU or kernel without user shadow stacks)\n");
        return 0;
    }
    if (!self_marked()) {
        printf("SKIPPED: this program lacks the SHSTK property (the build's start files are unmarked)\n");
        return 0;
    }
    int failures = 0;
    for (size_t i = 0; i < sizeof rows / sizeof rows[0]; i++) {
        int got = run(rows[i].tun, rows[i].mode);
        printf("%s [%s] %c: %d (want %d)\n", got == rows[i].want ? "ok  " : "FAIL", rows[i].tun, rows[i].mode, got,
               rows[i].want);
        failures += got != rows[i].want;
    }
    printf("%d failures\n", failures);
    return failures != 0;
}
