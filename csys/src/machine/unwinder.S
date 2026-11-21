/* Class machine: the unwinder linked into a static program (D-22.21, THREAD-FORCED-UNWIND-STATIC).
 *
 * `niji_machine_unwind_force` and `niji_machine_unwind_get_cfa` return the addresses of
 * `_Unwind_ForcedUnwind` and `_Unwind_GetCFA` (Itanium C++ ABI §1.3) as the program links them,
 * or 0. Assembled plainly they return 0: the C layer builds this form for everything but the C
 * library product (ld.so, the test builds), and `cargo xtask so` links it into libc.so.6, where a
 * dynamically linked process finds the unwinder with the loader's `dlsym` (D-22.19) and this form
 * leaves no symbol, relocation or start-up lookup behind. For libc.a, `cargo xtask so` assembles
 * the file with -DNIJI_STATIC_ARCHIVE and adds it as a member of its own: the references are
 * then weak, so a static link that takes the unwinder anyway (libgcc_eh's, pulled
 * by a C++ runtime or by `-fexceptions` cleanups) resolves them, and one that does not (a C
 * program without unwind cleanups) gets 0 and links no unwinder because of them.
 * Called only by the exit of a thread (`crate::cancel`), never at start-up. Plain `.text.<name>`
 * sections: the link editor places `.text.unlikely.*` ahead of the hot group (TEXT-PREFIX). */
	.section .text.niji_machine_unwind_force,"ax",@progbits
	.globl niji_machine_unwind_force
	.hidden niji_machine_unwind_force
	.type niji_machine_unwind_force, @function
	.p2align 4
niji_machine_unwind_force:
#ifdef NIJI_STATIC_ARCHIVE
	movq _Unwind_ForcedUnwind@GOTPCREL(%rip), %rax
#else
	xorl %eax, %eax
#endif
	ret
	.size niji_machine_unwind_force, .-niji_machine_unwind_force

	.section .text.niji_machine_unwind_get_cfa,"ax",@progbits
	.globl niji_machine_unwind_get_cfa
	.hidden niji_machine_unwind_get_cfa
	.type niji_machine_unwind_get_cfa, @function
	.p2align 4
niji_machine_unwind_get_cfa:
#ifdef NIJI_STATIC_ARCHIVE
	movq _Unwind_GetCFA@GOTPCREL(%rip), %rax
#else
	xorl %eax, %eax
#endif
	ret
	.size niji_machine_unwind_get_cfa, .-niji_machine_unwind_get_cfa

#ifdef NIJI_STATIC_ARCHIVE
	.weak _Unwind_ForcedUnwind
	.weak _Unwind_GetCFA
#endif
	.section .note.GNU-stack,"",@progbits
