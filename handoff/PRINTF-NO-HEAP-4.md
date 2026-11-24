# PRINTF-NO-HEAP-4 (handoff; continues PRINTF-NO-HEAP-3)

State: branch `task/printf-no-heap-4`, head `68a093693` (code), merged with origin/next 81bdd4b45 (LOCALE-PER-DIR-LOAD
and the restored locale archive, coordinator note). Role shiki:~/w/sh1 (Haswell, 26 vCPU). Decisions in force:
D-12.32, D-12.33; ruling #7808. Spec: ~/state/briefs/PRINTF-NO-HEAP-3.md. Predecessor record:
handoff/PRINTF-NO-HEAP-3.md (and -2, the land bar). Plants: handoff/PRINTF-NO-HEAP-4.plants (all 20 of the story).
Report: handoff/reports/PRINTF-NO-HEAP-4.json.

## What this run did
- Step 0 on the inherited head a8ea058c9: clippy -D warnings + `cargo xtask static` rc 0 (`printf-no-heap-4-lint-0`).
- Step 1, the named failure (gate-4 on mei): all five failed tests were the locale-archive host change (#7846), which
  the coordinator reports cleared; no printf cause. Rechecked by the gate on this head (below).
- `f34655be7` merge origin/next. One conflict, layout/contracts.tsv WINDOW-BIN-BYTES `windows-release`: this branch
  had tightened float 2->1, next had tightened tool 3->2; both tightenings kept (float:1, tool:2).
- `81d9feccd` handoff/PRINTF-NO-HEAP-4.plants: the 15 PRINTF-NO-HEAP-2 plants named `--test printf_no_heap`, a binary
  leg 4 folded into s12_stdio_symbols; they now run `--test s12_stdio_symbols no_heap` (every pattern still matches
  once), plus the 3 leg-4 plants.
- `27b79b48b` + `68a093693` (test only): the first plants run (`printf-no-heap-4-plants-1`) stopped at an unplanted
  baseline: `sym_fputs` "line-buffered: through the last newline", lengths [0,4,7,11,13,83,84] for
  [0,4,7,10,13,83,83]. Cause: under `cargo test` the tests of that binary are threads of one process, and input from
  an unbuffered or line-buffered stream flushes every line-buffered stream (ISO C 7.21.3p3; libniji does this in
  stdio_compat `flush_line_buffered`, try-only), so a test reading beside it writes the held "h"/"x" early. Not a
  libniji defect, and the gate cannot see it (nextest: one process per test). Same race in three checks:
  fputs_fuse_corners, fputs_nbf_writes, front_corners' line-buffered block. New `lbf_alone` runs them through
  `alone` in a child of the calling test (libtest names the thread after the test). Plants lbf-drain-all and
  lbf-setvbuf-unbuffered prove the moved checks still run and catch.

## Jobs on 68a093693 (shiki)
- `printf-no-heap-4-lint-3`: clippy --workspace --all-targets --all-features -D warnings + `cargo xtask static`: rc 0.
- `printf-no-heap-4-gate-3` (direct tier, --base origin/next, 19:38-20:14Z): rc 0, 20 suites, 11905 passed, 0 failed,
  conformance 137/137, peak 13G. The five #7846 locale reds of gate-4 on mei are gone (shiki has its archive).
- `printf-no-heap-4-plants-3` (/mnt/zram/claude/printf-no-heap-4/plants-3): 20 plants, 20 CAUGHT, 0 SURVIVED, no WARN;
  all baselines green, including the full `s12_stdio_symbols` run. Results: handoff/PRINTF-NO-HEAP-4-plants.txt.
- `printf-no-heap-4-xtask-test-3` (`cargo xtask test`): rc 0.
- cbench, `cargo xtask bench --compare --counters --runs 3 cbench_opt cbench_stdio`, instr/call: base = origin/next
  81bdd4b45 (`printf-no-heap-4-basebench-1`, a RAM copy of the primed tree), head = `printf-no-heap-4-cbench-ram-3`
  (RAM copy of the role) and `printf-no-heap-4-cbench-3` (the role; within 1 instr/call of it). 0 of 17 printf rows
  above base: snprintf_d 489.4 -> 487.5, ld_large 549.4 -> 546.6, x08 530.5 -> 526.7, s_path 229.2 -> 201.3
  (-12.2%), s_d_colon 961.1 -> 940.7, ls_line 2421.4 -> 2380.0, chk_ls_nlink 760.6 -> 699.0 (-8.1%), mixed
  1997.6 -> 1924.2, f2 1110.5 -> 1060.8, g 1286.5 -> 1238.0, e 1582.4 -> 1532.0, f_large 18433.9 -> 17812.7,
  fprintf_ls_line 1877.3 -> 1875.2, du_line 842.3 -> 839.8; cbench_stdio fprintf_d 633.8 -> 630.6, snprintf_d
  489.6 -> 487.5, mixed 2014.7 -> 1940.8.
- hot_text_products (NIJI_FULL_PROFILE=1 so + test): `printf-no-heap-4-hot-text-3` rc 101, 31 passed, 2 failed:
  init_dynamic_startup_only_program_stays_within_its_page_fault_bound (the dynamic `page_faults` run exited 127)
  and init_dynamic_startup_fault_count_does_not_depend_on_the_file_system (ext 10 faults vs tmpfs 58, i.e. the
  same program not starting). Not reproduced: the same head in a RAM copy passes both (`printf-no-heap-4-headhot-1`),
  origin/next passes both on shiki (`printf-no-heap-4-basehot-1`). The red run overlapped the plant copies and
  a bench build on /mnt/zram, the file system that also holds the role (73% used when I looked later). That is
  where I would look, but it is NOT VERIFIED. The two tests use separate directories (no shared-directory
  race). A full rerun on the role, with nothing else of mine running (`printf-no-heap-4-hot-text-4`), failed
  again (init_data_clusters_hold_at_every_text_phase, exit 127), which refutes the RAM-disk guess. Settled by
  `printf-no-heap-4-hotrep-1` (shiki ~/work/printf-no-heap-4/rep.sh): 10 full runs of
  hot_text_products each, origin/next copy then head copy, both under NIJI_FULL_PROFILE=1: **origin/next fails 9 of
  10, the head 9 of 10**, the same three tests (init_data_clusters_hold_at_every_text_phase,
  init_dynamic_startup_only_program_stays_within_its_page_fault_bound,
  init_dynamic_startup_fault_count_does_not_depend_on_the_file_system). The page_faults relay exits 127 (its
  vfork+execve failed; errno not printed); the head runs also show exit 99 (the `mount --bind` of the locale
  directory inside `unshare -r -m` failed) and exit 1. A host or harness fault on shiki, the same on next: not
  this story's (RULING NEEDED below). No hot_text run on shiki had ever been recorded before this one.

## Superseded
printf-no-heap-4-lint-1, gate-1 (stopped), plants-1 (baseline red, the race above) on 81d9feccd;
printf-no-heap-4-lint-2 (clippy expect_used) and plants-2 (stopped) on 27b79b48b.
printf-no-heap-4-hot-text-3 and -hot-text-4 on 68a093693: red only through the shiki hot_text fault above.

## Exact next step
Land. Then #7764 (below). The hot_text fault on shiki is a separate harness story.

## RULING NEEDED (new)
- hot_text_products is red 9 of 10 runs on shiki for origin/next itself (above). Recommend a harness story: make
  the page_faults relay print the execve errno and find why exec and the locale bind mount fail there. Until then,
  do not admit start-up stories to shiki, or accept the base's numbers as this story's start-up check.

## RULING NEEDED (carried from PRINTF-NO-HEAP-3, still open)
- Layout tracer: the directory-search `candidates` instance is never traced (#7884: layout story recommended).
- LOCPATH without the locale: the reference falls back to the default directory, libniji returns NULL (#7885).
- Layout planner head-bin tail, xtask/src/bins.rs `head_takes` (#7886).

## Follow-ups still open
#7764 (a) grouped/`I` floats past 128 bytes allocate; (b) registered conversions allocate. Not started.
Other s12_stdio_symbols checks that observe a stream's file while other tests may flush were not audited beyond the
three line-buffered blocks (the fgets-flush check is safe: an early flush changes nothing it observes).

## Not to redo
The +28 profile and fixes; leg-1 strings cause; leg-3 hot_text diagnosis; leg-4 fold; the lbf race diagnosis above.

## Refused
None.
