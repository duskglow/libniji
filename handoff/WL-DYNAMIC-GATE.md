# WL-DYNAMIC-GATE: the INVIOLABLE workload rule binds the dynamic kit (owner ruling #6965)

Branch `task/wl-dynamic-gate`, stacked on WL-DYNAMIC-KIT (its tip 98268cb83; merge base with next b2485a7f1).
Role keke:~/w/n3 (Zen 1 host, CPU class "AMD EPYC 7551 32-Core Processor"). Decision D-X.141 (reserved, written).
Plants file: `handoff/WL-DYNAMIC-GATE-plants.txt`.

## Done (leg 1)

- `validation/wl_lock.sh`: `WL_LINK` defaults to dynamic; `wl_build_products` caches the OTHER link's set from the same
  build (one `cargo xtask so` per sha); `wl_link_set`, `wl_prodfile`. wl_count/wl_percall/wl_prime_base check the
  product file of the run's link. `wl_gate.sh` (timing hosts) pins `WL_LINK=static` on every step (dynamic kits name
  products by absolute build-host path; the copy to timing hosts does not carry them).
- `validation/wl_verdict.py` (check: per row <= 1.10x glibc in instructions:u and page faults, held rows at their
  level, geomean instructions:u <= 1.00 on a full run, HOLD-RELEASE / MAY-TIGHTEN / HELD-UNRECORDED; holds: the
  ratchet against the recorded measurement and the merge base's copy). `wl_count.sh` runs it at the end and fails
  on FAIL for a dynamic release run without WLC_ENV (advisory otherwise).
- `validation/wl_holds.tsv`: header only so far (TO FILL, see next step).
- `validation/wl_verdict_test.sh` (32 checks, passes locally), `wl_lock_test.sh` section 8 (default dynamic, timing
  gate pins static) and 8b (cc: one build caches both sets; not yet run: no cc locally). affected.rs maps
  wl_verdict.py, wl_holds.tsv, wl_count.sh and the test to VALIDATION_VERDICT_TEST.
- `wl_kits.sh`/`wl_kits.py`: the nightly/RC static sweep; prints the rule's PASS/FAIL for both kits (no holds).
- Docs: validation/README.md section "The INVIOLABLE rule binds the dynamic kit", wl_gate.sh header, D-X.141.

## Jobs running on keke at refresh (started by leg 1)

- `wl-dynamic-gate-count-dyn` (count): `~/work/wl-dynamic-gate/count.sh 189ab9cbc7b9... <all 30 workloads>` from the
  role at code head 7a5d6fc7d (dynamic default; new = branch, base = next tip 189ab9cbc). Its tables are in
  `~/logs/wlc-7a5d6fc7d-133053-2718391.log`; count.sh prints wall time, fs outputs and bytes added under ~/wl
  (the "after" cost; "before" = WL-DYNAMIC-KIT's report: static column ~410 s per full count, +171.8 s second build).
  This run predates the verdict commit, so it prints no verdict: run `python3 validation/wl_verdict.py check <that
  log> --holds validation/wl_holds.tsv --all-count 30` by hand after filling the holds.
- `wl-dynamic-gate-lint1` (test): `~/work/wl-dynamic-gate/lint.sh` (both self-tests, clippy, xtask static) on
  head 4a2b2f3d9 (the verdict commit). It sat QUEUED for a slot from 13:37Z (keke saturated). Superseded by the final lint.

## Next step (leg 2)

1. `~/bin/jobwait wl-dynamic-gate-count-dyn wl-dynamic-gate-lint1` on keke; read the lint log's tail.
2. From the count log, for every workload whose BASE column (next 189ab9cbc) is over 1.10x glibc in instructions:u
   or page-faults, add a row to `validation/wl_holds.tsv`: cpu "AMD EPYC 7551 32-Core Processor", workload, metric
   (instr|pf), glibc, libniji (the base median; use the higher of new and base if they differ, both are next's libc),
   level = ceil to 4 decimals of libniji/glibc, recorded "189ab9cbc 2026-11-20", cause "dynamic start-up". Expect
   printf-loop instr and ~20 pf rows. Check the noise first: if new and base differ for a row, take the higher
   (a level a rerun of next can exceed is a flake). Optionally add the Skylake-SP rows from WL-DYNAMIC-KIT's table
   (b2485a7f1; needs that host's /proc/cpuinfo model name) - say so in the report either way.
3. `python3 validation/wl_verdict.py holds validation/wl_holds.tsv` and the `check` above (expect PASS); commit,
   push, sync; start on keke: lint (reuse lint.sh, new label), gate (`cargo xtask affected --run`, kind gate; direct
   tier, never FULL), plants (`cargo xtask plants handoff/WL-DYNAMIC-GATE-plants.txt --dir /dev/shm/claude/wl-dynamic-gate/plants-1 --timeout 600`).
4. precheck, report json (`handoff/reports/WL-DYNAMIC-GATE.json`), handback-check, hand back with the held rows and
   levels listed, the wall-time/bytes figures, and INFORMATIONAL: the nightly `wl_kits.sh` schedule is the
   coordinator's (ops cron, not in this repo); thread_common.txt's INVIOLABLE line (ops repo, read-only here) should
   name the dynamic kit, ruling #6965.

Not to redo: the design above; the verdict and tests are written and pass locally.
