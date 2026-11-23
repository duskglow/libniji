# RELEASE-STEP-EXEC-FAIL: the CI release step's failed counted runs (exit 127, exit 99)

State: branch `task/release-step-exec-fail`, HEAD = the commit that adds this note (code: bc886b1ac + e01b84ea0),
base origin/next 0495c4684. Kind: investigate. Decisions in force: none new (test-harness change only).
Verdict: the release-step exits (127, 99) are **not established**. The **gate red on the direct tier IS established**:
it is next's, caused by a host change, and fixed by LOCALE-PER-DIR-LOAD (not by this branch).

## Established (leg 2): next fails `init_data_clusters_hold_at_every_text_phase` on the Skylake-SP CI host since 06:32Z

* mei's `/var/log/dpkg.log`: `2026-11-23 06:32:42 install locales-all` (apt history: `apt install locales-all`).
  `/usr/lib/locale/locale-archive` no longer exists; `/usr/lib/locale` now holds 511 per-locale directories.
* next 0495c4684 was gated at 06:03Z (before it). CI trains with LOCALE-PER-DIR-LOAD (951 14:07, 952 14:57, 29657fdab
  15:43; its probes mount their own en_US archive) PASS the test in the fast pass; the trains without it (c07243b9d 16:09,
  48f597ce3 16:44) FAIL it with exactly next's numbers.
* Fast products of next (my role, test file of origin/next and of this branch alike, 2+2 runs, rc 101 each):
  `tool` 59 at `cold 0x2000`, 58 at the other nine phases.
* Same products, same test, only `/usr/lib/locale` changed in a private mount namespace (`unshare -r -m`, 2 runs each):
  en_US.UTF-8 **archive** (host `localedef --prefix`): `tool` 45 at all ten phases, **pass**; host directories: 58/59,
  fail; only the `en_US.utf8` directory: 38/39, fail. So the +1 follows the per-directory locale form, not the branch.
* The release profile passes it on the host's directories (job `release-step-exec-fail-release`, host arm: 35/35 ok).
* LOCALE-PER-DIR-LOAD's own `faults_env` comment (tip 6f9fd5047) already states this (58/59 by text pad) and fixes it.

## Release-step exits 127 / 99 (trains with LOCALE-PER-DIR-LOAD only): not established

* Only that branch's namespace script exits 99 (`mount --bind <call dir>/locale` failed) or 127 (`exec page_faults`
  from the call dir not found). Its call dirs are `/tmp/nht-<pid>-<call % 10>`; under `cargo test` all tests share a
  pid, so a call outlived by 10 later calls loses its directory (gives exactly 127 or 99). 952 failed on the FIRST
  counted call of its test (cold 0x800 `true`), peak memory 13G of the 16G gate limit.
* Not reproduced: leg 1 13 release runs (+10 instrumented: at most 5 later calls during any call); leg 2 job
  `release-step-exec-fail-tipslow` on tip 6f9fd5047, release products, `localedef` slowed by 20 s (the probe-archive
  OnceLock held while calls queue): tip file 2/2 green, tip + `call_dir` 2/2 green. (a) `rm -rf target/niji*`,
  (b) zram space, (c) GNU-stack warnings: excluded or not reproduced (leg 1).

## The change (bc886b1ac, e01b84ea0)

* `run_failure`: a failed counted run panics with its status and stderr (test `harness_failed_run_reports_its_stderr`).
* `call_dir`: every `faults_env` call gets its own directory, same 31-byte length (test
  `harness_fault_call_dirs_are_never_shared`). Plants: `handoff/RELEASE-STEP-EXEC-FAIL-plants.txt`, 3/3 CAUGHT.

## Jobs (mei, role s3)

* `release-step-exec-fail-gate2` (47546f1f2, code = final): direct tier 34 passed, 1 failed = the test above (next's,
  host locale form); unsafe-audit, csys-audit, clippy, static ok; peak 1.6G.
* `release-step-exec-fail-plants-2` (155df8005, code = final): 3/3 CAUGHT.
* `release-step-exec-fail-release2`: the CI release pair exactly (`NIJI_FULL_PROFILE=1 cargo xtask so && ... cargo test
  -p niji --test hot_text_products`) on the final code.
* Superseded: `-gate` (first run, ran beside plants), `-plants` (baseline bug fixed in e01b84ea0), `-release` (script rc
  = its namespace arm, where `perf` records nothing: `init_startup_data_accesses_are_in_the_data1_cluster`; host arm 35/35).
  Diagnostics, not proof: `-iso`, `-loc`, `-tipslow`, `-instr` (leg 1).

## Exact next step (coordinator)

Land LOCALE-PER-DIR-LOAD and this branch in one train (this branch alone fails the fast pass on mei like every train
without LOCALE-PER-DIR-LOAD); LOCALE-PER-DIR-LOAD keeps `call_dir` + `run_failure` when it merges (fixed-length name,
e.g. `/tmp/n-<pid:010>-<n:03>`, tested as leg 1's `fixed.rs`: 8/8 + 2/2 green). The next 127/99 then prints its stderr.
Earlier coordinator scope (train 953 empty-main bound): LD-START-MAPS +2422 of next's 2972 headroom (leg 1 table, report).

Scratch (mei ~/work/release-step-exec-fail/): iso.sh, loc.sh, rel.sh, tip.sh, slowbin/, orig/fixed/next .rs, logs.
