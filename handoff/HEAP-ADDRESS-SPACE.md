# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-10` (continues -9 .. -1); origin/next `d4cbe1667` merged (`3750e5800`, the
brief's "rebase on LAYOUT-RO-STABLE" done as a merge: a rebase would need a force-push). Build role keke `~/w/r1`
(Zen 1, 60 vCPU); scripts in keke `~/work/heap-address-space-10/` (`verify.sh`, `chain.sh`, `benchcopy.sh`,
`soexport.sh`). Spec: `~/state/briefs/HEAP-ADDRESS-SPACE-9.md` "## The change" + `-10.md` (ruling #7028, #7061).
Decisions in force: D-3.19, D-3.14, D-X.103, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3: growth with use; run -10 added the exact-placement rule), `-6-1.toml` (D-X, head-bin rule). Earlier runs'
reports: handoff/HEAP-ADDRESS-SPACE-9-report.md, -8-report.md.

## Run -10 so far

- Fuzz-check red: next's `ForeignPages` lacked `map_room`/`grow`: delegations added (`e76808c9a`).
- xtest red of run -9 (`kernel_pages_map_aligned_under_an_address_space_limit`, interposed suite only, 16 of 20 runs
  on keke): a LIBRARY defect, not the test. `map_aligned_exactly` (the exact path under RLIMIT_AS) tried only the
  aligned address just above and below the kernel's choice; strace: the kernel's (2 MiB-aligned) choice was a gap above
  the heap's own segment holding the bytes but no aligned range, both MAP_FIXED_NOREPLACE candidates EEXIST, map
  failed with room under the limit (issue #21's malloc failing early under `ulimit -v`). Fix `39acf3bd3`: such a gap is
  blocked with one-page PROT_NONE NOREPLACE mappings every `head + len` bytes inside (down - head, up + len) (first one
  a step above `down - head`, which is never inside the gap), up to 8 rounds (EXACT_ROUNDS), at most 8 blockers a
  round (EXACT_BLOCKERS), all removed before returning; the function is `#[cold] #[inline(never)]`.
- New deterministic test `kernel_pages_map_aligned_past_a_gap_without_an_aligned_range` (`88506b099`): a SEG + 2 MiB
  gap with no SEG multiple in its first 2 MiB, checked to be the kernel's first choice for SEG bytes. `verify.sh`
  (job `heap-address-space-10-v2`, rc 0): xtask clippy, cargo clippy --all-features -D warnings, static clean; both
  tests 20/20 native and 20/20 interposed. Plants (`heap-address-space-10-plants`, rc 0): 6 of 6 CAUGHT with their
  messages (handoff/HEAP-ADDRESS-SPACE-10-plants.txt).
- `cargo xtask affected --list --base origin/next`: FULL: the train's (xtask/src/bins.rs, run -6's change).

## OPEN: xtest red at the products step (job `heap-address-space-10-xtest`, head `88506b099` tree)

`layout: 1 of 13 contracts broken: WINDOW-BIN-BYTES`: fast profile "windows per mode ... tool 3/2" (the tool mode
touches window bins 0, 1, 2; budget 2). Head fast bin 0: 63980 of 64512 bytes, 104 members; bin 1 58090; bin 2 60451
(locale,strings,tool). Next's RELEASE build (bench) shows tool 3/3 (release budget), not comparable. Started to compare
the same profile: `heap-address-space-10-sonext` (fast `cargo xtask so` of next in `~/work/heap-address-space-10/next`)
then `-somerge` (merge commit `e76808c9a`, without the fix, in `.../merge`). Read their "windows per mode" and
"window bin" lines and diff `target/product/fast/layout-bins.tsv` (tool-mode members, heap symbols such as
map_room / map_aligned_exactly / map) between next, merge and r1 (head). If next or the merge alone has tool 3/2: not
this fix (report with numbers; merge-only = interaction of -9's branch with next). If only the head: the fix's text
moved a tool-mode function: find which (the tsv), keep hot functions byte-for-byte (§9), fix, re-run xtest.

## Other jobs started (keke)

- `heap-address-space-10-fullhot` (chain after xtest, on r1: NIJI_FULL_PROFILE so + hot_text_products).
- `heap-address-space-10-cbnext` (rc 0): next release counts, libniji: startup_instr_c 2160, startup_kinstr_c 146779,
  setloc 62087, printf 65722, wctype 62586; static cbench_startup 5 LOADs (two read-only: 0x701000 and 0x7a0000).
- `heap-address-space-10-cbhead` (running): same for the head in `~/work/heap-address-space-10/head`; compare rows and
  LOAD count; if the LOAD counts differ, the pad method of ruling #7061 (report unpadded pair too).

## Exact next step

1. jobwait sonext, somerge, cbhead, fullhot; settle the WINDOW-BIN-BYTES red as above (no reruns to pass).
2. After the last code change: verify.sh (lint + the two tests), xtest, fullhot again; cbench_startup head vs next,
   no row worse; precheck `--jobs <labels> --head <sha>`; report json; handback-check.
3. Hand back. Follow-ups (own stories): HEAP-SPILL-COLD; relro by-section comparison (#6325); returning empty
   segments' address space.

## Do not redo

The +1 dynamic fault hunt (-8); the fixed-layout #21 check (#6566, -7); the `.got` alignment (dropped -8); the -9
cbench_startup cause (an extra read-only PT_LOAD at a static link's boundary); the -10 xtest cause (above).
