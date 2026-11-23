/* crt_products.rs `plain_cc_runs_the_program_arrays_in_the_reference_order` (LIBC-INIT-ARRAY-REFS):
 * a program with an entry in each of its start and exit hooks (ELF gABI "Initialization and
 * Termination Functions"; x86-64 psABI `.init`/`.fini`): `.preinit_array`, `.init` (through crti.o
 * and crtn.o), `.init_array` with a priority, `.fini_array` with a priority, `.fini`, and `atexit`
 * from a constructor and from main. Each prints one line with write(2), so the order is the order
 * of the calls and no stream buffering takes part. Built and run the same way against libniji and
 * against the reference; the outputs must match. */
#include <stdlib.h>
#include <unistd.h>

static void say(const char *s) {
    size_t n = 0;
    while (s[n] != 0) {
        n++;
    }
    if (write(1, s, n) != (ssize_t)n) {
        _exit(3);
    }
}

static void at_exit_main(void) { say("atexit main\n"); }
static void at_exit_ctor(void) { say("atexit ctor\n"); }

/* The program's array entries get argc, argv and envp (run with one argument and an empty environment). */
static void pre(int argc, char **argv, char **envp) {
    say(argc == 1 && argv != 0 && argv[0] != 0 && argv[1] == 0 && envp != 0 && envp[0] == 0 ? "preinit args\n" : "preinit other args\n");
}
__attribute__((section(".preinit_array"), used)) static void (*const pre_entry)(int, char **, char **) = pre;

__attribute__((constructor(101))) static void ctor_101(int argc, char **argv, char **envp) {
    say(argc == 1 && argv != 0 && argv[0] != 0 && envp != 0 && envp[0] == 0 ? "ctor 101 args\n" : "ctor 101 other args\n");
    atexit(at_exit_ctor);
}
__attribute__((constructor)) static void ctor(void) { say("ctor\n"); }
__attribute__((destructor(101))) static void dtor_101(void) { say("dtor 101\n"); }
__attribute__((destructor)) static void dtor(void) { say("dtor\n"); }

void crt_arrays_init_hook(void);
void crt_arrays_fini_hook(void);
void crt_arrays_init_hook(void) { say("init section\n"); }
void crt_arrays_fini_hook(void) { say("fini section\n"); }
/* crti.o's `_init` and `_fini` prologues align the stack for these calls; crtn.o returns. */
__asm__(".section .init,\"ax\",@progbits\n\tcall crt_arrays_init_hook@PLT\n"
        ".section .fini,\"ax\",@progbits\n\tcall crt_arrays_fini_hook@PLT\n"
        ".text\n");

int main(void) {
    say("main\n");
    atexit(at_exit_main);
    return 0;
}
