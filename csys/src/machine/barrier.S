/* Class machine: explicit-zeroing compiler barrier (owner decision OD-C1). An out-of-line call the
 * optimiser cannot see into (not even under LTO: it is assembly), taking the buffer address, so
 * stores to the buffer before the call cannot be removed. Used by niji_secure_zero (D-X.11),
 * the function behind explicit_bzero and the heap's zeroing of secure blocks. */
	.section .text.niji_machine_barrier,"ax",@progbits
	.globl niji_machine_barrier
	.type niji_machine_barrier, @function
	.p2align 6
niji_machine_barrier:
	ret
	.size niji_machine_barrier, .-niji_machine_barrier
	.section .note.GNU-stack,"",@progbits
