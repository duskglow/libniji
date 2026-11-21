/* Class machine: pthread_once/call_once (spec §22.5; S11, OD-6 revised). Status: approved.
 *
 * int niji_machine_once(pthread_once_t *once, void (*init)(void)): the routine runs in this
 * frame, so that an exception thrown by it (a C++ `std::call_once` routine, libstdc++ builds
 * it on pthread_once) unwinds through no Rust frame. The frame's call-frame information names
 * a personality routine (niji_once_personality, boundary/src/sync.rs): during the cleanup
 * phase of an unwind it makes the object new again and wakes its waiters, so that a later call
 * runs the routine again, as glibc does; the exception continues to the caller's handler.
 * No unwinder function is called: the personality finds the object through the cancellation
 * record niji_once_begin pushed in this frame, so the library needs no libgcc_s.
 *
 * niji_once_begin(once, init, record) returns -1 when this thread now runs `init` (the record
 * at %rsp registered for cancellation), else the result (0, or EINVAL). niji_once_end(once,
 * record) completes the object.
 *
 * The glibc names exist only in the products (NIJI_EXPORT). */
	.section .data.rel.ro.niji_once,"aw",@progbits
	.p2align 3
.Lonce_personality:
	.quad niji_once_personality

	.section .text.niji_machine_once,"ax",@progbits
	.hidden niji_once_begin
	.hidden niji_once_end
	.hidden niji_once_personality
	.globl niji_machine_once
	.hidden niji_machine_once
	.type niji_machine_once, @function
	.p2align 4
niji_machine_once:
	.cfi_startproc
	.cfi_personality 0x9b, .Lonce_personality	/* pcrel | sdata4 | indirect */
	testq %rdi, %rdi
	jz 1f
	cmpl $3, (%rdi)			/* done (ONCE_DONE): no call, no store */
	jne 1f
	xorl %eax, %eax
	ret
1:	pushq %rbx
	.cfi_adjust_cfa_offset 8
	.cfi_offset %rbx, -16
	pushq %r12
	.cfi_adjust_cfa_offset 8
	.cfi_offset %r12, -24
	subq $40, %rsp			/* the 4-word record; %rsp 16-aligned at the calls */
	.cfi_adjust_cfa_offset 40
	movq %rdi, %rbx
	movq %rsi, %r12
	movq %rsp, %rdx
	call niji_once_begin
	cmpl $-1, %eax
	jne 2f
	call *%r12			/* may unwind: the personality resets the object */
	movq %rbx, %rdi
	movq %rsp, %rsi
	call niji_once_end
	xorl %eax, %eax
2:	addq $40, %rsp
	.cfi_adjust_cfa_offset -40
	popq %r12
	.cfi_adjust_cfa_offset -8
	.cfi_restore %r12
	popq %rbx
	.cfi_adjust_cfa_offset -8
	.cfi_restore %rbx
	ret
	.cfi_endproc
	.size niji_machine_once, .-niji_machine_once

#ifdef NIJI_EXPORT
/* The exported entry points: tail calls, so that the routine's frame sits directly below the
 * caller's (call_once's void result is %eax, ignored). */
	.section .text.pthread_once,"ax",@progbits
	.globl pthread_once
	.type pthread_once, @function
	.p2align 6			/* exported entries are 64-byte aligned (D-4.33) */
pthread_once:
	jmp niji_machine_once
	.size pthread_once, .-pthread_once
	.section .text.__pthread_once,"ax",@progbits

	.globl __pthread_once
	.type __pthread_once, @function
	.p2align 6			/* exported entries are 64-byte aligned (D-4.33) */
__pthread_once:
	jmp niji_machine_once
	.size __pthread_once, .-__pthread_once
	.section .text.call_once,"ax",@progbits

	.globl call_once
	.type call_once, @function
	.p2align 6			/* exported entries are 64-byte aligned (D-4.33) */
call_once:
	jmp niji_machine_once
	.size call_once, .-call_once
#endif

	.section .note.GNU-stack,"",@progbits
