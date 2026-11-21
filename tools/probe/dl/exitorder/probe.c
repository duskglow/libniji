/* probe.c - black-box probe of the order in which the destructors of dlopen()ed objects still loaded
 * at exit run (story DL-EXIT-ORDER-PROBE). Usage: probe DIR OP... where each OP is `<name>:<how>` on
 * DIR/lib<name>.so, done in the order given:
 *   k  dlopen(RTLD_NOW), handle kept open to exit
 *   n  dlopen(RTLD_NOW | RTLD_NODELETE), then dlclose (the object stays: NODELETE)
 *   N  dlopen(RTLD_NOW | RTLD_NODELETE), handle kept open
 *   u  dlopen(RTLD_NOW), dlsym of its unique object u_<name>, then dlclose (pinned by the binding)
 *   x  dlopen(RTLD_NOW), then dlclose (unloaded)
 *   g  dlopen(RTLD_NOW | RTLD_GLOBAL), handle kept open
 * Each step prints `<op> loaded=<0|1>` after it; then `main returns`, and the program's own destructor
 * prints `dtor main`. Public interfaces only (dlopen(3), dlsym(3), dlclose(3)). */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static const char *dir;
static char buf[4096];

static const char *path(const char *n) {
    snprintf(buf, sizeof buf, "%s/lib%s.so", dir, n);
    return buf;
}

static void say(const char *s) { (void)!write(1, s, strlen(s)); }

/* Loaded now? (an RTLD_NOLOAD open, closed again so the count is unchanged) */
static int loaded(const char *n) {
    void *h = dlopen(path(n), RTLD_NOW | RTLD_NOLOAD);
    if (h) dlclose(h);
    return h != NULL;
}

__attribute__((destructor)) static void dmain(void) { say("dtor main\n"); }

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    dir = argv[1];
    for (int i = 2; i < argc; i++) {
        char name[64];
        const char *colon = strchr(argv[i], ':');
        if (!colon || (size_t)(colon - argv[i]) >= sizeof name) return 2;
        memcpy(name, argv[i], (size_t)(colon - argv[i]));
        name[colon - argv[i]] = 0;
        char how = colon[1];
        int flags = RTLD_NOW | ((how == 'n' || how == 'N') ? RTLD_NODELETE : 0) | (how == 'g' ? RTLD_GLOBAL : 0);
        void *h = dlopen(path(name), flags);
        if (!h) {
            printf("%s: dlopen failed: %s\n", argv[i], dlerror());
            return 1;
        }
        if (how == 'u') {
            char sym[80];
            snprintf(sym, sizeof sym, "u_%s", name);
            if (!dlsym(h, sym)) {
                printf("%s: dlsym %s failed\n", argv[i], sym);
                return 1;
            }
        }
        if (how == 'n' || how == 'u' || how == 'x') dlclose(h);
        char line[128];
        snprintf(line, sizeof line, "%s loaded=%d\n", argv[i], loaded(name));
        say(line);
    }
    say("main returns\n");
    return 0;
}
