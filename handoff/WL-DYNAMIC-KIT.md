# WL-DYNAMIC-KIT: workload gate and per-call counts with libniji linked dynamically

Branch `task/wl-dynamic-kit`, cut from origin/next b2485a7f1. Measurement only: no gate, bound or rule changes kit
(the default of every tool stays static). Full report with the table: `handoff/WL-DYNAMIC-KIT-report.md`.

## State

- `WL_LINK=dynamic` (validation/wl_lock.sh) keys products, kits, per-call binaries and count rows as
  `<sha>-release-dyn`; `wl_prep.sh`, `wl_count.sh` and `wl_percall.sh` take it. Links go through
  `validation/wl_dyncc.sh` (system `cc`, libniji's start files, `libc.so.6`, `libm.so.6` and `ld.so`);
  `validation/wl_dynlink.py` retargets the by-path `DT_NEEDED` in place (no `LD_LIBRARY_PATH`, which would reach the
  host `sh`/`env` of the workload commands; the loader takes `libc.so.6` only from `LD_LIBRARY_PATH` or its install
  directory).
- `validation/wl_kits.sh SHA` measures glibc / static / dynamic interleaved on one head and host, plus each program's
  per-start cost and start count, and prints the table (`wl_kits.py`).
- Fixed on the way: the workload kits did not link on the Zen 1 or Skylake-SP build hosts (object trees still under
  their pre-rename names; recipes calling the pre-rename driver); release count rows were never cached
  (`wl_count_run.sh` made the wrong directory).
- Self-test: `wl_lock_test.sh` sections 6-7 (41 checks on the Skylake-SP host, `cc` present).

## Measured (b2485a7f1 products, release profile, Skylake-SP build host, 5 rounds, medians)

- Geomean vs glibc, instructions:u: static 0.818, dynamic 0.902 (0.862 if libniji's per-start cost matched glibc's).
  Page faults: static 0.978, dynamic 1.151. task-clock (advisory, unbooked, loaded host): static 0.854, dynamic 0.909.
- One start (`<prog> --version`): dynamic 1.04-2.00M instructions vs glibc 0.27-0.87M vs static 0.07-0.57M;
  faults dynamic 111-170 vs glibc 74-112.
- Dynamic instructions > 1.10: printf-loop 3.646 (20000 starts, 100 % start-up); > 1.00: 9 (wc, md5sum, printf-loop,
  grep-F 1.035, grep-E, diff, tar-c, gzip-c, tar-gz; all but printf-loop and grep-F within 0.5 %).
  Static > 1.00: grep-F 1.011, grep-E, diff (0.0 %).
- Dynamic faults > 1.10: 22 of 30 (about +45 per start); static: 1 (ls-l-enUS 1.143).
- Per call (builtin spec): dynamic = static + 1 instruction per libc call (the PLT jump; `-z now` as for glibc).

## RULING NEEDED (owner): which kit the INVIOLABLE rule binds

Recommend switching the per-gate measurement to dynamic (replace, not add), in three phases. Details and costs are
in the report, "Switch plan".

## Remaining in this run (refresh leg, 2026-11-20 ~10:45Z)

All code and the measurement are done and pushed (code head a5898fd57; acc2ab964 and later commits are handoff/ only).
Left: (1) `~/bin/jobwait wl-dynamic-kit-gate wl-dynamic-kit-lint` on the Skylake-SP build host (role s2; the gate is the
direct tier and exits 7 if `affected --list` says FULL: then write `FULL: the train's` and use precheck + lint); both
run on the synced a5898fd57. (2) Sync the role to the final head (sync_role --story WL-DYNAMIC-KIT --base origin/next).
(3) On the role: `cargo xtask precheck --jobs wl-dynamic-kit-gate,wl-dynamic-kit-lint,wl-dynamic-kit-kits3,wl-dynamic-kit-percall
--head <final sha>` (a jobrun job). (4) Set `head` and `gate.affected` in handoff/reports/WL-DYNAMIC-KIT.json, push,
then `ops handback-check WL-DYNAMIC-KIT --head <sha> --jobs "<JOBS>"` and hand back COMPLETE with the RULING NEEDED line
from the report. Nothing else of this run is running on the host besides those two jobs.

## Next step

The coordinator takes the ruling. If yes: one story flips the defaults (`WL_LINK` default dynamic in wl_lock.sh, the
gate's recorded levels re-recorded dynamically) and adds a nightly `wl_kits.sh` run; DYN-START-SPEED owns
printf-loop and the fault rows.

Not to redo: the dynamic link method (the by-path and retarget approach, `-rpath-link` for libm's libc) is settled and
tested; the per-call PLT cost (+1) is measured.
