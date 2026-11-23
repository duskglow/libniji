# LD-RELR-LOOP: packed relative relocations for libc.so.6, and a tight self-relocation

Proposed by STARTUP-STRUCTURE-REVIEW (C4, rank 5). Kind: code. Size: S-M. Needs the RELR ruling (pending since
DYN-START-PARITY).

Premise: libc.so.6's 1,573 RELATIVE entries cost ~57k in `pass1` (about 30 each with the table walk). ld.so's own
232 words cost 16,250 (~70 each). DYN-START-LINK-SHAPE (a) measured RELR at +113k with the old loop: the loop must
get the window treatment first.

Change: a RELR loop at a few instructions per relocated word (bitmap words, the window refreshed only on a miss);
libc.so.6 linked with `-z pack-relative-relocs`; the boot self-relocation through the same tight loop.

Must prove: RELATIVE work plus self-relocation <= 15k (from ~89k); .rela.dyn about 36 KB smaller; faults not up.
