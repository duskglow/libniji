/* Class owner: the opted-in registrations of libc.so.6 (DL-ATEXIT-OWNER, D-27.29; owner rulings
 * #4860-#4862). Each entry is exported at the Liella-only version node LIELLA_OWNER_1 as a
 * NON-default version (single `@`), so a default link never binds it and keeps taking the
 * libc_nonshared.a stubs; only a module linked through the link-stub directory (liella-owner/)
 * references it. The entry passes its own return address (the caller's, [rsp] at entry, before
 * any push) as the last argument to the C library's registration, which attributes the entry to
 * the module holding that address. Frame-free tail jumps. Each entry starts a 64-byte line in its
 * own section, like every exported entry (D-4.33, layout contract FN-ALIGN-EXPORTS). Linked
 * into libc.so.6 only, never into libc.a: a static program has one module and nothing to
 * attribute. */
/* Built twice: into libc.so.6 with each entry at the NON-default version, and with
 * -DLIELLA_OWNER_LINK_STUB into the link-time interface liella-owner/libc-owner.so (the same
 * library otherwise) with each at the default version, so an opted-in link binds it. */
#ifdef LIELLA_OWNER_LINK_STUB
#define OWNER_VERSION(entry, name) .symver entry, name@@LIELLA_OWNER_1
#else
#define OWNER_VERSION(entry, name) .symver entry, name@LIELLA_OWNER_1
#endif

	.section .text.liella_owner_atexit_entry,"ax",@progbits
	.globl liella_owner_atexit_entry
	.type liella_owner_atexit_entry, @function
	.p2align 6			/* exported entries are 64-byte aligned (D-4.33) */
liella_owner_atexit_entry:
	/* atexit(f): rdi = f; rsi = return address. */
	movq (%rsp), %rsi
	jmp liella_owner_atexit@PLT
	.size liella_owner_atexit_entry, .-liella_owner_atexit_entry
	OWNER_VERSION(liella_owner_atexit_entry, atexit)

	.section .text.liella_owner_at_quick_exit_entry,"ax",@progbits
	.globl liella_owner_at_quick_exit_entry
	.type liella_owner_at_quick_exit_entry, @function
	.p2align 6			/* exported entries are 64-byte aligned (D-4.33) */
liella_owner_at_quick_exit_entry:
	/* at_quick_exit(f): rdi = f; rsi = return address. */
	movq (%rsp), %rsi
	jmp liella_owner_at_quick_exit@PLT
	.size liella_owner_at_quick_exit_entry, .-liella_owner_at_quick_exit_entry
	OWNER_VERSION(liella_owner_at_quick_exit_entry, at_quick_exit)

	.section .text.liella_owner_pthread_atfork_entry,"ax",@progbits
	.globl liella_owner_pthread_atfork_entry
	.type liella_owner_pthread_atfork_entry, @function
	.p2align 6			/* exported entries are 64-byte aligned (D-4.33) */
liella_owner_pthread_atfork_entry:
	/* pthread_atfork(prepare, parent, child): rdi, rsi, rdx; rcx = return address. */
	movq (%rsp), %rcx
	jmp liella_owner_pthread_atfork@PLT
	.size liella_owner_pthread_atfork_entry, .-liella_owner_pthread_atfork_entry
	OWNER_VERSION(liella_owner_pthread_atfork_entry, pthread_atfork)

	.section .note.GNU-stack,"",@progbits
