/* Class machine: valgrind's RUNNING_ON_VALGRIND client request (request 0x1001, valgrind.h's documented
 * amd64 sequence, VALGRIND-LOADER-VIEW, D-NEW). %rax points at the six-word request block (the request
 * and five zero arguments), %rdx holds the default answer 0; the four rotations of %rdi (3 + 13 + 61 + 51 =
 * 128 bits, so %rdi is unchanged) followed by `xchgq %rbx,%rbx` are no-ops on the bare machine, and under
 * valgrind they ask the tool, which leaves its nesting depth (1 or more) in %rdx. The result is %rdx. No
 * system call; the only memory touched is the request block on this function's own stack. */
	.section .text.niji_machine_running_on_valgrind,"ax",@progbits
	.globl niji_machine_running_on_valgrind
	.hidden niji_machine_running_on_valgrind
	.type niji_machine_running_on_valgrind, @function
	.p2align 4
niji_machine_running_on_valgrind:
	subq $56, %rsp
	movq $0x1001, (%rsp)
	movq $0, 8(%rsp)
	movq $0, 16(%rsp)
	movq $0, 24(%rsp)
	movq $0, 32(%rsp)
	movq $0, 40(%rsp)
	movq %rsp, %rax
	xorl %edx, %edx
	rolq $3, %rdi
	rolq $13, %rdi
	rolq $61, %rdi
	rolq $51, %rdi
	xchgq %rbx, %rbx
	movq %rdx, %rax
	addq $56, %rsp
	ret
	.size niji_machine_running_on_valgrind, .-niji_machine_running_on_valgrind
	.section .note.GNU-stack,"",@progbits
