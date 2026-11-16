/* Class machine: call code at an address (owner decision OD-C1): ELF constructors and entry points
 * in ld.so. niji_machine_callN(fn, args...) tail-calls fn with the arguments shifted down. */
	.section .text.niji_machine_call0,"ax",@progbits
	.globl niji_machine_call0
	.type niji_machine_call0, @function
	.p2align 6
niji_machine_call0:
	jmp *%rdi
	.size niji_machine_call0, .-niji_machine_call0
	.section .text.niji_machine_call0,"ax",@progbits
	.section .text.niji_machine_call3,"ax",@progbits
	.globl niji_machine_call3
	.type niji_machine_call3, @function
	.p2align 6
niji_machine_call3:
	movq %rdi, %rax
	movq %rsi, %rdi
	movq %rdx, %rsi
	movq %rcx, %rdx
	jmp *%rax
	.size niji_machine_call3, .-niji_machine_call3
	.section .text.niji_machine_call0,"ax",@progbits
	.section .text.niji_machine_call4,"ax",@progbits
	.globl niji_machine_call4
	.type niji_machine_call4, @function
	.p2align 6
niji_machine_call4:
	movq %rdi, %rax
	movq %rsi, %rdi
	movq %rdx, %rsi
	movq %rcx, %rdx
	movq %r8, %rcx
	jmp *%rax
	.size niji_machine_call4, .-niji_machine_call4
	.section .note.GNU-stack,"",@progbits
