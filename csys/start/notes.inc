/* The notes of the start files (CRT-START-FILES, GitHub #27), included by csys/src/machine/entry.S
 * (crt1.o, Scrt1.o, rcrt1.o, gcrt1.o, grcrt1.o), crti.S and crtn.S. Not a source of its own: the C
 * layer's archive takes only csys/src, and nothing assembles this file alone.
 *
 * NIJI_NOTE_PROPERTY: `.note.gnu.property` (x86-64 psABI, "Program Property"), one
 * NT_GNU_PROPERTY_TYPE_0 (5) note, owner "GNU", whose properties are sorted by type, each 8-byte
 * aligned on ELFCLASS64:
 *   GNU_PROPERTY_X86_FEATURE_1_AND (0xc0000002) = IBT (1) | SHSTK (2): every indirect-branch target
 *     in the including object starts with endbr64 (_start, __gmon_start__, _init, _fini), and no
 *     code in it returns to an address other than its caller's (SHSTK);
 *   GNU_PROPERTY_X86_ISA_1_NEEDED (0xc0008002) = x86-64 baseline (1): no instruction beyond it.
 * The link editor ANDs the feature property over its inputs and keeps the ISA one only when every
 * input has it: an input without the note drops both from the program.
 *
 * NIJI_NOTE_ABI_TAG: `.note.ABI-tag` (LSB Core, "ABI note tag"), NT_GNU_ABI_TAG (1), owner "GNU",
 * OS 0 (Linux) and the earliest kernel the executable needs, 3.2.0, the value the reference start
 * files carry (a readelf probe of the installed Scrt1.o). */
	.macro NIJI_NOTE_PROPERTY
	.pushsection .note.gnu.property,"a",@note
	.p2align 3
	.long 4
	.long 32
	.long 5
	.asciz "GNU"
	.long 0xc0000002
	.long 4
	.long 3
	.p2align 3
	.long 0xc0008002
	.long 4
	.long 1
	.p2align 3
	.popsection
	.endm

	.macro NIJI_NOTE_ABI_TAG
	.pushsection .note.ABI-tag,"a",@note
	.p2align 2
	.long 4
	.long 16
	.long 1
	.asciz "GNU"
	.long 0
	.long 3
	.long 2
	.long 0
	.popsection
	.endm
