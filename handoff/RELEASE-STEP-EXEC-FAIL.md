# RELEASE-STEP-EXEC-FAIL: the CI release step's failed counted runs (exit 127, exit 99)

State: branch `task/release-step-exec-fail`, HEAD = the commit that adds this note (code: bc886b1ac), base origin/next 0495c4684.
Kind: investigate. Verdict: **cause NOT established**; the branch makes the next failure name its cause and removes a
latent race. Decisions in force: none new (test-harness change only).

## What the CI logs say (trains 951 tip 0fabf6a8f, 952 tip 6f9fd5047)

* Both trains contain `task/locale-per-dir-load` (LOCALE-PER-DIR-LOAD), not on next. It changed `faults_env` in
  `crates/core/tests/hot_text_products.rs`: each counted run is now
  `unshare -r -m sh -c 'mount --bind "$1" /usr/lib/locale || exit 99; ...; PATH=$2; exec page_faults relay "$mode"'`.
* So **exit 99** (952, `init_data_clusters_hold_at_every_text_phase`, cold-800) = the bind mount of
  `<call dir>/locale` failed, and **exit 127** (951, `init_dynamic_startup_only_program_stays_within_its_page_fault_bound`)
  = `sh` could not exec `page_faults` from the call directory (file missing, or for that dynamic program its
  interpreter `/tmp/niji-hot-ldso-<pid>/...` missing). The assert printed only the status, so the logs cannot say
  which or why.
* The fast pass runs the same file under nextest (one process per test); the release step runs `cargo test`
  (all 33 tests in one process, one pid).

## Hypotheses tested

| hypothesis | result | evidence |
|---|---|---|
| (a) `rm -rf target/niji target/niji-test` before the gate | not reproduced | repro 1: cleared, fast `so`, release `so`, release test x3 on tip 6f9fd5047: 3/3 green |
| (b) /mnt/zram 91-93% full | excluded by construction | every path the failing runs touch (`/tmp/nht-<pid>-<n>/{page_faults,locale}`, `/tmp/niji-hot-ldso-<pid>`) is on the gate wrapper's private tmpfs `/tmp` (189G, 1% used), not on zram. My role (`~/w/s3`) is on zram too, but ran after the 17:05Z cleanup, so my runs do not test the full-disk state |
| (c) pad objects without `.note.GNU-stack` | excluded | 70 such ld warnings in every green run; the 951 failure links no pad object |
| (d, mine) `faults_env`'s `<pid>-<call % 10>` directory reused by a later call while an earlier one runs | real latent race, **not shown to fire** | instrumented tip: 10 release runs (4 with the CI condition, `target/product/ld-prefix` rebuilt beside the counts; host load up to 42 on 48 threads): 0 reuses; 67-68 calls per run; at most **5** later calls started during any call (10 needed) |

## Not tested (next step if it recurs)

The CI step runs inside the same gate invocation as the 25-minute full tier (same private `/tmp`, possible leftover
processes), on a host loaded by other jobs. With this branch landed, the failure message carries the run's stderr
(`mount: ...` or `sh: page_faults: not found`), which names the cause. Probe-settled: nothing (no reference probe).

## The change (bc886b1ac)

* `run_failure`: a failed counted run panics with its status and stderr. Test `harness_failed_run_reports_its_stderr`.
* `call_dir`: every `faults_env` call gets its own directory (`/tmp/niji-hot-t-<pid>-<nnnn>`, 31 bytes as before, so
  counts do not move). Test `harness_fault_call_dirs_are_never_shared`.
* Plants: `handoff/RELEASE-STEP-EXEC-FAIL-plants.txt` (3).
* LOCALE-PER-DIR-LOAD rewrites the same `faults_env` lines (its dir `/tmp/nht-<pid>-<n>`, exec path `<dir>/page_faults`,
  33 bytes): when it merges next it keeps `call_dir`/`run_failure` with its own fixed-length name
  (e.g. `/tmp/n-<pid:010>-<n:03>`, same 21-byte dir length as `/tmp/nht-<pid>-<n>`; tested on tip 6f9fd5047 as
  `fixed.rs`: 8/8 release runs green).

## Coordinator scope (16:11Z): `init_dynamic_empty_main_stays_within_its_instruction_bound`, fast, bound 173600

Skylake-SP host, my role (`/mnt/zram/claude/w/s3`), fast products, `cargo test ... -- --nocapture`, 2 runs each, every
run identical. "next" = origin/next 0495c4684 plus this branch's test-only change (the products are next's). Each
candidate = `git merge origin/task/<b>` onto next (all clean), applied alone (job `release-step-exec-fail-instr`).

| tree | instructions | delta vs next | headroom to 173600 |
|---|---|---|---|
| next | 170628 | - | 2972 |
| next + LD-START-MAPS (e3fa3d83a) | 173050 | **+2422** | 550 |
| next + THREAD-CREATE-COST (7cbd370b4) | 170631 | +3 | 2969 |
| next + SHSTK-OPT-IN-2 (90fdd8142, fast-forward of next) | 170990 | +362 | 2610 |

next alone is under the bound (not an environment/host finding). The three deltas sum to +2787 (173415, 185 under
the bound) if they add; train 953 read 173794 in ci-z, so its other stories (or the CI tree) add the rest, about +379
over the sum. Not measured: the 953 tip itself and its other five stories. LD-START-MAPS uses 81% of the headroom.

## Scratch (mei, ~/work/release-step-exec-fail/)

repro1-4 scripts and logs (r1, r2, r3d, r4), the tip variants (orig/detect/detect2/fixed.rs), candidate patches and
`instr.sh` (r3). Nothing deleted in ci-z or other roles.
