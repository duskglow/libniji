# GATE-BOUNDED-PARALLEL (handoff, leg 1 -> leg 2)

State: branch `task/gate-bounded-parallel`, head 25b512dea (pushed), base origin/next 189ab9cbc (GATE-DEV-NO-FULL already
landed there). Role keke:~/w/r3, NOT yet synced with the change (it holds origin/next for the "before" measurement).

Done (leg 1):
- `xtask/src/affected.rs` `run_selection`: when a non-FULL run selects both the xtask tests and the products, it runs
  `cargo test -p xtask --bins --test <every xtask target with !t.products>` BESIDE `cargo xtask so` (thread scope,
  `products_beside_xtask_tests`; side log `target/affected-xtask.log`, appended to `target/affected.log`), then
  `cargo test -p xtask --test transcripts` (the xtask targets `names_products` marks: they run target/niji/ldconfig and
  target/niji-test/iconv, so they must see fresh products; before, they ran on the previous build's products).
  Memory guard: `memory_room()` (smallest cgroup v2 `memory.max` on the path, and MemTotal) >= `SIDE_BY_SIDE_BYTES`
  (PLACEHOLDER 12 GiB: set it from the measurement), else products first, then the same tests. No selection changed.
- Module doc links the record; record `records/pending/GATE-BOUNDED-PARALLEL-1.toml` (D-NEW, series D-X): the five
  classes with rationale + the side-by-side rule. Its body ends `Measured (...): MEASUREMENTS.`: fill that in.
- 3 new unit tests + plants file `handoff/GATE-BOUNDED-PARALLEL-plants.txt` (6 plants). NOT YET COMPILED (no local cargo).

Running on keke (started 13:34Z): job `gate-bounded-parallel-before` (kind test) = gate 16G `prof.sh` (warm-up 239 s,
then `cargo nextest run -p xtask` per-test profile -> ~/work/gate-bounded-parallel/nextest.log, /usr/bin/time -v in it)
then gate 16G `meas.sh before layout-ldso-order-3-gate` (target/product removed first; samples cgroup `anon` every 2 s
to anon-before-<label>.txt; run log run-before-<label>.log; prints seconds, anon peak, step lines). Expected end ~14:45Z.

Next steps, in order:
1. `~/bin/jobwait gate-bounded-parallel-before`. Profile: `grep -E "PASS|FAIL|SLOW" nextest.log | sort` by the bracket
   time, top ~10 tests and per-binary totals; also the per-binary `finished in` lines of run-before-*.log (cargo test
   runs the integration binaries one after the other). Fix only what is cheap and safe; name the rest with numbers.
2. Sync: `bash ~/src/liella-ops/tools/sync_role.sh --story GATE-BOUNDED-PARALLEL --base origin/next <worktree> keke r3`
   (check rc; 4 = BUSY). Then on the role: `cargo xtask clippy`, `cargo xtask static`, `cargo xtask records check`,
   `cargo test -p xtask --bin xtask side_by_side the_xtask_targets_that_run the_products_and` (fix compile errors).
3. Set SIDE_BY_SIDE_BYTES = measured side-by-side anon peak + headroom (must stay under the 16G gate limit, or it never
   runs side by side in a gate); fill MEASUREMENTS in the record; commit, push, sync.
4. After run: `setsid nohup ~/bin/jobrun gate-bounded-parallel-after test -- sh -c ". ~/.cargo/env; ~/bin/gate 16G sh
   ~/work/gate-bounded-parallel/meas.sh after layout-ldso-order-3-gate startup-got-page-4-gate" </dev/null >/dev/null 2>&1 &`
   (the console must print `note: the xtask tests that run no products go beside the products build`). A
   startup-got-page-4 "before" row is still missing: run `meas.sh before startup-got-page-4-gate` on origin/next only if
   time allows (needs the role at origin/next: not after the sync), else report it as GATE-DEV-NO-FULL's estimate.
5. Plants (after the timing run, not beside it): `setsid nohup ~/bin/jobrun gate-bounded-parallel-plants plants -- sh -c
   '. ~/.cargo/env; cd ~/w/r3 && cargo xtask plants handoff/GATE-BOUNDED-PARALLEL-plants.txt --dir
   /dev/shm/claude/gate-bounded-parallel/plants-1 --timeout 600' </dev/null >/dev/null 2>&1 &` and the direct gate
   `gate-bounded-parallel-gate` (`cargo xtask affected --run` under ~/bin/gate 16G; it selects the xtask tests, no products).
6. Foreground `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`, then
   `cargo xtask precheck --jobs <labels> --head <sha>`; report `handoff/GATE-BOUNDED-PARALLEL-report.md`,
   `handoff/reports/GATE-BOUNDED-PARALLEL.json` (+ "jobs"), `ops handback-check`.
Hand-back must say: D-NEW pending record (CI assigns the id); premise = GATE-DEV-NO-FULL's split (1,577 s products,
1,118 s xtask tests serial on keke).
