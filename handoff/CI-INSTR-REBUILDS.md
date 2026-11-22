# CI-INSTR-REBUILDS: why ci-instr rebuilds the release libraries (handoff, leg 1)

Branch `task/ci-instr-rebuilds`, base origin/next 5a9b5d0a1. Commits: 4cd8a106b (fix), 5fb6babe5 (plants file),
then the D-27.78 record (two commits). Role: mei:~/w/s1 (Skylake-SP). ci-instr itself runs on keke (Zen 1 host,
role ci-instr).

## Cause (found)

* `NIJI_BUILD_TAG` (xtask/src/so.rs `source_hash_tag`, D-27.22) was a hash over EVERY file under `crates/`, `csys/`,
  `include/` plus `Cargo.lock`. `niji-boundary` and `niji-ld` read it with `option_env!`, and every release library
  builds on them, so any train touching a test file, bench or non-product crate (mathtools, native, spectest...)
  rebuilt ld, libc, ldconfig, libm and libresolv.
* Most rebuilds are still REQUIRED: the gate logs (mei ~/logs/ci-gate-<sha>-<t>.log, "changed files:" vs base)
  show a product source in most trains (e.g. 90390071c changes crates/abi/src/layout.rs). Per train (vs its base):
  of 52 rebuilt runs since 11-19, 6 had no product file at all (0add1dd07, 8e34f9c72, 08084029d, 33d98c85d,
  5a9b5d0a1, 0299fd6e1). Consecutive-tip diffs were NOT taken (git history of others' commits is off-limits), so the
  exact count of tag-only transitions is an estimate. Scratch tables: ~/w/.scratch/CI-INSTR-REBUILDS/{runs,changed}.txt,
  classify.py (local).
* Not the cause: sync (rsync -c, no mtimes; checked no source file is newer than .synced after a build); role reset
  (sync_role removes target/niji only, never target/product); toolchain (pinned 1.99.1 in both roles).

## Fix (done, unit-tested)

Tag = Cargo.lock, csys/, include/, crates/Cargo.toml, crates/Cargo.lock, and the crates the 5 product manifests reach
by [dependencies]/[build-dependencies] (target tables too, never dev-deps), without tests/ benches/ examples/.
`check_tag_covers` after each product cargo build: every dep-info input (and absolute build-script watched path)
inside the workspace must be covered, else the build fails. `cargo test -p xtask so::tests` 30 passed; clippy -p xtask clean.

## Jobs running at the end of leg 1

* mei `ci-instr-rebuilds-plants` (9 plants, handoff/CI-INSTR-REBUILDS-plants.txt, dir /mnt/zram/claude/ci-instr-rebuilds/plants-1), on head 5fb6babe5 (code = final).
* keke `ci-instr-rebuilds-exp`: waits for the live ci-instr-90390071c job, copies ~/w/ci-instr to
  /dev/shm/claude/ci-instr-rebuilds/c1, runs ~/work/ci-instr-rebuilds/exp.sh (bind mount at ~/w/ci-instr in
  `unshare -r -m`, NIJI_FULL_PROFILE=1, CARGO_LOG=cargo::core::compiler::fingerprint=info): r0-control (copy as is,
  expect all cargo steps 0 s), r1-next (sync 5a9b5d0a1 content: expect abi rebuilt, file changed), r2-testonly-old
  (append comment to crates/core/tests/hot_text_products.rs, OLD xtask: expect rebuild, boundary dirty for env
  NIJI_BUILD_TAG), r3-newtag-first (sync branch content: one-time rebuild), r4-testonly-new (same edit: expect 0 s),
  r5-same-tip (expect 0 s). Logs c1.logs/<name>.log; summary in ~/logs/ci-instr-rebuilds-exp.log.
* A lint job (ci-instr-rebuilds-lint) was started on a stale sync and killed (SUPERSEDED, not a result).

## Exact next steps (leg 2)

1. mei: `~/bin/jobwait ci-instr-rebuilds-plants`; results.txt in the plants dir: all CAUGHT (list WARN lines).
2. Sync (rc must be 0; rc 4 = busy), then on mei one job: `cargo xtask records check && cargo clippy --workspace
   --all-targets --all-features -- -D warnings && cargo xtask static`; then `cargo xtask affected --list` (FULL? then
   precheck + `cargo test -p xtask` as kind test) and the direct gate `ci-instr-rebuilds-gate`.
3. keke: `~/bin/jobwait ci-instr-rebuilds-exp`; quote the fingerprint lines (`grep -iE "dirty|stale" c1.logs/r*.log`)
   and the step lines per run in the report.
4. Step 3 of the brief (reuse other products): libm.a 05e5943362eb and libresolv.a 20629075c406 were byte-identical
   on mei ci-z and keke ci-instr; keke 90390071c raw archives: libc.a 35094cf142ab, libld_so.a b39e7376f3c7,
   libldconfig_bin.a ce2b7cc1ba45 (md5 prefixes, scratch md5-1.txt). Compare with mei:~/w/ci-z/target/product once its
   gate for 90390071c (or any tip both built) has run its release step. Then the train's release build runs only
   at the END of the full gate (db.py ci_gate_cmd) on the Skylake-SP host, while ci-instr runs beside the gate:
   waiting for it would make ci-instr end later, not sooner; the traced layout is per host anyway (ci-instr
   would still trace). Check gate wall times in mei ~/logs/jobs.tsv (ci-gate rows) against ci-instr's to confirm.
5. Report: handoff/reports/CI-INSTR-REBUILDS.json, precheck --jobs <labels> --head <sha>, handback-check.

## Informational

* The traced layout (`so/libc-archive` step) costs ~588 s in EVERY ci-instr run, cargo reused or not: larger than
  the rebuild; a candidate story (trace-at-build on the Zen 1 host).
* `build_tag(root)` is computed about 8 times per `cargo xtask so` (each a full walk + read); minor.
