/* lib.c - the objects of the exit-order probe (probe.c), one per -D: NAME is the object's tag; with
 * -DUNIQ the object also defines the STB_GNU_UNIQUE object `u_<NAME>`; -DGDEF defines `g_val` and
 * -DGUSE refers to it (bound at load through the global scope). Constructor and destructor
 * report with write(2), so the record does not depend on stdio buffering at exit. */
#include <unistd.h>

#define S2(x) #x
#define S(x) S2(x)
#define CAT2(a, b) a##b
#define CAT(a, b) CAT2(a, b)

#ifdef UNIQ
#define U S(CAT(u_, NAME))
__asm__(".globl " U "\n .section .data." U ",\"aw\"\n .type " U ", @gnu_unique_object\n .size " U
        ", 4\n" U ": .long 5\n .text\n");
#endif

#ifdef GDEF
int g_val = 42; /* found by GUSE objects through the global scope */
#endif
#ifdef GUSE
extern int g_val; /* no DT_NEEDED on the definer */
int use_g(void) { return g_val; }
#endif

static const char c_msg[] = "ctor " S(NAME) "\n";
static const char d_msg[] = "dtor " S(NAME) "\n";
__attribute__((constructor)) static void c(void) { (void)!write(1, c_msg, sizeof c_msg - 1); }
__attribute__((destructor)) static void d(void) { (void)!write(1, d_msg, sizeof d_msg - 1); }
int tag_fn(void) { return 1; }
