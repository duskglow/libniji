# STARTUP-GOT-PAGE: start-up reads no GOT entry; it reads RELRO's first window once, at a fixed point

State: branch `task/startup-got-page-2` (continues `task/startup-got-page`), cut from origin/next 923df67e9.
Commits: 341f4f7d3 (relaxable test/binop GOT operands), 9be5c2f79 (D-X.122 + plants), cb4eeb062 (start-up RELRO
read, run 2), e985d991a (D-X.122 amendment + plants file), then this handoff and the report. Role keke:~/w/n1 (Zen 1
host). Decision `records/decisions/D-X/D-X.122.toml` (amends D-X.101 and D-X.15). Plants:
`handoff/STARTUP-GOT-PAGE-2-plants.txt` (5: the four of run 1 and `sgp2-no-relro-read`). Scratch scripts (keke):
`~/work/startup-got-page-2/{proof2,relay,gate2}.sh`, run 1's `~/work/startup-got-page/{gotprobe,pfaddr}.sh`.

## What the branch does
* `xtask/src/copies.rs` (run 1): D-X.15 step 3 also marks 64-bit `test`/binop GOT operands `R_X86_64_REX_GOTPCRELX`,
  so the static link turns them into immediates; start-up reads no `.got` and `.got`'s place no longer moves a
  written start-up page (STARTUP-DATA-WINDOWS 7 on next and on LOCALE-SLOTS-GROWTH-5 merged, which reads 8 without it).
* `crates/boundary/src/runtime.rs` (run 2, ruling #6541 option B): `protect_relro` (inlined into `runtime::start`, an
  existing start-up function) reads RELRO's first byte once before its `mprotect`. RELRO starts on a 64 KiB boundary
  (`.data.rel.ro` aligned 65536), so that read fault maps the first window at a fixed point; before, the `.got` read
  did it by luck, and without either the first relro-reading call paid it (first `%.2f`: 4 > FLOAT_BOUND 3).
* Tests: `init_startup_data_pages_hold_at_every_data_phase` (run 1, relro pads move `.got`) and
  `init_first_float_conversion_stays_within_its_page_fault_bound` (the read's effect, plant `sgp2-no-relro-read`).

## Measurements (run 2; keke, fast products, static probe, ASLR off; base / + relaxation alone / + relaxation and read)
Jobs `startup-got-page-2-proof-next`, `-proof-lsg5` (scratch clones, nothing pushed), `-relay` (10 relayed runs).
| base | STARTUP-DATA-WINDOWS | relayed true | tool | fork | first %.2f | instr:u true | instr:u tool |
|---|---|---|---|---|---|---|---|
| origin/next 18cf8293f | 7 / 7 / 7 | 11 / 10 / 11 | 43 / 43 / 43 | 5 / 5 / 5 | 3 / 4 / 3 | 239063 / 239139 / 239085 | 295514 / 295543 / 295534 |
| LSG-5 1ed232a5a | 8 BROKEN / 7 / 7 | 11 / 10 / 11 | 43 / 43 / 43 (one 44) | 5 / 5 / 5 | 3 / 4 / 3 | 239073 / 239059 / 239057 | 295978 / 295971 / 295969 |
Per-fault records (`perf record`, env -i LANG): true 15, tool 42, float 17 in all six builds. Layout-bins (next pair):
only `runtime::start` changes, 5902 -> 5926 B in bin 0 (the read is one 3-byte `movzbl`; the rest is codegen around
it); no section changes bin. `perf stat` "true" page-faults:u means read 81 / 81 / 82 on next, but the probe's own
per-run minflt spreads 100-105 there, so the relayed counts above are the comparison. C (page-aligned `.got`) was
not built: B is at base level and its instruction delta is inside the build-to-build spread.

## Run 2, third fix: a compared immediate is not a start-up data touch (b91d73666)
Run 1's gate (and run 2's affected gate, job `startup-got-page-2-gate`) failed
`init_written_startup_objects_are_in_the_data1_cluster`: `_IO_2_1_stdin_ (224 bytes) in .bss._IO_2_1_stdin_`.
Cause: `tools/hottext/startup_data.py` declares start-up data from the start-up functions' operands, `$0x<addr>`
immediates included; the relaxation turned `stdio::parts_first_use`'s `cmp` against the GOT slot of
`&_IO_2_1_stdin_` (an unnamed `.got` byte, left out) into `cmp $<stdin>, %reg`, so stdin became an "rw" key. Fix:
the `$` immediates of `cmp`/`test` are skipped (memory operands still count). Plant `sgp2-cmp-imm-counted`. Not yet
gated (see next step). Run 1's handoff did not list this failure.

## Jobs
* Final-head code: b91d73666. `startup-got-page-2-lint1` (clippy all-targets + static, rc 0) was on cb4eeb062:
  rerun clippy + static on the final head.
* SUPERSEDED: `startup-got-page-2-gate` (killed: pre-fix tree; its affected step FAILED on the data1 test above),
  `startup-got-page-2-plants` (rc 2: my `--timeout 600` timed out the traced baseline; a traced so takes ~1100 s),
  `startup-got-page-2-plants2` (killed: pre-fix tree), `-lint0` (inherited head, rc 0), proofs/relay (evidence,
  above; not gates).

## Open / not to redo
* "true 81 or better" from the ruling note: with B `true` is at base level (relayed 11 = 11; run 1's 82 -> 81 was the
  relaxation leaving `true` with no relro read, which B gives back by design). Report as INFORMATIONAL; not chased.
* Do not re-measure options A or C. Do not move the read into a `#[cold]` function: it runs on every static start,
  so a cold call would cost a text fault.

## Exact next step (leg 2)
1. The role keke:~/w/n1 was BUSY at the end of leg 1: orphaned `tools/hottext/regen.sh` / `gdb ... trace_gdb.py`
   processes from the killed gate (cwd outside the role, so `kill-mine` misses them). List them with the sync's
   BUSY output (it prints PIDs), end them by PID in a separate ssh call, then sync (`sync_role.sh ... keke n1`, rc 0).
2. Start detached on keke: `startup-got-page-2-lint2` (clippy --workspace --all-targets --all-features -D warnings +
   `cargo xtask static`); then `startup-got-page-2-gate3` = `~/bin/gate 16G sh ~/work/startup-got-page-2/gate3.sh`
   (static, affected --base origin/next which escalates to FULL and runs `cargo xtask test`, then
   NIJI_FULL_PROFILE so + hot_text_products); and `startup-got-page-2-plants3` = `cargo xtask plants
   handoff/STARTUP-GOT-PAGE-2-plants.txt --dir /dev/shm/claude/startup-got-page-2/plants-3 --timeout 3600`
   (6 plants). Wait for them (gate ~2 h: FULL affected took 63 min).
3. Write handoff/reports/STARTUP-GOT-PAGE-2.json (impact: correctness (FLOAT_BOUND 4 -> 3 restored on the branch),
   faults (STARTUP-DATA-WINDOWS 8 -> 7 on LSG-5 merged), size (+24 B runtime::start), coverage (6 plants)), push,
   `ops handback-check`, hand back with JOBS lint2, gate3, plants3.
Plan rows: D-X.101, D-X.15, D-X.122.
