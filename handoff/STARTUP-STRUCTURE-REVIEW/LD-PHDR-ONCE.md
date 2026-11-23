# LD-PHDR-ONCE: decode each object's program headers once

Proposed by STARTUP-STRUCTURE-REVIEW (C3, rank 3). Kind: code. Size: S-M.

Premise: `Elf::read_phdr` runs ~550 times per empty start (82,779 instructions exclusive; 149,568 with 3 libraries:
~22k per extra object). Callers: `image_span_kind` (144 calls), `link` (107), `check_table` (109),
`Object::new` (115), `published_dynamic` (44), `dynamic` (32). DYN-START-LINK-SHAPE (d) named it.

Change: one validated decode per object into a fixed array (count bounded as `check_table` bounds it today). Every
walk iterates the array.

Must prove: `read_phdr`-class work <= 10k on the empty main; 3-library marginal -20k per object; layout and fault
bounds unchanged (start-up checks §9).
