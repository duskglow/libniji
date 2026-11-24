# LDSO-PROGRAM-MODE-SPACE: handoff (leg 1)

State: branch `task/ldso-program-mode-space`, fast-forwarded to origin/next 522ef9f30 (LD-START-MAPS-3 landed there at
13:00Z; the branch had no commits then). Code head 43f6ba8e9. Role keke `~/w/n4` (Zen 1, 60 vCPU, kernel 6.8), synced
at 43f6ba8e9. Decision in force: D-27.98 (reserved; `records/decisions/D-27/D-27.98.toml`). Plants:
`handoff/LDSO-PROGRAM-MODE-SPACE-plants.txt`. Scratch on keke: `~/work/ldso-program-mode-space/` (`measure.sh`,
`alignprobe.sh`, `vmprobe.c`, `maps_at_exit.c`, `set_interp.py`; `prod-before` = 88c0518f6 products, `prod-lsm3` =
522ef9f30, `prod-after` = 43f6ba8e9; `out-*/find.txt`). Coordinator note (#8370, 13:16Z): if no gap, land guard
test + plant only. A gap was found for programs aligned above the page, so the loader change is in.

## Finding (step 2)
Program mode's only extra address space beyond the reference's: the program's aligned reservation kept its slack
(`StartMapper::keeps_slack`, `p_align` minus a page). Both loaders keep the gaps between the program's segments
reserved (the kernel's exec leaves them unmapped): that is the reference's own difference. Keke's `find` is
page-aligned, so it shows neither; Yohane's numbers need an aligned `find` (or something else on their system;
the coordinator asks them for kernel version and maps at the peak).

## Measurements (KiB)
`find` (issue #21 formula: least `ulimit -v` over 200000 entries minus over 3; LANG=en_US.UTF-8, tmpfs dirs):
| tree | layout | glibc exec / prog / --library-path | libniji exec | ld.so PROG | ld.so --library-path |
|---|---|---|---|---|---|
| 88c0518f6 | fixed, 4 phases | 29904 all | 29436 | 28916..29436 | 28916..29436 |
| 88c0518f6 | randomised, 8 runs | 29904 all | 29032..29436 | 28944..29436 | 28996..29444 |
| 522ef9f30 (LSM3) | fixed, 4 phases | 29904 all | 36704..36960 | 36704..36960 | 36704..36960 |
| 522ef9f30 (LSM3) | randomised, 6 runs | (29904) | 36704..36948 | 36704..36956 | 36704..36716 |
Pinning to node 0 changed nothing. The 522ef9f30 rise is the 3-entry need falling (LSM3's start), both modes alike.
Aligned program (`vmprobe.c`, libc-linked, separate code; VmSize at entry; program mode minus exec), 0x1000/0x10000/0x200000:
reference 0 / +240 / +8176; libniji 522ef9f30 0 / +300 / +10220; libniji 43f6ba8e9 0 / +240 / +8176 (also at the walk
peak). In-tree test output (libc-free program): libniji +0/+240/+8176, reference +0/+240/+8176.
Cost: ld.so `.text` 453991 -> 457575 (+3584 B: `map_planned`/`load` instances for `ProgramMapper`), text segment
crosses a page: every process maps 4 KiB more of ld.so (VmSize 7364 -> 7368 at entry). Code is program-mode only.

## The change (43f6ba8e9)
- `crates/boundary/src/ld/image.rs`: `ProgramMapper(StartMapper)` (delegates all but `keeps_slack`, which stays
  false: the slack is trimmed by two `munmap`s); used in `program_from_headers` and `program_by_view` (both `map::load`
  calls). Start dependencies keep `StartMapper` (exec'd start code unchanged).
- Test `crates/ld/tests/ldso_program_options.rs::ldso_program_mode_maps_no_more_than_an_exec_beyond_the_reference`:
  same run, both loaders, both modes, three alignments; margin 16 KiB (`PROGRAM_SPACE_MARGIN_KIB`). 10/10 passed
  (job `ldso-program-mode-space-t1`).
- Plant `program-slack-kept` (program_from_headers back on `StartMapper`).

## Jobs (keke) on 43f6ba8e9
Started 13:5xZ: `ldso-program-mode-space-gate` (static + `affected --run --base origin/next`), `-plants`,
`-startup` (waits for the gate, then `cargo xtask test`, full-profile `so`, `hot_text_products`). Earlier:
`-so0`/`-so1` (builds), `-so2` (so + records check + clippy all-targets all-features: clean, same code as 43f6ba8e9),
`-t1` (test), probes `-m-before2`, `-m-before-pin`, `-m-before-rand`, `-m-lsm3`. Superseded: `-m-before` (stopped:
interpreter path too long for find's PT_INTERP).

## Exact next step
1. `~/bin/jobwait ldso-program-mode-space-gate ldso-program-mode-space-plants ldso-program-mode-space-startup` on
   keke; on a red: fix, rerun only that suite. Plants: expect CAUGHT with the msg.
2. §9: ld.so only changed (no libc layout key); diff `target/product/fast/layout-bins.tsv` against 522ef9f30's
   if `-startup` reports a start-up red; loader fault deltas are not this story's (LAYOUT-LDSO note in §9).
3. Sync, `cargo xtask precheck --jobs <labels> --head <sha>` on the role; write `handoff/reports/LDSO-PROGRAM-MODE-SPACE.json`
   (impact: correctness/space -2044 KiB program mode 2 MiB-aligned, -60 KiB at 64 KiB, measured; size +3584 B ld.so
   text, cost; coverage 1 test 1 plant), push, `ops handback-check`, hand back COMPLETE.
INFORMATIONAL for the hand-back: start dependencies keep their slack in both modes (D-27.92 / DYN-START-RING): a
2 MiB-aligned library keeps up to 2 MiB - 4 KiB reserved per object (not measured against the reference here);
the reference's program-mode gap reservation could also be trimmed for the program (kernel parity, up to
(segments-1) x (align - tail)), not done: it is the reference's behaviour and the span reservation's design.
