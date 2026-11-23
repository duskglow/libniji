# RELEASE-STEP-EXEC-FAIL: the CI release step's failed counted runs (exit 127, exit 99)

State: branch `task/release-step-exec-fail-2` (run 2; continues `task/release-step-exec-fail` 52c9912ec), HEAD = the
commit that adds this note; code = the merge ad8438b75 of origin/next 81bdd4b45 (LOCALE-PER-DIR-LOAD landed).
Kind: investigate. Decisions in force: none new (test-harness change only).
Verdict: the release-step exits (127, 99) are **not established**; the one race that gives exactly them is closed and
any further failure of a counted run now prints its stderr. The direct-tier gate red of run 1 was next's, caused by a
host locale change (fixed by LOCALE-PER-DIR-LOAD; the owner also restored the host's archive at 18:10Z).

## Run 2: merge of next (ruling #8177)

* Conflict only in `crates/core/tests/hot_text_products.rs` (both rewrote `faults_env`). Kept next's namespace script
  and probe locale archive, and on top of it this branch's two harness changes:
  * `call_dir`: next still used `/tmp/nht-<pid:010>-<call % 10>` (ten rotating slots, one pid under `cargo test`).
    Now `/tmp/n<pid:010>-<n:04>`: own directory per call, **21 bytes, next's length**, so the exec path
    `<dir>/page_faults` (found through the script's PATH) and the bounds measured with it do not move.
  * `run_failure` replaces next's status-only assert: a 99 (bind mount) or 127 (`unshare`/`setpriv`/probe not
    executable) now names its step through its stderr.
* Plant `calldir-width` now expects 21 bytes. The use of `run_failure` inside `faults_env` has no plant: only a
  failing counted run reaches it (the helper itself is planted by `failure-stderr`).

## Established in run 1 (kept short; flag #8169, accepted)

* next 0495c4684 failed `init_data_clusters_hold_at_every_text_phase` on the Skylake-SP CI host after its 06:32Z
  `apt install locales-all` (per-locale directories, no archive): `tool` 58/59 by text pad; with an en_US archive in a
  private mount namespace 45 at every pad, pass. LOCALE-PER-DIR-LOAD's probe archive removes the dependence.
* The 127/99 exits come only from LOCALE-PER-DIR-LOAD's namespace script; the `% 10` directory race gives exactly
  them. Not reproduced in 13 + 10 instrumented release runs nor with `localedef` slowed by 20 s (at most 5 later calls
  started during any call). `rm -rf target/niji*`, zram space, GNU-stack warnings: excluded or not reproduced.

## Jobs (mei, role s3, final code ad8438b75; later commits are handoff/ only)

* `release-step-exec-fail-2-gate`: direct tier (niji::hot_text_products), 35 passed, 0 failed; peak 3.1G. No probe
  NOTE in the log: the counted runs went through the namespace path with the probe archive.
* `release-step-exec-fail-2-release`: the CI release pair exactly (`NIJI_FULL_PROFILE=1 cargo xtask so &&
  NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`): 35 passed, 0 failed.
* `release-step-exec-fail-2-plants`: 3/3 CAUGHT (calldir-rotate, failure-stderr, calldir-width), no WARN
  (`handoff/RELEASE-STEP-EXEC-FAIL-plants.txt`).
* Clippy (workspace, all targets, all features, -D warnings) and `cargo xtask static`: clean on the inherited head and
  on ad8438b75.

## Exact next step

Land this branch (the train is its FULL gate). If a release step fails a counted run again, its panic now carries the
stderr: read it before anything else. Note: each counted run goes through `setpriv ... unshare ... unshare -r -m sh -c`;
`setpriv` and `unshare` also exit 127 when they cannot exec the next program, so 127 alone does not mean the probe
was missing.

Scratch (mei ~/work/release-step-exec-fail/, ~/work/release-step-exec-fail-2/): run-1 scripts and logs, run-2 lint logs.
