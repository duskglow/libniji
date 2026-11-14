/* Class machine: the epilogue of the `.init` and `.fini` functions, the content of liella-crtn.o
 * (installed as crtn.o; SL-CRT, D-NEW in records/pending/SL-CRT-1.toml). The compiler driver
 * links it last (GCC's endfile spec ends with `crtn.o%s`), so these halves close the functions
 * crti.S opens: give back the 8 bytes the prologue took and return.
 *
 * Alignment 1 (no .p2align): the link editor must put no fill between the last contribution and
 * this code, which every contribution falls through into. */

	.section .init,"ax",@progbits
.Lliella_machine_crtn_init:
	addq $8, %rsp
	ret

	.section .fini,"ax",@progbits
.Lliella_machine_crtn_fini:
	addq $8, %rsp
	ret

	.section .note.GNU-stack,"",@progbits
