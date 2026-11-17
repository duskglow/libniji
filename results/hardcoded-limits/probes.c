/* AUDIT-HARDCODED-LIMITS black-box probes: run a library past a fixed count and report where (if
 * anywhere) it stops.  One program, one probe per argv[1]; each prints one result line.
 * Built with: cc -O1 -o probes probes.c -ldl -lanl */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <fmtmsg.h>
#include <fnmatch.h>
#include <locale.h>
#include <netdb.h>
#include <printf.h>
#include <semaphore.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>
#include <wordexp.h>

static long calls;
static void count(void) { calls++; }

static int dlopen_many(const char *dir, int n) {
    char p[4096];
    for (int i = 0; i < n; i++) {
        snprintf(p, sizeof p, "%s/libp%d.so", dir, i);
        if (!dlopen(p, RTLD_NOW | RTLD_LOCAL)) {
            printf("dlopen_many: stopped at %d of %d: %s\n", i, n, dlerror());
            return 1;
        }
    }
    printf("dlopen_many: ok %d objects open at once\n", n);
    return 0;
}

static int dlopen_wide(const char *path) {
    void *h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!h) { printf("dlopen_wide: failed: %s\n", dlerror()); return 1; }
    printf("dlopen_wide: ok\n");
    return 0;
}

static int sem_many(int n) {
    char name[64];
    int bad = -1, err = 0;
    for (int i = 0; i < n; i++) {
        snprintf(name, sizeof name, "/ahl_probe_%d_%d", (int)getpid(), i);
        sem_t *s = sem_open(name, O_CREAT | O_EXCL, 0600, 0);
        if (s == SEM_FAILED && bad < 0) { bad = i; err = errno; }
    }
    for (int i = 0; i < n; i++) {
        snprintf(name, sizeof name, "/ahl_probe_%d_%d", (int)getpid(), i);
        sem_unlink(name);
    }
    if (bad >= 0) printf("sem_many: stopped at %d of %d: %s\n", bad, n, strerror(err));
    else printf("sem_many: ok %d named semaphores open at once\n", n);
    return bad >= 0;
}

static int severity_many(int n) {
    for (int i = 0; i < n; i++) {
        if (addseverity(100 + i, "SEV") != MM_OK) {
            printf("severity_many: addseverity failed at %d of %d\n", i, n);
            return 1;
        }
    }
    printf("severity_many: ok %d severities added\n", n);
    return 0;
}

static int gai_many(int n) {
    struct addrinfo hints = { .ai_flags = AI_NUMERICHOST | AI_NUMERICSERV, .ai_family = AF_INET };
    struct gaicb *cbs = calloc(n, sizeof *cbs), **list = calloc(n, sizeof *list);
    for (int i = 0; i < n; i++) {
        cbs[i].ar_name = "127.0.0.1"; cbs[i].ar_service = "80"; cbs[i].ar_request = &hints;
        list[i] = &cbs[i];
    }
    int r = getaddrinfo_a(GAI_WAIT, list, n, NULL);
    int ok = 0, again = 0, other = 0;
    for (int i = 0; i < n; i++) {
        int e = gai_error(&cbs[i]);
        if (e == 0) ok++; else if (e == EAI_AGAIN) again++; else other++;
    }
    printf("gai_many: getaddrinfo_a(%d requests) = %d (%s); ok %d, EAI_AGAIN %d, other %d\n", n, r,
           r ? gai_strerror(r) : "success", ok, again, other);
    return r != 0;
}

static int locale_many(int n) {
    char name[32];
    for (int i = 0; i < n; i++) {
        snprintf(name, sizeof name, "l%d", i);
        if (!newlocale(LC_ALL_MASK, name, (locale_t)0)) {
            printf("locale_many: newlocale stopped at %d of %d: %s\n", i, n, strerror(errno));
            return 1;
        }
    }
    printf("locale_many: ok %d distinct locales held at once\n", n);
    return 0;
}

static int pr_arginfo(const struct printf_info *info, size_t n, int *types, int *size) {
    (void)info; (void)n; (void)types; (void)size; return 0;
}
static void pr_va(void *mem, va_list *ap) { (void)mem; (void)ap; }

static int printf_registry(int nmod, int ntype) {
    wchar_t w[8];
    int i;
    for (i = 0; i < nmod; i++) {
        swprintf(w, 8, L"Q%d", i);
        if (register_printf_modifier(w) < 0) break;
    }
    int mods = i, merr = i < nmod ? errno : 0;
    for (i = 0; i < ntype; i++)
        if (register_printf_type(pr_va) < 0) break;
    printf("printf_registry: modifiers %d of %d (%s); types %d of %d (%s)\n", mods, nmod,
           merr ? strerror(merr) : "all", i, ntype, i < ntype ? strerror(errno) : "all");
    (void)pr_arginfo;
    return 0;
}

static int atexit_many(long n) {
    for (long i = 0; i < n; i++)
        if (atexit(count) != 0) { printf("atexit_many: failed at %ld of %ld\n", i, n); return 1; }
    printf("atexit_many: ok %ld handlers registered\n", n);
    return 0;
}

static int fnmatch_depth(int d) {
    size_t len = (size_t)d * 3 + 2;
    char *p = malloc(len);
    char *q = p;
    for (int i = 0; i < d; i++) { *q++ = '@'; *q++ = '('; }
    *q++ = 'a';
    for (int i = 0; i < d; i++) *q++ = ')';
    *q = 0;
    int r = fnmatch(p, "a", FNM_EXTMATCH);
    printf("fnmatch_depth: %d nested @() groups: %s\n", d, r == 0 ? "match" : r == FNM_NOMATCH ? "no match" : "error");
    free(p);
    return 0;
}

static int wordexp_depth(int d) {
    size_t len = (size_t)d * 6 + 2;
    char *p = malloc(len);
    char *q = p;
    for (int i = 0; i < d; i++) { memcpy(q, "${x:-", 5); q += 5; }
    *q++ = 'a';
    for (int i = 0; i < d; i++) *q++ = '}';
    *q = 0;
    wordexp_t w;
    unsetenv("x");
    int r = wordexp(p, &w, WRDE_NOCMD);
    printf("wordexp_depth: %d nested ${x:-...}: rc %d%s%s\n", d, r, r == 0 ? " word " : "",
           r == 0 && w.we_wordc ? w.we_wordv[0] : "");
    if (r == 0) wordfree(&w);
    free(p);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    const char *t = argv[1];
    int n = argc > 2 ? atoi(argv[2]) : 0;
    if (!strcmp(t, "dlopen_many")) return dlopen_many(argv[3], n);
    if (!strcmp(t, "dlopen_wide")) return dlopen_wide(argv[2]);
    if (!strcmp(t, "sem_many")) return sem_many(n);
    if (!strcmp(t, "severity_many")) return severity_many(n);
    if (!strcmp(t, "gai_many")) return gai_many(n);
    if (!strcmp(t, "locale_many")) return locale_many(n);
    if (!strcmp(t, "printf_registry")) return printf_registry(n, atoi(argv[3]));
    if (!strcmp(t, "atexit_many")) { int r = atexit_many(atol(argv[2])); return r; }
    if (!strcmp(t, "fnmatch_depth")) return fnmatch_depth(n);
    if (!strcmp(t, "wordexp_depth")) return wordexp_depth(n);
    return 2;
}
