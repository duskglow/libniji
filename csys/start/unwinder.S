/* Class machine: the program's own unwinder, offered to the C library (niji-unwinder.o, linked by
 * niji-gcc and niji-g++ into every dynamic program; D-22.24, NIJI-GXX-UNWINDER).
 *
 * A program linked with the compiler's unwinder library (libgcc_eh, as niji-gcc links it) carries
 * `_Unwind_ForcedUnwind` and `_Unwind_GetCFA` hidden: no lookup can see them, and its C++
 * personality routines and landing pads work only with that copy. `__niji_program_unwinder`
 * (exported by the wrapper's --export-dynamic-symbol) returns the address of one of them as the
 * program links it: `_Unwind_ForcedUnwind` for 0, `_Unwind_GetCFA` for anything else, or 0 when
 * the program links no unwinder. The references are weak and hidden, so a program that does not
 * take the unwinder anyway (a C program without unwind cleanups) links none because of this file
 * and gets 0 without a dynamic relocation. The C library calls it only when a thread's exit finds
 * frames to unwind and no unwinder through the loader's lookup (crate::cancel), never at start-up. */
	.section .text.__niji_program_unwinder,"ax",@progbits
	.globl __niji_program_unwinder
	.type __niji_program_unwinder, @function
	.p2align 4
__niji_program_unwinder:
	endbr64
	testl %edi, %edi
	jnz 1f
	movq _Unwind_ForcedUnwind@GOTPCREL(%rip), %rax
	ret
1:	movq _Unwind_GetCFA@GOTPCREL(%rip), %rax
	ret
	.size __niji_program_unwinder, .-__niji_program_unwinder

	.weak _Unwind_ForcedUnwind
	.hidden _Unwind_ForcedUnwind
	.weak _Unwind_GetCFA
	.hidden _Unwind_GetCFA

/* The x86 feature and ISA properties (notes.inc): a link whose every input has them keeps them. */
#include "notes.inc"
	NIJI_NOTE_PROPERTY

	.section .note.GNU-stack,"",@progbits
