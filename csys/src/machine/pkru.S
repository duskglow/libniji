/* Class machine: PKRU read and write (owner decision OD-C1). Callers check CPUID.7:ECX.OSPKE. */
	.section .text.niji_machine_rdpkru,"ax",@progbits
	.globl niji_machine_rdpkru
	.type niji_machine_rdpkru, @function
	.p2align 6
niji_machine_rdpkru:
	xorl %ecx, %ecx
	rdpkru
	ret
	.size niji_machine_rdpkru, .-niji_machine_rdpkru
	.section .text.niji_machine_wrpkru,"ax",@progbits
	.globl niji_machine_wrpkru
	.type niji_machine_wrpkru, @function
	.p2align 6
niji_machine_wrpkru:
	movl %edi, %eax
	xorl %ecx, %ecx
	xorl %edx, %edx
	wrpkru
	ret
	.size niji_machine_wrpkru, .-niji_machine_wrpkru
	.section .note.GNU-stack,"",@progbits
