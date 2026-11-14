/* The content of liella-Mcrt1.o (installed as Mcrt1.o; SL-CRT, D-NEW in
 * records/pending/SL-CRT-1.toml): no code, no data and no symbols.
 *
 * GCC's x86-64 GNU/Linux driver never names Mcrt1.o (its startfile spec picks gcrt1.o for -pg,
 * -p and -profile, grcrt1.o for those with -static-pie, rcrt1.o, Scrt1.o or crt1.o otherwise),
 * so nothing links it unless a link line names it by hand. The development package ships the
 * file where a build looks for it, and an object that names it links as if it were absent.
 * The one section marks the stack non-executable: an object without it would make the link
 * editor give the program an executable stack. */
	.section .note.GNU-stack,"",@progbits
