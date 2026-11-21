/* The `.init`/`.fini` functions of a plain compiler-driver link (crates/core/tests/crt_products.rs,
 * SL-CRT). This object adds code to the middle of `_init` and `_fini`, between crti.o's prologue and
 * crtn.o's epilogue, as the ELF gABI allows any object to: each addition records %rsp where it
 * calls (the x86-64 psABI wants it 16-byte aligned at a call) and calls a C function that prints
 * it; the addition must then fall through to the epilogue and return.
 * Built three ways: a program (hello, world); with -DCRT_USE_LIB a program that also needs the
 * shared object; with -DCRT_LIB that shared object, whose own `_init`/`_fini` the loader runs. */
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#ifdef CRT_LIB
#define PFX "lib"
#else
#define PFX "prog"
#endif

static long init_sp __asm__(PFX "_init_sp") __attribute__((used)) = -1;
static long fini_sp __asm__(PFX "_fini_sp") __attribute__((used)) = -1;
static void init_hook(void) __asm__(PFX "_init_hook") __attribute__((used));
static void fini_hook(void) __asm__(PFX "_fini_hook") __attribute__((used));

__asm__(".section .init,\"ax\",@progbits\n"
        "\tmovq %rsp, " PFX "_init_sp(%rip)\n"
        "\tcall " PFX "_init_hook\n"
        "\t.section .fini,\"ax\",@progbits\n"
        "\tmovq %rsp, " PFX "_fini_sp(%rip)\n"
        "\tcall " PFX "_fini_hook\n"
        "\t.text\n");

/* "<prefix> <what> sp%16=<n>\n" with write(2): .fini runs after stdio's last flush. */
static void say(const char *what, long sp) {
    char buf[64];
    size_t n = 0;
    const char *parts[] = {PFX, " ", what, " sp%16="};
    for (unsigned i = 0; i < sizeof parts / sizeof parts[0]; i++) {
        memcpy(buf + n, parts[i], strlen(parts[i]));
        n += strlen(parts[i]);
    }
    long r = sp & 15;
    if (r >= 10) buf[n++] = (char)('0' + r / 10);
    buf[n++] = (char)('0' + r % 10);
    buf[n++] = '\n';
    (void)!write(1, buf, n);
}

static void init_hook(void) { say("init", init_sp); }
static void fini_hook(void) { say("fini", fini_sp); }

#ifdef CRT_LIB
int crt_lib_value(void) { return 42; }
#else
#ifdef CRT_USE_LIB
int crt_lib_value(void);
#endif
int main(void) {
    printf("hello, world\n");
#ifdef CRT_USE_LIB
    printf("lib says %d\n", crt_lib_value());
#endif
    fflush(stdout);
    return 0;
}
#endif
