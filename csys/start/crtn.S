/* Class machine: the epilogue of the `.init` and `.fini` functions, the content of niji-crtn.o
 * (installed as crtn.o; SL-CRT, whose decision record states the start files' rules). The compiler driver
 * links it last (GCC's endfile spec ends with `crtn.o%s`), so these halves close the functions
 * crti.S opens: give back the 8 bytes the prologue took and return.
 *
 * Alignment 1 (no .p2align): the link editor must put no fill between the last contribution and
 * this code, which every contribution falls through into. */

	.section .init,"ax",@progbits
.Lniji_machine_crtn_init:
	addq $8, %rsp
	ret

	.section .fini,"ax",@progbits
.Lniji_machine_crtn_fini:
	addq $8, %rsp
	ret

	.section .note.GNU-stack,"",@progbits
