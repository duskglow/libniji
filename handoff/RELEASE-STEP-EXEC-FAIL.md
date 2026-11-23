# RELEASE-STEP-EXEC-FAIL: the CI release step's failed counted runs (exit 127, exit 99)

State: branch `task/release-step-exec-fail`, HEAD = the commit that adds this note (code: bc886b1ac + e01b84ea0),
base origin/next 0495c4684. Kind: investigate. Decisions in force: none new (test-harness change only).
Verdict: the release-step exits (127, 99) are **not established**. The **gate red on the direct tier IS established**:
it was next's, caused by a host change (fixed by LOCALE-PER-DIR-LOAD; the owner also restored the host's archive at
18:10Z, after which this branch's gate is green).

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

## Host change at 18:10Z (coordinator note on flag #8169)

The owner restored mei's `/usr/lib/locale/locale-archive` (en_US.utf8 only) at 18:10Z: the gate red above is gone on
the host (gate3 green). The per-directory measurements above were taken before 18:10Z.

## Jobs (mei, role s3; final code = bc886b1ac + e01b84ea0, later commits are handoff/ only)

* `release-step-exec-fail-gate3` (after 18:10Z): direct tier 35 passed, 0 failed; peak 2.0G.
* `release-step-exec-fail-plants-3`: 3/3 CAUGHT, no WARN.
* `release-step-exec-fail-release3` (after 18:10Z): the CI release pair exactly (`NIJI_FULL_PROFILE=1 cargo xtask so &&
  NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`). `-release2` (before 18:10Z, per-directory host):
  35/35 ok.
* Superseded: `-gate` (ran beside plants), `-gate2` (34/1: the host-locale red above, before 18:10Z), `-plants` (baseline
  bug fixed in e01b84ea0), `-plants-2` (recorded tree content differs from this head), `-release` (script rc = its
  namespace arm, where `perf` records nothing; host arm 35/35). Diagnostics only: `-iso`, `-loc`, `-tipslow`, `-instr`.

## Exact next step (coordinator)

Land this branch; LOCALE-PER-DIR-LOAD (re-gating solo since 18:10Z) keeps `call_dir` + `run_failure` when it merges
next (fixed-length name, e.g. `/tmp/n-<pid:010>-<n:03>`, tested as leg 1's `fixed.rs`: 8/8 + 2/2 green), so any further
127/99 prints its stderr. Note: on the tip each counted run goes through `setpriv --pdeathsig KILL -- unshare --user
--map-current-user --pid --fork --kill-child --mount-proc -- unshare -r -m sh -c ...`; `setpriv` and `unshare` also
exit 127 when they cannot exec the next program, so 127 alone does not mean `page_faults` was missing.
Earlier coordinator scope (train 953 empty-main bound): LD-START-MAPS +2422 of next's 2972 headroom (leg 1 table, report).

Scratch (mei ~/work/release-step-exec-fail/): iso.sh, loc.sh, rel.sh, tip.sh, slowbin/, orig/fixed/next .rs, logs.
