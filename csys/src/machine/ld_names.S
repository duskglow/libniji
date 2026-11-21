/* Class machine: the interpreter's names for tools (VALGRIND-LDSO-SYMBOLS, GitHub issue #13).
 *
 * Valgrind (3.24, amd64) refuses to start a program unless it finds functions named `strcmp` and
 * `strlen` among the symbols of the object whose soname is ld-linux-x86-64.so.2: its message reads
 * "A must-be-redirected function whose name matches the pattern: strcmp in an object with soname
 * matching: ld-linux-x86-64.so.2 was not found whilst processing symbols from the object", and it
 * names glibc's debug-information package as the fix. It reads the loader's symbol table or, for
 * a stripped loader, the detached debug file found by build-id (libc6-dbg). Valgrind then replaces
 * those functions with its own, which have the C semantics of the names.
 *
 * `strlen` is the interpreter's own niji_machine_strlen: `cargo xtask so` defines it as a local
 * alias at that address when it links ld.so (--defsym; the version script keeps every symbol but
 * the loader's exports local). The interpreter compares no NUL-terminated strings, so this file
 * holds a `strcmp` of its own for the tools to find: C17 7.24.4.2, the bytes compared as unsigned
 * char, the sign of the first difference returned. Nothing in ld.so calls it.
 *
 * Local symbols only (no .globl): nothing links against them, ld.so exports nothing new
 * (spec/abi lists and tools/deb/check-symbols.sh unchanged), and the stripped loader drops them
 * with the rest of its symbol table; libc6-dbg's debug file keeps them, as the
 * check in tools/deb/check-system.sh requires. `R` (SHF_GNU_RETAIN) keeps the unreferenced
 * section through the link's --gc-sections. `cargo xtask so` assembles this file alone, after the
 * interpreter's archive in its link (its text comes last, so no other function moves);
 * crates/boundary/build.rs leaves it out of the C layer's archive, so neither libc.so.6 nor libc.a
 * has it. */
	.section .text.niji_ld_strcmp,"axR",@progbits
	.type strcmp, @function
	.p2align 4
strcmp:
	.cfi_startproc
1:	movzbl (%rdi), %eax
	movzbl (%rsi), %ecx
	subl %ecx, %eax
	jnz 2f
	testl %ecx, %ecx
	jz 2f
	incq %rdi
	incq %rsi
	jmp 1b
2:	ret
	.cfi_endproc
	.size strcmp, .-strcmp

	.section .note.GNU-stack,"",@progbits
