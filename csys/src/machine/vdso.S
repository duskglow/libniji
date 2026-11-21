/* Class machine: call a vDSO function by address (spec Appendix A.2 vDSO).
 * niji_machine_vcall2(fn, a, b) and niji_machine_vcall3(fn, a, b, c) tail-call fn. */
	.text
	.section .text.niji_machine_vcall2,"ax",@progbits
	.globl niji_machine_vcall2
	.type niji_machine_vcall2, @function
	.p2align 6
niji_machine_vcall2:
	movq %rdi, %rax
	movq %rsi, %rdi
	movq %rdx, %rsi
	jmp *%rax
	.size niji_machine_vcall2, .-niji_machine_vcall2
	.text
	.text
	.section .text.niji_machine_vcall3,"ax",@progbits
	.globl niji_machine_vcall3
	.type niji_machine_vcall3, @function
	.p2align 6
niji_machine_vcall3:
	movq %rdi, %rax
	movq %rsi, %rdi
	movq %rdx, %rsi
	movq %rcx, %rdx
	jmp *%rax
	.size niji_machine_vcall3, .-niji_machine_vcall3
	.text
	.section .note.GNU-stack,"",@progbits
