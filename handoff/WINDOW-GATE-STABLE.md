# WINDOW-GATE-STABLE: the fast window gate judges what ships (D-X.145)

State: branch `task/window-gate-stable`, code commit `02df7fc7c` (later commits: handoff/ only). Build role on the
Skylake-SP host (48 threads, role on zram). Decision in force: D-X.145 (amends D-X.136). No product code changed.

## What changed

- `layout/contracts.tsv` WINDOW-BIN-BYTES: new `windows-block=release`. `xtask/src/layout.rs::window_counts_block`
  reads it (empty list, or a profile without `windows-<profile>`, is an error).
- `xtask/src/binbudget.rs::verdict`: the windows-per-mode lines (over and under) go to `broken` only where the profile
  blocks; elsewhere (fast) they print under `WINDOW-COUNT REPORT (non-blocking, D-X.145 ...)`. Bin bytes, bin
  placement, unbudgeted or vanished modes, `.rodata1`/`.data1` and the D-X.138 data bins block in every profile.
- Tests: `binbudget::tests::a_profile_whose_counts_do_not_block_reports_them_and_keeps_the_byte_checks`,
  `layout::tests::window_counts_block_in_the_shipped_profile_only`. Plants: `handoff/WINDOW-GATE-STABLE-plants.txt`.

## Measurement (Skylake-SP host; `cargo xtask so`, job labels window-gate-stable-so-*)

| run | profile | tree | cargo-libc | so total | WINDOW-BIN-BYTES |
|---|---|---|---|---|---|
| so-base | fast, 16 units | next (cold target) | 34.2 s | 401 s | ok, tool 2/2, bins 0-2 64072/53098/65153 |
| so-heap16 | fast, 16 units | next + HEAP-10 heap.rs files | 41.7 s | 484 s | RED tool 3/2 (bins 63980/57706/60515) |
| so-heap1cgu | fast, 1 unit | next + HEAP-10 | 80.2 s | 238 s (failed) | trace-at-build failed: STARTUP-DATA-DECLARED `heap::RSEQ_NODE.0` |
| so-next1cgu | fast, 1 unit | next | 84.5 s | 237 s (failed) | same failure on next alone |
| so-relbase | release | next | 94.1 s | 421 s | ok, tool 3/3, all modes at budget |
| heapb2 (fast) | fast, 16 units, this branch | next + HEAP-10 heap.rs files | 47.0 s | 899 s | ok: same bins as so-heap16, `tool 3/2` now a WINDOW-COUNT REPORT line; 13 of 13 contracts hold |
| heapb2 (release) | release, this branch | next + HEAP-10 | 148.1 s | 504 s | ok, every mode at budget, tool 3/3 (= next) |

The HEAP case's two heap.rs files were taken from `origin/task/heap-address-space-10` (the brief names its branch)
as an uncommitted overlay on the role, never committed here; next's copies equal its merge base's. The trace step
(libc-archive) varies 283-802 s run to run with host load; compare the cargo-libc rows for compile cost.

Option A (one codegen unit in the fast profile) rejected: about 2x the libc crates' compile after a heap.rs edit
(41.7 -> 84.5 s), breaks trace-at-build on next itself (a tracer rule and new fast levels everywhere would be
needed), and still judges a thin-LTO layout that ships nowhere. Option B chosen: no extra build in the per-story gate;
the release verdict comes from `NIJI_FULL_PROFILE=1 cargo xtask so`, which start-up stories (§9) and CI already run.
Gate run time: unchanged (the fast verdict computes the same lines; only their blocking changes). The `so` total is
dominated by the gdb trace in both profiles (libc-archive 283-431 s), so the compile is not where the time goes.

Accepted cost (in D-X.145): a story whose gate builds only fast products no longer sees a window-count regression
there; it sees it at its full-profile start-up check or in CI.

## Plants

`window-gate-stable-plants-2` (`handoff/WINDOW-GATE-STABLE-plants.txt`): 4 plants, 4 CAUGHT, no WARN lines.
`counts-always-block` and `counts-never-block` (binbudget unit tests), `block-list-ignored` (layout unit test),
`release-hot-growth`: 16 KiB of nops at the head of `process::run_thread_dtors` (runs in every mode), judged by
`NIJI_FULL_PROFILE=1 cargo xtask so` on traced products: `float 3/2, fork 3/2, locale 3/2, tool 4/3`, WINDOW-BIN-BYTES
BROKEN. Superseded: `window-gate-stable-plants` (that plant's mutant did not compile: `niji-boundary` denies
`unsafe_code`; fixed with a statement-level allow, 3 other plants CAUGHT there too).

## Open questions / next step

- The fast data bins (D-X.138) can in principle move with the same codegen-unit re-partitioning (per-unit `anon.*`
  constants in `.data.rel.ro`); none moved in HEAP-10's case. If one does, the same `*-block` switch fits there.
- Next step: none for this story; land it. HEAP-ADDRESS-SPACE-10's fast red becomes a WINDOW-COUNT REPORT line.
