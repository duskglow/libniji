/* Class owner: the link-time interface stub of the opt-in (DL-ATEXIT-OWNER, D-27.29). Built as
 * liella-owner/libc-owner.so with soname libc.so.6: it is never loaded, only read by the link
 * editor, which then records `atexit`, `at_quick_exit` and `pthread_atfork` references at
 * LIELLA_OWNER_1 (a version need on libc.so.6). Under Liella those bind to libc.so.6's
 * non-default versions; a loader without the node refuses the module at load, naming it. */
	.text
	.globl atexit
	.type atexit, @function
atexit:
	ud2
	.size atexit, .-atexit
	.globl at_quick_exit
	.type at_quick_exit, @function
at_quick_exit:
	ud2
	.size at_quick_exit, .-at_quick_exit
	.globl pthread_atfork
	.type pthread_atfork, @function
pthread_atfork:
	ud2
	.size pthread_atfork, .-pthread_atfork
	.section .note.GNU-stack,"",@progbits
