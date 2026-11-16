/* Class machine: the prologue of the `.init` and `.fini` functions, the content of niji-crti.o
 * (installed as crti.o; SL-CRT, whose decision record states the start files' rules).
 *
 * ELF gABI "Initialization and Termination Functions" and the x86-64 psABI (".init"/".fini"
 * sections): the link editor concatenates every input's `.init` (and `.fini`) section into one
 * output section, and the dynamic section's DT_INIT (DT_FINI) names its start, `_init` (`_fini`)
 * by default. The compiler driver links this object right after the start object and crtn.o
 * last (GCC's startfile and endfile specs: `crti.o%s` ... `crtn.o%s`, in every link, `-shared`
 * included), so the output section is
 *
 *   _init:  this prologue | every other object's .init code, in link order | crtn.o's epilogue
 *
 * one function that runs each contribution in turn. libniji calls it (DT_INIT through ld.so for a
 * shared object and a dynamic program, the weak `_init` in a static one) with %rsp 8 below a
 * 16-byte boundary; the prologue takes 8 more bytes, so a contribution that calls a function
 * calls with the stack aligned as the psABI requires, and crtn.o gives them back and returns.
 *
 * - `_init`/`_fini` are hidden: each output gets its own, found by the link editor; none is
 *   exported, so no object's DT_INIT can bind to another object's function.
 * - endbr64 first: the functions are reached by an indirect call (a no-op without IBT).
 * - No call to `__gmon_start__`: libniji's profiling start objects (gcrt1.S) start profiling from
 *   `.preinit_array` themselves.
 * - The sections are "ax" PROGBITS: code, never writable. crtn.S's halves have alignment 1, so
 *   the link editor puts no fill between the last contribution and the epilogue.
 * Outside csys/src on purpose: crates/boundary/build.rs archives every source under csys/src,
 * and no archive may carry `.init` code. */

	.section .init,"ax",@progbits
	.p2align 4
	.globl _init
	.hidden _init
	.type _init, @function
_init:
.Lniji_machine_crti_init:
	endbr64
	subq $8, %rsp

	.section .fini,"ax",@progbits
	.p2align 4
	.globl _fini
	.hidden _fini
	.type _fini, @function
_fini:
.Lniji_machine_crti_fini:
	endbr64
	subq $8, %rsp

	.section .note.GNU-stack,"",@progbits
