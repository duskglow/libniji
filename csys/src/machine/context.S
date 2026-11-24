/* Class machine: user-context switch (spec §21.2; D-21.2, D-21.3, D-21.5). Status: approved.
 *
 * ucontext_t offsets (spec/abi/layouts.tsv): uc_link 8, gregs at 40 (R8 0, R9 1, R12 4,
 * R13 5, R14 6, R15 7, RDI 8, RSI 9, RBP 10, RBX 11, RDX 12, RAX 13, RCX 14, RSP 15, RIP 16,
 * eight bytes each), fpregs (a pointer) at 224, uc_sigmask at 296, __fpregs_mem at 424 (its
 * mxcsr at 448), __ssp at 936. As glibc, getcontext records the x87 environment and MXCSR (not
 * the registers' contents, which the ABI does not preserve across calls) and points fpregs at
 * __fpregs_mem.
 *
 * Shadow stack (D-21.5, D-21.7). __ssp[0] is the raw shadow-stack pointer (SSP) of the capturing
 * call: the context resumes at __ssp[0] + 8, the SSP that call returns to. It is 0 when the
 * thread had no shadow stack (rdssp is a no-op then, so the zeroed register stays 0); with one,
 * getcontext and swapcontext also set uc_flags to 0 (a user context; without a shadow stack they
 * leave it). A context suspended on another shadow stack has a restore token at __ssp[0] (the
 * value (__ssp[0] + 8) | 1): swapcontext leaves one when it leaves a stack (saveprevssp), and
 * makecontext's stack gets one from the kernel (map_shadow_stack with SHADOW_STACK_SET_TOKEN).
 * Resuming such a context switches to its stack (rstorssp, then saveprevssp); resuming one
 * without a token, captured on the current shadow stack, pops this stack up to it (incssp), as
 * longjmp does. A context with UC_SIGCONTEXT_SS (2) in uc_flags is the kernel's signal frame,
 * whose __ssp is not the library's: setcontext pops the current shadow stack to the innermost
 * signal frame's interrupted SSP instead (D-21.7). Every resume jumps to RIP (a `ret` would have
 * no matching shadow-stack entry). Without a shadow stack none of this runs.
 *
 * setcontext and swapcontext never block the library's internal signal (32, D-20.1, OD-8):
 * they install a copy of the context's mask with its bit cleared.
 *
 * makecontext's argument placement is Rust's (niji_boundary::context); it resumes the new
 * context at niji_machine_start_context with the function in %r12, which `call`s it, so that
 * the function's return has its shadow-stack entry. The glibc names exist only in the products
 * (NIJI_EXPORT). */
	.text

/* SSP as the caller sees it once this call returns, into %rax (0 without a shadow stack). */
	.macro NIJI_SSP_AFTER_RETURN
	xorl %eax, %eax
	rdsspq %rax
	testq %rax, %rax
	jz 77f
	addq $8, %rax
77:
	.endm

/* int niji_machine_getcontext(ucontext_t *uc): returns 0. */
	.globl niji_machine_getcontext
	.hidden niji_machine_getcontext
	.type niji_machine_getcontext, @function
	.p2align 6
niji_machine_getcontext:
	movq %rbx, 0x80(%rdi)
	movq %rbp, 0x78(%rdi)
	movq %r12, 0x48(%rdi)
	movq %r13, 0x50(%rdi)
	movq %r14, 0x58(%rdi)
	movq %r15, 0x60(%rdi)
	movq %rdi, 0x68(%rdi)
	movq %rsi, 0x70(%rdi)
	movq %rdx, 0x88(%rdi)
	movq %rcx, 0x98(%rdi)
	movq %r8, 0x28(%rdi)
	movq %r9, 0x30(%rdi)
	movq (%rsp), %rcx
	movq %rcx, 0xa8(%rdi)		/* RIP: the return address */
	leaq 8(%rsp), %rcx
	movq %rcx, 0xa0(%rdi)		/* RSP: the caller's, once this returns */
	xorl %eax, %eax
	rdsspq %rax
	movq %rax, 936(%rdi)		/* __ssp[0]: the raw SSP (D-21.7) */
	testq %rax, %rax
	jz 9f
	movq $0, (%rdi)			/* uc_flags: a user context (D-21.7) */
9:	leaq 0x1a8(%rdi), %rcx
	movq %rcx, 0xe0(%rdi)		/* fpregs = &__fpregs_mem */
	fnstenv (%rcx)
	fldenv (%rcx)			/* fnstenv masks the x87 exceptions: put them back */
	stmxcsr 0x1c0(%rdi)
	leaq 0x128(%rdi), %rdx		/* rt_sigprocmask(SIG_BLOCK, NULL, &uc->uc_sigmask, 8) */
	xorl %esi, %esi
	xorl %edi, %edi
	movl $8, %r10d
	movl $14, %eax
	syscall
	xorl %eax, %eax
	ret
	.size niji_machine_getcontext, .-niji_machine_getcontext

/* int niji_machine_setcontext(const ucontext_t *uc): resumes uc (its mask, x87 environment,
 * MXCSR, shadow stack and registers). Does not return. */
	.globl niji_machine_setcontext
	.hidden niji_machine_setcontext
	.type niji_machine_setcontext, @function
	.p2align 6
niji_machine_setcontext:
	pushq %rdi
	movq 0x128(%rdi), %rax		/* a copy of the mask without the library's internal */
	btrq $31, %rax			/* signal 32, which is never blocked (D-20.1, OD-8) */
	pushq %rax
	movq %rsp, %rsi			/* rt_sigprocmask(SIG_SETMASK, &copy, NULL, 8) */
	xorl %edx, %edx
	movl $2, %edi
	movl $8, %r10d
	movl $14, %eax
	syscall
	popq %rax
	popq %rdx
	jmp 1f
/* The register half of setcontext and swapcontext: uc in %rdx; the shadow stack's top (if
 * any) is the entry of the call to setcontext or swapcontext. */
1:	xorl %eax, %eax
	rdsspq %rax
	testq %rax, %rax
	jnz 3f
2:	movq 0xe0(%rdx), %rcx
	fldenv (%rcx)
	ldmxcsr 0x1c0(%rdx)
	movq 0xa0(%rdx), %rsp
	movq 0x80(%rdx), %rbx
	movq 0x78(%rdx), %rbp
	movq 0x48(%rdx), %r12
	movq 0x50(%rdx), %r13
	movq 0x58(%rdx), %r14
	movq 0x60(%rdx), %r15
	movq 0xa8(%rdx), %r11		/* resume at RIP, by a jump */
	movq 0x68(%rdx), %rdi
	movq 0x70(%rdx), %rsi
	movq 0x98(%rdx), %rcx
	movq 0x28(%rdx), %r8
	movq 0x30(%rdx), %r9
	movq 0x88(%rdx), %rdx
	xorl %eax, %eax
	jmpq *%r11
/* With a shadow stack (SSP in %rax): switch to the context's, or pop this one up to it. */
3:	testb $2, (%rdx)		/* uc_flags UC_SIGCONTEXT_SS: a kernel signal frame */
	jnz 7f
	movq 936(%rdx), %rcx		/* the context's raw SSP; 0: none recorded, leave it */
	testq %rcx, %rcx
	jz 2b
	addq $8, %rcx			/* resume above its capturing call's entry */
	leaq 1(%rcx), %rsi
	cmpq %rsi, -8(%rcx)		/* a restore token for it: another shadow stack */
	jne 4f
	movl $1, %esi			/* drop this call's entry: the stack left behind then */
	incsspq %rsi			/* resumes one entry up, where swapcontext recorded it */
	rstorssp -8(%rcx)
	saveprevssp			/* and gets its restore token below that */
	jmp 2b
4:	subq %rax, %rcx			/* the same stack: pop to it if it is above */
	jbe 2b
	shrq $3, %rcx
5:	cmpq $255, %rcx
	jbe 6f
	movl $255, %esi
	incsspq %rsi
	subq $255, %rcx
	jmp 5b
6:	incsspq %rcx
	jmp 2b
/* A kernel signal frame (D-21.7): the kernel pushed the interrupted SSP with bit 63 set just
 * below that SSP. Find the first such entry above this call's (no return address or restore
 * token has bit 63 set) and pop to the interrupted SSP. */
7:	movq %rax, %rcx
8:	movq (%rcx), %rsi
	addq $8, %rcx
	movq %rcx, %r8
	btsq $63, %r8
	cmpq %r8, %rsi
	jne 8b
	jmp 4b
	.size niji_machine_setcontext, .-niji_machine_setcontext

/* int niji_machine_swapcontext(ucontext_t *save, const ucontext_t *to): saves the caller's
 * context into save (it resumes as this call returning 0) and resumes to, switching the
 * signal mask in one system call. */
	.globl niji_machine_swapcontext
	.hidden niji_machine_swapcontext
	.type niji_machine_swapcontext, @function
	.p2align 6
niji_machine_swapcontext:
	movq %rbx, 0x80(%rdi)
	movq %rbp, 0x78(%rdi)
	movq %r12, 0x48(%rdi)
	movq %r13, 0x50(%rdi)
	movq %r14, 0x58(%rdi)
	movq %r15, 0x60(%rdi)
	movq %rdi, 0x68(%rdi)
	movq %rsi, 0x70(%rdi)
	movq %rdx, 0x88(%rdi)
	movq %rcx, 0x98(%rdi)
	movq %r8, 0x28(%rdi)
	movq %r9, 0x30(%rdi)
	movq (%rsp), %rcx
	movq %rcx, 0xa8(%rdi)
	leaq 8(%rsp), %rcx
	movq %rcx, 0xa0(%rdi)
	movq $0, 0x90(%rdi)		/* RAX: the saved context returns 0 */
	xorl %eax, %eax
	rdsspq %rax
	movq %rax, 936(%rdi)		/* __ssp[0]: the raw SSP (D-21.7) */
	testq %rax, %rax
	jz 9f
	movq $0, (%rdi)			/* uc_flags: a user context (D-21.7) */
9:	leaq 0x1a8(%rdi), %rcx
	movq %rcx, 0xe0(%rdi)
	fnstenv (%rcx)
	fldenv (%rcx)
	stmxcsr 0x1c0(%rdi)
	pushq %rsi
	movq 0x128(%rsi), %rax		/* to's mask without signal 32 (as setcontext) */
	btrq $31, %rax
	pushq %rax
	movq %rsp, %rsi			/* rt_sigprocmask(SIG_SETMASK, &copy, &save->uc_sigmask, 8) */
	leaq 0x128(%rdi), %rdx
	movl $2, %edi
	movl $8, %r10d
	movl $14, %eax
	syscall
	popq %rax
	popq %rdx
	jmp 1b
	.size niji_machine_swapcontext, .-niji_machine_swapcontext

/* Where a makecontext context starts: %r12 is its function, the stack holds its arguments past
 * the sixth above this call's return address, and %rbx (callee-saved, so the function preserves
 * it) points at two words makecontext wrote: uc_link, and the runtime's thread-exit function.
 * When the function returns, resume uc_link, or end the thread (spec §21.2: "exits the thread
 * if that is NULL"). Both are reached by `call`, so a shadow stack has this frame's entry on
 * top as setcontext expects (neither returns). */
	.globl niji_machine_start_context
	.hidden niji_machine_start_context
	.type niji_machine_start_context, @function
	.p2align 6
niji_machine_start_context:
	callq *%r12
	movq %rbx, %rsp
	movq (%rsp), %rdi
	testq %rdi, %rdi
	jz 2f
	callq niji_machine_setcontext
2:	movq 8(%rsp), %rax
	andq $-16, %rsp
	callq *%rax
	hlt
	.size niji_machine_start_context, .-niji_machine_start_context

/* usize niji_machine_ssp(void): the calling thread's SSP once this returns, 0 when it has no
 * shadow stack (makecontext asks whether to map one). */
	.globl niji_machine_ssp
	.hidden niji_machine_ssp
	.type niji_machine_ssp, @function
	.p2align 4
niji_machine_ssp:
	NIJI_SSP_AFTER_RETURN
	ret
	.size niji_machine_ssp, .-niji_machine_ssp

/* long niji_machine_shstk_enter(unsigned long lock) (D-21.6): turn on the calling thread's
 * shadow stack (arch_prctl ARCH_SHSTK_ENABLE 0x5001, ARCH_SHSTK_SHSTK 1) and, when that succeeded
 * and `lock` is not 0, lock those features (ARCH_SHSTK_LOCK 0x5003, mask `lock`). Returns the
 * enable's result (0 or -errno) by a jump: the new shadow stack has no entry for this call, and
 * the caller must be a frame that never returns (the static start). ld.so has its own copy
 * (machine/entry.S, niji_machine_ld_shstk_enter). */
	.section .text.niji_machine_shstk_enter,"ax",@progbits
	.globl niji_machine_shstk_enter
	.hidden niji_machine_shstk_enter
	.type niji_machine_shstk_enter, @function
	.p2align 4
niji_machine_shstk_enter:
	movq %rdi, %rdx
	movl $0x5001, %edi
	movl $1, %esi
	movl $158, %eax			/* SYS_arch_prctl */
	syscall
	testq %rax, %rax
	jnz 1f
	testq %rdx, %rdx
	jz 1f
	movl $0x5003, %edi
	movq %rdx, %rsi
	movl $158, %eax
	syscall
	xorl %eax, %eax
1:	popq %r11
	jmpq *%r11
	.size niji_machine_shstk_enter, .-niji_machine_shstk_enter
	.text

#ifdef NIJI_EXPORT
	.globl getcontext
	.type getcontext, @function
	.p2align 6
getcontext:
	jmp niji_machine_getcontext
	.size getcontext, .-getcontext

	.globl setcontext
	.type setcontext, @function
	.p2align 6
setcontext:
	jmp niji_machine_setcontext
	.size setcontext, .-setcontext

	.globl swapcontext
	.type swapcontext, @function
	.p2align 6
swapcontext:
	jmp niji_machine_swapcontext
	.size swapcontext, .-swapcontext
#endif
	.section .note.GNU-stack,"",@progbits
