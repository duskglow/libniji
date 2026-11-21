/* Class machine: CPUID and XGETBV (owner decision OD-C1).
 * niji_machine_cpuid(uint32_t leaf, uint32_t subleaf, uint32_t out[4]): out = eax, ebx, ecx, edx.
 * niji_machine_xgetbv0(void): XCR0 (callers check CPUID.1:ECX.OSXSAVE first).
 * One section per function (HOT-ORDER): startup runs both, so the hot-text list can place them
 * in the startup group (the build's traced hot-text.list). */
	.section .text.niji_machine_cpuid,"ax",@progbits
	.globl niji_machine_cpuid
	.type niji_machine_cpuid, @function
	.p2align 6
niji_machine_cpuid:
	pushq %rbx
	movl %edi, %eax
	movl %esi, %ecx
	movq %rdx, %r8
	cpuid
	movl %eax, (%r8)
	movl %ebx, 4(%r8)
	movl %ecx, 8(%r8)
	movl %edx, 12(%r8)
	popq %rbx
	ret
	.size niji_machine_cpuid, .-niji_machine_cpuid
	.section .text.niji_machine_xgetbv0,"ax",@progbits
	.globl niji_machine_xgetbv0
	.type niji_machine_xgetbv0, @function
	.p2align 6
niji_machine_xgetbv0:
	xorl %ecx, %ecx
	xgetbv
	shlq $32, %rdx
	orq %rdx, %rax
	ret
	.size niji_machine_xgetbv0, .-niji_machine_xgetbv0
	.section .note.GNU-stack,"",@progbits
