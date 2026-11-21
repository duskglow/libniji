# DYN-START-RING: cut the ring transitions of a dynamic start

State: branch `task/dyn-start-ring` from origin/next 0122a0d8a (not merged since; next is 37 commits ahead, none in
the loader/link files this touches); role keke:~/w/n1 (Zen 1); Skylake-SP rows on mei (~/work/dyn-start-ring).
Leg 2 found and fixed the leg-1 regression; the final head's gate, plants and start-up checks were started (queued
behind other jobs at hand-back time); see the report for their labels.

## Commits

1. 771ed7154 lazy internal-signal handler (owner ruling #7075): cancel.rs `install` once per process, called from
   thread.rs `create_inner` before the clone; runtime.rs start-up calls removed; conformance s20_internal_signal
   updated. Record records/pending/DYN-START-RING-1.toml (amends D-20.1), differences row sentence.
2. fbe54cca4 loader NULL-folding decision without `/proc/sys/vm/mmap_min_addr` (ld.so's dynamic symbol table has no
   copy/fill entries; only its own code calls them). records/pending/DYN-START-RING-2.toml (amends D-4.22).
3. 6f15029fe, b69f20d2f view arena + start mappers keep reservation slack. records/pending/DYN-START-RING-3.toml.
4. 28fe215ab system-call bound test (hot_text_products.rs), plants file.
5. fc5f304e6 (leg 2) **the regression's cause**: libc.so.6 link-layout flip, not loader code. The first read-only
   segment shrank 0x168 bytes, `.plt` moved to 0x2e000, the gap before the 64 KiB-aligned `.text` (0x30000) held a
   whole page and GNU ld split the text into two executable PT_LOADs (one more header + mapping per start). Fix:
   `PLT_AFTER_TEXT` linker fragment (xtask/src/so.rs, D-X.143's method) + `check_executable_segment` + two unit tests;
   records/pending/DYN-START-RING-4.toml (amends D-X.143). Same commit: view arena 1 GiB -> 64 MiB (the 1 GiB one
   pushed libc and later mappings from 0x7ffff7.. to 0x7fffb7.., another 1 GiB page-table range).
6. 6ec5642f5 bounds set (calls besides openat, tracer excludes execve): empty 23, three 50, clock 23; 2 new plants.
7. 414a84a0d comment/record wording only (the 1 GiB arena and the split were not measured apart).

## Measurements (median of 101, ASLR off, rp.sh: dsq-probe kit + static s0; equal-length dirs base0 / aft02)

Skylake-SP (mei, interleaved A/B/A/B, job dyn-start-ring-sk-abab1; both rounds agree):

| row | base0 user / kernel / faults / syscalls | aft02 | glibc 2.39 (job dyn-start-ring-sk-base0) |
|---|---|---|---|
| n0 | 763,613 / 806,255-807,415 / 59 / 37 | 763,008 / 772,774-773,456 / 58 / 29 | 106,444 / 722,426 / 49 / 46 |
| n3 | 1,027,170 / 1,237,187-1,246,018 / 75 / 79 | 1,026,324 / 1,182,523-1,183,001 / 74 / 59 | 141,648 / 1,023,332 / 65 / 72 |
| nt | 798,698 / 834,264-834,991 / 62 / 37 | 798,093 / 801,464-803,072 / 61 / 29 | 108,554 / 737,045 / 50 / 46 |
| s0 | 2,088 / 145,730 / 7 / 10 | 2,069 / 145,101-145,334 / 7 / 9 | gs0 138,540 / 284,727 / 25 / 15 |

Zen 1 (keke, job dyn-start-ring-so3, base0 from leg 1's dyn-start-ring-base0, NOT interleaved, host load ~20):
user n0 763,452 -> 762,847, n3 1,027,009 -> 1,026,163, nt 798,566 -> 797,961, s0 2,076 -> 2,057; faults -1 on each
dynamic row; syscalls n0 36 -> 28, n3 78 -> 58, nt 36 -> 28, s0 10 -> 9. Kernel medians n0 822,950 -> 788,748 but
n3 1,260,051 -> 1,264,048, nt 852,228 -> 871,322 and s0 (unchanged code) 149,804 -> 159,502.
Interleaved Zen 1 A/B (job dyn-start-ring-abab1, two rounds, kernel median / min): n0 824,174 / 815,823 and 825,591
-> 791,400 / 782,098 and 790,876 (-33k both rounds); n3 1,263,020 and 1,337,842 -> 1,269,120 and 1,208,055 (mins
1,256,685 -> 1,203,985); nt 853,524 and 854,983 -> 873,619 and 867,767 (**median worse**, mins 845,024 -> 819,138);
s0 150,031 and 156,032 -> 159,494 and 159,772 (mins equal; static code unchanged but for one rt_sigaction fewer).
The Zen 1 kernel medians mix two modes under host load ~20 (s0 shows it on unchanged code); job
dyn-start-ring-krep1 (krep.sh: 8 alternating rounds of nt, s0, n0, run beside the gate, so all levels are higher)
settled it: nt base0 858,253 / 908,319..911,377 vs aft02 839,519 / 870,587..873,058 (lower in all 8 rounds, about
-37k); n0 829,026 / 879,350..882,559 vs 788,346 / 840,938..843,802 (lower in all 8); s0 151,449 / 160,744..161,200
vs 150,090 / 160,179..160,886 (round 1 flipped mode, else level or lower).

Syscalls aft02 n0: execve, arch_prctl SET_FS x2, openat x4 (preload, hwcaps v3, v2, libc), fstat, mmap x11, close,
rseq, mprotect x2, munmap (the arena), set_tid_address, uname, set_robust_list, exit_group.

## Next step

1. Collect: `~/bin/jobwait dyn-start-ring-gate2 dyn-start-ring-startup1 dyn-start-ring-plants2 dyn-start-ring-abab1`
   on keke (chain.sh runs gate2 then startup1 on the role). Fix a red, then precheck:
   `cargo xtask precheck --jobs dyn-start-ring-gate2,dyn-start-ring-startup1,dyn-start-ring-plants2 --head <sha>`.
2. Zen 1 kernel ratchet: read abab1; if a Zen 1 kernel row is worse in the interleaved runs, profile it.
3. Not done (brief items left): uname, the early ARCH_SET_FS pair, initial thread block/TLS merge, read_phdr per-header
   user cost, whole-file views replaced by header reads, the layout-bins.tsv diff against the base (no base copy).
