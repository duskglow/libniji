# LD-START-MAPS: the start's mappings at or below the reference's kernel cost

Proposed by STARTUP-STRUCTURE-REVIEW (C6, rank 7). Kind: code. Size: M. Coordinate with the layout owners (fault
bounds).

Premise: per start, libniji runs 758,398 kernel instructions and takes 59 faults against glibc 2.39's 649,293 and
50 (the static floor is 298,579 and 26). It makes fewer system calls (26 vs 29) but 11 mmaps vs 7:
* a 64 MiB PROT_NONE reservation;
* a whole-file read-only view of libc.so.6 (4.7 MB) before its segments;
* three anonymous areas (16 KiB, 70 KiB, 68 KiB).

Change: read headers with pread into the start arena instead of a whole-file view on the start path; reserve and
map each object's span once; fold the anonymous areas where the thread-block layout allows (D-22.9).

Must prove: kernel instructions and faults per start <= the reference's on both CPU classes (host kit form);
DYN-START-PARITY-2's file-system dependence gone.
