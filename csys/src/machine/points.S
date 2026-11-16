/* Class machine: the direct cancellation-point entries (CANCEL-FAST-2, D-22.11 "Direct points";
 * plan S2 hot-kernel). In the products these ARE `read`, `write`, `pread`/`pread64`,
 * `pwrite`/`pwrite64` and `close`/`__close` (second labels under NIJI_EXPORT, versions from the
 * Rust twins' `export! { asm ... }`): a call reaches the system call with no Rust hop. Test builds
 * define only the niji_machine_* names; the Rust twins (crates/boundary/src/posix/s11_lowio.rs)
 * call them, so the tests run this code.
 *
 * Why assembly: the Rust export spent ~40 instructions per call around the system call (the
 * slice check, the thread-pointer and descriptor loads, the argument shuffle into the shared
 * window, the result conversion); glibc's wrapper is ~21. Here the common case is the descriptor
 * load, three tests, the shared window (machine/thread_start.S) and one result test.
 *
 * Every entry takes the Rust path (`niji_*_rest`: the full former export, unchanged) for
 * whatever is not the common case: a process the runtime did not start (`niji_machine_owned`
 * clear, D-X.58: its thread blocks are not libniji's, so `%fs:0xa0` is never read there), no
 * descriptor yet (`%fs:0xa0` is 0: a thread the runtime did not create and has not adopted), a count of 0 or above isize::MAX, a NULL buffer (the export's
 * slice rules, EFAULT without a system call). A kernel error result (-4095..-1, which includes the
 * window's -ECANCELED) tail-jumps to `niji_cancel_point_fail` with the raw result: it acts on a
 * due request (the cancellation word, checked there) and otherwise sets errno and returns -1.
 *
 * CONTRACT: the C contracts of the exports; nothing is dereferenced here except the thread block
 * word at %fs:0xa0 (`ThreadBlock::thread`, asserted in crates/rt/src/tls.rs) and, through the
 * window, the descriptor's cancellation word at offset 32 (`Thread::cancel`, asserted in
 * crates/rt/src/thread.rs). The stack pointer never moves: the default call-frame row describes
 * every instruction, so `backtrace` from a signal handler in the window unwinds through here.
 * No .macro (the affected map reads this file for the symbols it defines and uses). */

	.section .text.niji_machine_read,"ax",@progbits
	.hidden niji_machine_cancel_window	/* machine/thread_start.S */
	.hidden niji_cancel_point_fail	/* crates/boundary/src/posix/s11_lowio.rs */
	.hidden niji_close_point_fail
	.hidden niji_read_rest
	.hidden niji_write_rest
	.hidden niji_pread_rest
	.hidden niji_pwrite_rest
	.hidden niji_close_rest
	.hidden niji_machine_aio_used

/* ssize_t niji_machine_read(int fd, void *buf, size_t n) */
	.globl niji_machine_read
	.hidden niji_machine_read
	.type niji_machine_read, @function
	.p2align 6
	.cfi_startproc
niji_machine_read:
#ifdef NIJI_EXPORT
	.globl read
	.type read, @function
read:
#endif
	cmpb $0, niji_machine_owned(%rip)
	je niji_read_rest
	movq %fs:0xa0, %r11
	testq %r11, %r11
	jz niji_read_rest
	testq %rdx, %rdx
	jle niji_read_rest
	testq %rsi, %rsi
	jz niji_read_rest
	addq $32, %r11
	xorl %eax, %eax			/* SYS_read */
	call niji_machine_cancel_window
	cmpq $-4095, %rax
	jae 1f
	ret
1:	movq %rax, %rdi
	jmp niji_cancel_point_fail
	.cfi_endproc
	.size niji_machine_read, .-niji_machine_read
	.section .text.niji_machine_write,"ax",@progbits

/* ssize_t niji_machine_write(int fd, const void *buf, size_t n) */
	.globl niji_machine_write
	.hidden niji_machine_write
	.type niji_machine_write, @function
	.p2align 6
	.cfi_startproc
niji_machine_write:
#ifdef NIJI_EXPORT
	.globl write
	.type write, @function
write:
#endif
	cmpb $0, niji_machine_owned(%rip)
	je niji_write_rest
	movq %fs:0xa0, %r11
	testq %r11, %r11
	jz niji_write_rest
	testq %rdx, %rdx
	jle niji_write_rest
	testq %rsi, %rsi
	jz niji_write_rest
	addq $32, %r11
	movl $1, %eax			/* SYS_write */
	call niji_machine_cancel_window
	cmpq $-4095, %rax
	jae 1f
	ret
1:	movq %rax, %rdi
	jmp niji_cancel_point_fail
	.cfi_endproc
	.size niji_machine_write, .-niji_machine_write
	.section .text.niji_machine_pread,"ax",@progbits

/* ssize_t niji_machine_pread(int fd, void *buf, size_t n, off_t off) */
	.globl niji_machine_pread
	.hidden niji_machine_pread
	.type niji_machine_pread, @function
	.p2align 6
	.cfi_startproc
niji_machine_pread:
#ifdef NIJI_EXPORT
	.globl pread
	.type pread, @function
pread:
	.globl pread64
	.type pread64, @function
pread64:
#endif
	cmpb $0, niji_machine_owned(%rip)
	je niji_pread_rest
	movq %fs:0xa0, %r11
	testq %r11, %r11
	jz niji_pread_rest
	testq %rdx, %rdx
	jle niji_pread_rest
	testq %rsi, %rsi
	jz niji_pread_rest
	addq $32, %r11
	movq %rcx, %r10
	movl $17, %eax			/* SYS_pread64 */
	call niji_machine_cancel_window
	cmpq $-4095, %rax
	jae 1f
	ret
1:	movq %rax, %rdi
	jmp niji_cancel_point_fail
	.cfi_endproc
	.size niji_machine_pread, .-niji_machine_pread
	.section .text.niji_machine_pwrite,"ax",@progbits

/* ssize_t niji_machine_pwrite(int fd, const void *buf, size_t n, off_t off) */
	.globl niji_machine_pwrite
	.hidden niji_machine_pwrite
	.type niji_machine_pwrite, @function
	.p2align 6
	.cfi_startproc
niji_machine_pwrite:
#ifdef NIJI_EXPORT
	.globl pwrite
	.type pwrite, @function
pwrite:
	.globl pwrite64
	.type pwrite64, @function
pwrite64:
#endif
	cmpb $0, niji_machine_owned(%rip)
	je niji_pwrite_rest
	movq %fs:0xa0, %r11
	testq %r11, %r11
	jz niji_pwrite_rest
	testq %rdx, %rdx
	jle niji_pwrite_rest
	testq %rsi, %rsi
	jz niji_pwrite_rest
	addq $32, %r11
	movq %rcx, %r10
	movl $18, %eax			/* SYS_pwrite64 */
	call niji_machine_cancel_window
	cmpq $-4095, %rax
	jae 1f
	ret
1:	movq %rax, %rdi
	jmp niji_cancel_point_fail
	.cfi_endproc
	.size niji_machine_pwrite, .-niji_machine_pwrite

/* int niji_machine_close(int fd): the descriptor is closed regardless, outside the window
 * (§22.3), then a due request is acted on (in niji_close_point_fail, which also sets errno).
 * Any asynchronous I/O ever started (niji_machine_aio_used, set by crates/boundary/src/aio.rs
 * before its first request or internal descriptor) sends every close to the Rust path, which
 * cancels the descriptor's requests first. */
	.section .text.niji_machine_close,"ax",@progbits
	.globl niji_machine_close
	.hidden niji_machine_close
	.type niji_machine_close, @function
	.p2align 6
	.cfi_startproc
niji_machine_close:
#ifdef NIJI_EXPORT
	.globl close
	.type close, @function
close:
	.globl __close
	.type __close, @function
__close:
#endif
	cmpb $0, niji_machine_aio_used(%rip)
	jne niji_close_rest
	movl $3, %eax			/* SYS_close */
	syscall
	cmpb $0, niji_machine_owned(%rip)
	je 1f
	movq %fs:0xa0, %r11
	testq %r11, %r11
	jz 1f
	movl 32(%r11), %r11d
	andl $13, %r11d			/* DISABLED | REQUESTED | ACTING */
	cmpl $4, %r11d			/* == REQUESTED: due */
	je 2f
1:	cmpq $-4095, %rax
	jae 2f
	ret
2:	movq %rax, %rdi
	jmp niji_close_point_fail
	.cfi_endproc
	.size niji_machine_close, .-niji_machine_close

/* The asynchronous-I/O flag (read above; written only by crates/boundary/src/aio.rs). In a section of
 * its own so that crates/libc-so/startup-state.list can declare it: the first close reads it. */
	.section .data.niji_machine_aio_used,"aw",@progbits
	.globl niji_machine_aio_used
	.type niji_machine_aio_used, @object
niji_machine_aio_used:
	.byte 0
	.size niji_machine_aio_used, 1
/* D-X.58: set (by crates/boundary/src/thread.rs, `init_initial`) when the runtime started the
 * process, so every thread block that runs library code is one it made; clear in a process another
 * C library or loader started, whose blocks' words at 0x9c-0x187 are not libniji's: the entries
 * above then never read %fs:0xa0 and take the Rust path. Written at start-up, so in a section of its
 * own that `cargo xtask so` moves into the start-up state cluster (crates/libc-so/startup-state.list). */
	.section .data.niji_machine_owned,"aw",@progbits
	.globl niji_machine_owned
	.hidden niji_machine_owned
	.type niji_machine_owned, @object
niji_machine_owned:
	.byte 0
	.size niji_machine_owned, 1
	.section .note.GNU-stack,"",@progbits
