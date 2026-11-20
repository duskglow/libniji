# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-9` (continues -8 .. -1; base origin/next `3f2a84520`, merged in run -8); head =
this handoff's commit. Build role keke `~/w/r2` (Zen 1, 60 vCPU); scripts and binaries in keke
`~/work/HEAP-ADDRESS-SPACE-9/` (`probe.sh`, `probe2.sh`/`probe3.sh` + `pad.S`, `probe4.sh`, `chain.sh`). Spec:
`~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change"; rulings #6513, #6514, #6566, #6997, #7001. Decisions in force:
D-3.19, D-3.14, D-X.103, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml` (D-3: growth
with use), `-6-1.toml` (D-X: amends D-X.103, head-bin rule). Code commits: -8 `10e62ddd2`; -7 `c7f02c3f6`,
`6591abefd`; -6 `8afd17494`, `ce6328854`, `ad420da74`; -5 `7c6a4078b`. Run -9: test only (`d3edbb094`, `60ec2a659`).
Full report: handoff/HEAP-ADDRESS-SPACE-9-report.md.

## Run -9: the cbench_startup cause (ruling #6997) is the static link's layout, not the heap

- The C start-up child (`cbench_startup true`, static) never calls malloc: strace base vs head identical (execve,
  arch_prctl, the 67104-byte TLS mapping, set_tid_address, set_robust_list, rseq, rt_sigaction, mprotect, exit_group);
  page faults identical (8-9).
- The head's binary has 5 PT_LOADs, next's 4. `.rodata` is byte-identical (0x922dc); `.text` grew 0x4c0 (1216 B)
  and crossed a page, so `.rodata` starts one page later and ends 0x2dc past 0x790000; the 64 KiB-aligned
  `.rodata1` (start-up read-only window) moves to 0x7a0000 and ld starts a new read-only LOAD across the ~60 KiB gap.
  On next `.rodata` ends 0xd24 below the boundary (one page of slack: no split). Any change that adds >= 985 bytes
  of text to this link splits it.
- Proof (Zen 1, 101 runs each, medians): base 4 LOADs kernel 165.7k / 164.3k, user 2190; base + 4 KiB of unrelated
  text 5 LOADs 173.4k, user 2198; head 5 LOADs 172.3k / 173.5k, 2198; head + 60 KiB text pad 4 LOADs 166.7k, 2190.
  The whole startup_kinstr_c (+7.5k) and startup_instr_c (+8) delta is the extra PT_LOAD.
- The heap's own start-up cost, LOAD count held equal (base vs head + pad): setlocale / printf / wctype children user
  +6 / +4 / +6 (min), kernel within run-to-run noise (min +0.6k / +1.7k / -2.0k, median +1.7k / -2.1k / -1.7k), faults
  equal, same system calls (mmap, two munmap, mbind; mbind now covers 4.3 MiB, was 16 MiB).
- Ruling #7001: test `kernel_pages_map_aligned_under_an_address_space_limit` (s03_memory_symbols): under an RLIMIT_AS
  that refuses the over-mapping, `map` and `map_room` return aligned zero-filled memory, `grow` maps a free range and
  refuses a taken one. Plants: handoff/HEAP-ADDRESS-SPACE-9-plants.txt.

## Do not redo

- The +1 dynamic fault hunt (run -8). The fixed-layout issue #21 check (#6566, run -7). The `.got` alignment (dropped
  run -8). The cbench_startup cause (above): it is measured; reshaping heap code to dodge one binary's boundary would
  be luck, not a fix.

## Exact next step

FIRST: merge origin/next (`189ab9cb` or later; this run's brief allowed no merge). Next's new `ForeignPages`
(boundary heap.rs, `impl Pages for ForeignPages`) lacks this branch's `map_room` and `grow`, so the merged tree does
not compile (handback-check's fuzz-check, run -9). Add both as delegations to `KernelPages` (as `OwnPages` does),
then lint, the affected tests, plants (HEAP-ADDRESS-SPACE-8 and -9 files), fullhot, `cargo xtask test`, precheck.
Then the RULING NEEDED (in the hand-back): land with the cbench_startup row recorded as layout jitter and a layout story to
remove the read-only LOAD split for every static program (recommended), or trim >= 232 bytes of heap text so this
one link stays below the boundary. Then the open follow-ups, each its own story: HEAP-SPILL-COLD (the spill as a cold
extension of grow_segment; refill/realloc/AS_HEADROOM to 33092 / 35144 / 35152; findutils `find` under the issue's
`ulimit -v`; randomised limited check, #6566); relro by-section comparison (#6325 step 1); returning empty segments'
address space. DYN_TRUE_BOUND Zen 1 count: see the report.
