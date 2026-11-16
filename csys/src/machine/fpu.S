/* Class machine: MXCSR and x87 control/status/environment access (owner decision OD-C1).
 * Values pass in registers; memory operands use the red zone below %rsp (SysV ABI). */
	.section .text.niji_machine_mxcsr_get,"ax",@progbits
	.globl niji_machine_mxcsr_get
	.type niji_machine_mxcsr_get, @function
	.p2align 6
niji_machine_mxcsr_get:
	stmxcsr -4(%rsp)
	movl -4(%rsp), %eax
	ret
	.size niji_machine_mxcsr_get, .-niji_machine_mxcsr_get
	.section .text.niji_machine_mxcsr_set,"ax",@progbits
	.globl niji_machine_mxcsr_set
	.type niji_machine_mxcsr_set, @function
	.p2align 6
niji_machine_mxcsr_set:
	movl %edi, -4(%rsp)
	ldmxcsr -4(%rsp)
	ret
	.size niji_machine_mxcsr_set, .-niji_machine_mxcsr_set
	.section .text.niji_machine_x87_cw_get,"ax",@progbits
	.globl niji_machine_x87_cw_get
	.type niji_machine_x87_cw_get, @function
	.p2align 6
niji_machine_x87_cw_get:
	fnstcw -2(%rsp)
	movzwl -2(%rsp), %eax
	ret
	.size niji_machine_x87_cw_get, .-niji_machine_x87_cw_get
	.section .text.niji_machine_x87_cw_set,"ax",@progbits
	.globl niji_machine_x87_cw_set
	.type niji_machine_x87_cw_set, @function
	.p2align 6
niji_machine_x87_cw_set:
	movw %di, -2(%rsp)
	fldcw -2(%rsp)
	ret
	.size niji_machine_x87_cw_set, .-niji_machine_x87_cw_set
	.section .text.niji_machine_x87_sw_get,"ax",@progbits
	.globl niji_machine_x87_sw_get
	.type niji_machine_x87_sw_get, @function
	.p2align 6
niji_machine_x87_sw_get:
	fnstsw %ax
	movzwl %ax, %eax
	ret
	.size niji_machine_x87_sw_get, .-niji_machine_x87_sw_get
	.section .text.niji_machine_x87_clex,"ax",@progbits
	.globl niji_machine_x87_clex
	.type niji_machine_x87_clex, @function
	.p2align 6
niji_machine_x87_clex:
	fnclex
	ret
	.size niji_machine_x87_clex, .-niji_machine_x87_clex
	.section .text.niji_machine_x87_env_store,"ax",@progbits
	.globl niji_machine_x87_env_store
	.type niji_machine_x87_env_store, @function
	.p2align 6
niji_machine_x87_env_store:
	fnstenv (%rdi)
	fldenv (%rdi)
	ret
	.size niji_machine_x87_env_store, .-niji_machine_x87_env_store
	.section .text.niji_machine_x87_env_load,"ax",@progbits
	.globl niji_machine_x87_env_load
	.type niji_machine_x87_env_load, @function
	.p2align 6
niji_machine_x87_env_load:
	fldenv (%rdi)
	ret
	.size niji_machine_x87_env_load, .-niji_machine_x87_env_load
	.section .text.niji_machine_mxcsr_mask,"ax",@progbits
	.globl niji_machine_mxcsr_mask
	.type niji_machine_mxcsr_mask, @function
	.p2align 6
niji_machine_mxcsr_mask:
	subq $520, %rsp
	movq %rsp, %rax
	andq $-16, %rax
	fxsave (%rax)
	movl 28(%rax), %eax
	addq $520, %rsp
	ret
	.size niji_machine_mxcsr_mask, .-niji_machine_mxcsr_mask
/* The hardware square roots (M6-2): correctly rounded in the unit's current rounding direction,
 * raising their flags in that unit (IEEE 754 squareRoot; Intel SDM SQRTSS/SQRTSD/FSQRT). */
	.section .text.niji_machine_sqrtsd,"ax",@progbits
	.globl niji_machine_sqrtsd
	.type niji_machine_sqrtsd, @function
	.p2align 4
niji_machine_sqrtsd:
	sqrtsd %xmm0, %xmm0
	ret
	.size niji_machine_sqrtsd, .-niji_machine_sqrtsd
	.section .text.niji_machine_fsqrt,"ax",@progbits
	.globl niji_machine_fsqrt
	.type niji_machine_fsqrt, @function
	.p2align 4
niji_machine_fsqrt:
	fldt (%rdi)
	fsqrt
	fstpt (%rsi)
	ret
	.size niji_machine_fsqrt, .-niji_machine_fsqrt
	.section .note.GNU-stack,"",@progbits
