# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-6` (continues -5 .. -1), head = this handoff's commit. Code commits on -6:
`8afd17494` (window-bin planner: phase-only groups stay out of the head bin), `ce6328854` (issue #21 bounds at the
measured levels, ruling #6513; SEG_FIRST cost in the record, ruling #6514). Run -5's code commit: `7c6a4078b`.
Build role keke `~/w/n3` (Zen 1); scripts in keke `~/work/HEAP-ADDRESS-SPACE-6/` (`pf.sh`: page_faults linked with a
`pre` text pad, run in mode rebind under `perf record -e minor-faults -d`; `basefull.sh`: merge base 7f7e6c9e4 built
with NIJI_FULL_PROFILE=1 in `/dev/shm/claude/HEAP-ADDRESS-SPACE-6/base-full`). Run -5's scripts: `~/work/HEAP-ADDRESS-SPACE-5/`.
Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change"; direction: `~/state/briefs/HEAP-ADDRESS-SPACE-5.md`;
rulings #6513 (land growth with use now, bounds at measured levels) and #6514 (keep SEG_FIRST = 16).
Decisions in force: D-3.19, D-3.14, D-X.103, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3: growth with use), `-3-2.toml` (D-X: GOT alignment), `-6-1.toml` (D-X: amends D-X.103, head-bin rule).

## Run -6

- Run -5's `fullhot` red, cause: not a hot heap function. `page_faults` rebind with a 0x800 `pre` pad took 1 fault
  at `Mo::hinted` (0x420040; perf minor-fault addresses, pf.sh). Full-profile layout-bins.tsv, base vs head: bin 0
  60626 -> 60400 plan bytes (heap `map` 728 -> `map_room` 433 etc.), and the planner then moved the rebind phase
  group (`Resolution::same_texts`, `Mo::hinted`, `Mo::form`, unlink, unlinkat, rmdir; 1142 B, signature `rebind`
  only) from bin 1 into bin 0's tail. Bin 0 is not anchored (HEAD_ROOM 0x400): with 0x800 ahead of the group its
  bytes past offset 0xF800 are alone in a second window. On base the bytes there were a cold filler.
  hot-text.signatures base vs head: only `map` -> `map_room` and `grow_segment` (no mode). Fast profile: no
  phase-only group in bin 0 on base or head.
- Fix (`xtask/src/bins.rs`, `head_may_take`): a group whose counted modes are all phase modes never joins the head
  bin; the planner test `a_measured_phase_shares_its_warm_up_windows` updated (and covers a mixed group).
- Ruling #6513: `heap_meta_products.rs` refill 39236, realloc 41288, AS_HEADROOM 42000, comments name targets
  33092 / 35144 / 35152 and HEAP-SPILL-COLD. Ruling #6514: SEG_FIRST 16 kept, VmSize cost in the D-3 record.
- Jobs (keke), all on code head `ce6328854` (synced tree 9d83ff6e0b): `heap-address-space-6-lint0` (inherited head
  0440021fb: clippy + static green), `-pf1` and `-basefull` (diagnostics, rc 0), `-lint1` (clippy, static,
  `cargo test -p xtask a_measured_phase`: green). Running at the refresh, chained in `~/work/HEAP-ADDRESS-SPACE-6/seq2.sh`:
  `-fullhot1` (NIJI_FULL_PROFILE so + hot_text_products), then `-gate1` (`cargo xtask affected --run` under
  ~/bin/gate 16G), then `-xtest1` (`cargo xtask test`); and `-plants` (handoff/HEAP-ADDRESS-SPACE-6-plants.txt,
  dir `/dev/shm/claude/HEAP-ADDRESS-SPACE-6/plants-1`). Superseded: `heap-address-space-6-fullhot0` (inherited head,
  reproduced run -5's red: pre 0x800 rebind 1).
- Notes for the report: no `cargo xtask precheck` exists on next (checked xtask/src/main.rs on origin/next); one
  no-op heredoc call slipped (a `python3 - <<X` with an empty body, ran nothing): record it as a rule slip.
  Run -5's plants: 7/7 CAUGHT (`heap-address-space-5-plants`). SECURITY-PRIVATE: none.

## Exact next step (run -6, after the refresh)

`. ~/src/liella-ops/tools/vm_ssh.sh; vm_ssh_cmd keke; "${VSSH[@]}" '~/bin/jobwait heap-address-space-6-fullhot1
heap-address-space-6-gate1 heap-address-space-6-xtest1 heap-address-space-6-plants'` (call again on exit 3). Check:
fullhot1 has `text phase pre 0x800: rebind 0` and 26 passed (also grep `Resolution10same_text` in
`~/w/n3/target/product/release/layout-bins.tsv`: bin 1, not 0); gate1 green; xtest1 green; plants 3/3 CAUGHT (copy
the results file to handoff/HEAP-ADDRESS-SPACE-6-plants.txt). Red: find the cause, fix, rerun only the failing job.
Then write handoff/reports/HEAP-ADDRESS-SPACE-6.json (thread_common shape plus `jobs`; impact: correctness = the
0x800 rebind fault fixed; perf/size none; coverage = 3 plants and one planner assertion; address space: issue #21
bounds tightened against next, malloc1 +16448 -> +4420, alloc +32836 -> +28996, refill +49220 -> +39236, realloc
+67652 -> +41288, need 67652 -> 42000 KiB, measured by run -5), commit, push, `python3
/home/claude/src/liella-ops/bin/ops handback-check HEAP-ADDRESS-SPACE-6 --head <sha> --jobs "<JOBS>"` until exit 0,
then hand back (COMPLETE: issue #21 targets not reached, next: HEAP-SPILL-COLD; JOBS: keke:heap-address-space-6-lint1,
-fullhot1, -gate1, -xtest1, -plants; SUPERSEDED: -fullhot0). Leave nothing running (`~/bin/kill-mine n3 --dry-run`).

## Next steps for the story, in order

1. HEAP-SPILL-COLD: the spill as a cold extension of grow_segment (never a parameter of alloc_small/alloc_slow);
   then tighten refill/realloc/AS_HEADROOM to 33092 / 35144 / 35152 and recheck a smaller SEG_FIRST.
2. Earlier items still open: relro by-section comparison (ruling #6325 step 1); cbench_malloc / cbench_startup
   counters; returning empty segments' address space; findutils `find` under the issue's `ulimit -v` formula.
