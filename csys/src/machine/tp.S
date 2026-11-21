/* Class machine: thread-pointer read (FS base). x86_64 TLS ABI: the word at %fs:0 holds the
 * thread pointer itself (spec Appendix A.2, KERN/ABI). */
	.section .text.niji_machine_tp,"ax",@progbits
	.globl niji_machine_tp
	.hidden niji_machine_tp
	.type niji_machine_tp, @function
	.p2align 6
niji_machine_tp:
	movq %fs:0, %rax
	ret
	.size niji_machine_tp, .-niji_machine_tp

/* int *niji_machine_errno_location(void): `__errno_location` (spec §2.1, D-2.1, D-X.60). In the
 * products with the runtime this IS `__errno_location` (second label; the Rust twin in
 * crates/boundary/src/posix/s02_diagnostics.rs calls this code in the tests). The block's errno
 * word (thread pointer + 0x80, asserted in crates/rt/src/tls.rs) only in a process the runtime
 * started (`niji_machine_owned`, machine/points.S, D-X.58): elsewhere 0x80 of the block is
 * another library's word, and `niji_errno_rest` (Rust, cold) returns the thread's registry
 * word, stable for the thread's life. Why assembly: the owned test is one rip-relative `cmpb`
 * of the hidden byte here, where Rust reaches the assembler's symbol through the GOT. */
	.section .text.niji_machine_errno_location,"ax",@progbits
	.hidden niji_errno_rest	/* crates/boundary/src/posix/s02_diagnostics.rs */
	.globl niji_machine_errno_location
	.hidden niji_machine_errno_location
	.type niji_machine_errno_location, @function
	.p2align 6
	.cfi_startproc
niji_machine_errno_location:
#ifdef NIJI_EXPORT
#ifdef NIJI_RUNTIME
	.globl __errno_location
	.type __errno_location, @function
__errno_location:
#endif
#endif
	cmpb $0, niji_machine_owned(%rip)
	je niji_errno_rest
	movq %fs:0, %rax
	addq $0x80, %rax
	ret
	.cfi_endproc
	.size niji_machine_errno_location, .-niji_machine_errno_location
	.section .note.GNU-stack,"",@progbits
