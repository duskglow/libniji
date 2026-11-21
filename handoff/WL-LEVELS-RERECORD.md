# WL-LEVELS-RERECORD: re-record the workload page-fault levels from honest counts

Branch `task/wl-levels-rerecord` (from origin/next 0299fd6e1). Roles: mei:~/w/s1 (Skylake-SP, Xeon 8151, dispatcher's)
and keke:~/w/wllr (Zen 1, EPYC 7551; approved by coordinator ruling #7429: story-owned, counts as jobrun jobs, REMOVE
the dir and any /dev/shm scratch before hand-back and say so). Decision id reserved: **D-X.155** (amends D-X.151).
Scratch: `~/w/.scratch/WL-LEVELS-RERECORD/` (local): `start_counts.sh`, `parse_counts.py`, commit messages.

## Done (leg 1)

- 8822525a4 `validation/wl_stats.sh`: `WL_RUN_ENV='HOME=/ PATH=/usr/bin:/bin LANG=en_US.UTF-8 LC_CTYPE=C.UTF-8'`;
  `wl_run_prefix` prints `env -i $WL_RUN_ENV setarch -R` (WLC_ASLR=1: empty, as before). LANG/LC_CTYPE are the values
  every earlier count inherited from the ssh session (probed on both hosts), so only the env size/contents is fixed.
  dash adds PWD (= ~/wl/work, the same path on every host). Rows cached as `.r.tsv` (`wl_count_run.sh`).
  `wl_stats_test.sh`: two callers with different environments give the measured program the same env (64 passed).
- 927e92c21 `validation/wl_verdict.py holds`: level must EQUAL ceil4(libniji/glibc) of its row (an edited level is
  refused); a raise against the base is accepted only as a correction: cause contains `correction D-X.<n>`,
  `recorded` differs from base, libniji count not above base's (so glibc fell). Tests in `wl_verdict_test.sh`
  (46 passed). Plants: `handoff/WL-LEVELS-RERECORD-plants.txt` (8 code plants; add the holds-row plant, below).
- Counts started 22:39Z, all of next 0299fd6e1 (`--base <sha> --rev <sha>`, COUNT_FRESH=1, so base and new are both
  next, both measured fresh): keke labels `wl-levels-rerecord-k1..k5` (k1, k2 with printf-loop; 21 other held rows
  in all five), mei `wl-levels-rerecord-m1..m5` (21 rows, no printf-loop: no levels exist for Skylake-SP). They queued
  for slots (keke still queued at 23:00Z); m1 started first and builds the products. Each keke count ends rc 1 while the
  old levels are in force (sed-s etc. over level): expected, that is the evidence.

## Next step (leg 2)

1. `~/bin/jobwait` the counts (one call per host, <= 600 s). Fetch logs into scratch (`ssh ... cat ~/logs/<label>.log >
   ~/w/.scratch/WL-LEVELS-RERECORD/logs/<label>.log`), then `python3 -I parse_counts.py logs/k*.log` and `logs/m*.log`:
   prints per workload the libniji/glibc values over the counts (repeatability: "N x10" = identical) and the proposed
   level (highest libniji of new+base / lowest glibc).
2. Edit `validation/wl_holds.tsv` (Zen 1 rows only): every row whose stored glibc number the counts do not reproduce
   gets glibc, libniji, level from the counts, `recorded` = `0299fd6e1 2026-11-21 (fixed env, D-X.155)`, cause
   `dynamic start-up; correction D-X.155` when the level rises; rows that measure at or under 1.10 drop. Update the
   header comment (method: lowest of 5 in the fixed env; recorded = highest libniji / lowest glibc over the 5 counts).
   Run `python3 validation/wl_verdict.py holds validation/wl_holds.tsv --base <next's copy>` and re-run
   `wl_verdict.py check <each keke log> --holds validation/wl_holds.tsv --cpu "AMD EPYC 7551 32-Core Processor"`: all PASS.
3. Add plant `level-edited` on `validation/wl_holds.tsv` (lit: one row's level changed alone) with cmd
   `bash validation/wl_verdict_test.sh`, msg `FAIL wl_holds.tsv`.
4. D-X.155 row in `plan/07-decisions/x-implementation-decisions.md` (after D-X.151; amends it): fixed env, the
   recompute check, the correction rule, the before/after table with old number, honest number, how measured, job
   labels; "corrections under the owner's ruling 2026-11-21, not loosenings". Evidence also in the commit body
   (coordinator: old value, new value, the 5 counts, host and class).
5. Commit, push, sync both roles (`sync_role.sh --story WL-LEVELS-RERECORD --base origin/next <wt> mei s1`), on mei:
   clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`), `cargo xtask static`, gate
   (`wl-levels-rerecord-gate`, `cargo xtask affected --run`), plants (`wl-levels-rerecord-plants`, dir
   `/mnt/zram/claude/wl-levels-rerecord/plants-1`), precheck; report json; handback-check. Remove keke:~/w/wllr.
