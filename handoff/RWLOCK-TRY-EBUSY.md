# RWLOCK-TRY-EBUSY: rwlock try-locks return EBUSY when the caller holds the write lock (issue #15)

State: branch `task/rwlock-try-ebusy`, code commit 522d82446 (on origin/next 575214f4b); this note and
`handoff/reports/RWLOCK-TRY-EBUSY.json` in the commit after it. Role mei:~/w/s3.

## The change

`crates/rt/src/pthread_sync.rs`, `RwLock`: `read` and `write` share one entry check, `enter`:
1. a timed or clock form validates its deadline first: `tv_nsec` outside [0, 1e9) is `EINVAL` whatever the lock
   state, also on a free lock or one the caller holds (an unknown clock was already `EINVAL` first, in the boundary);
2. the blocking, timed and clock forms report a lock the caller holds for writing as `EDEADLK` (unchanged);
3. the try forms skip that check, so a write lock the caller holds is busy like any other: `EBUSY`.
The deadline check moved out of `wait_change` (it ran only when the call had to wait). No fast-path cost: one
compare of the `LockWait` tag before the existing writer-tid load.

Decision record: `records/pending/RWLOCK-TRY-EBUSY-1.toml` (D-NEW, series D-22; CI assigns the id). No earlier
record chose `EDEADLK` for the try forms (D-22.4 and D-22.12 do not mention it), so nothing is amended.

## Probe (black box, reference glibc 2.39 on the build host vs libniji, static link)

Matrix: 8 lock configurations (static initialiser, `init(NULL)`, `setkind_np` 0/1/2, each private and
process-shared; `setkind_np` accepts 0..2 only, -1 and 3 are `EINVAL` on both) x caller holds nothing / a read
lock / the write lock x 18 call forms (`rdlock`, `wrlock`, both try forms, timed forms with past, future and
`tv_nsec` -1 deadlines, clock forms with MONOTONIC and REALTIME past deadlines, clock 99, `tv_nsec` -1), 432 cells,
each in its own child (a call still blocked after 2 s counts as "blocks": only `wrlock` with a read lock held, on both).
- Before (probe-before): 96 cells differed: the 16 try cells (`EBUSY` vs `EDEADLK`) and 80 bad-`tv_nsec` cells
  (reference `EINVAL` first; libniji took a free or read-held lock (0) or said `EDEADLK`).
- After (probe-after1): every one of these cells equal; the issue's reproducer prints `16 Device or resource busy`
  on both. `tv_nsec` = 1e9 is `EINVAL` on both too.

## Tests

- `conformance/c/s22_rwlock_self.c` (clause s22_5_c_2, run by `crates/core/tests/s22_threads_clauses.rs`): every
  non-blocking cell above (8 configurations x 3 holds x 15 forms, the blocking `wrlock` on a read-held lock left
  out), plus the issue's reproducer word for word; a 60 s alarm is a hang guard only (names the cell).
- `crates/rt/tests/pthread_sync.rs` `rwlocks_share_readers_exclude_writers_and_detect_self_deadlock`: the two try
  forms on a write-held lock are `EBUSY`.

## Jobs (mei)

- rwlock-try-ebusy-lint-1: `cargo clippy --workspace --all-targets --all-features -D warnings`, `xtask clippy`,
  `xtask static`, `records check`: rc 0.
- rwlock-try-ebusy-gate-1: `affected --run --base origin/next`: rc 0; 11 suites, 2010 passed, 0 failed,
  100 ignored; conformance 13 passed; peak 2.8G.
- rwlock-try-ebusy-plants-1: `handoff/RWLOCK-TRY-EBUSY-plants.txt`, 4 plants (try check reverted, through the
  conformance program and through the rt test; deadline check removed; blocking self-deadlock check removed).
  Result: 4 CAUGHT, 0 SURVIVED, 0 WARN (the two conformance plants print "caught by no test the log names": the
  failing program is a conformance run, not a cargo test; the msg lines name it).
- rwlock-try-ebusy-bdb-1: Berkeley DB in a swapped trixie root: `tools/deb/build.sh --flavour system` from a
  scratch-repo snapshot of the role, then `tools/deb/swap-system.sh` with `SWAP_EXTRA_PKGS=db5.3-util` and one
  extra command (`db5.3_recover -e` on a fresh directory, i.e. an environment with locking, then `db5.3_load`,
  `db5.3_dump`, `db5.3_stat -c` in it). Result: rc 0, `BDB: PASS`: swap PASS (libc6 2.39+niji20261117.1-1 is the root's libc.so.6 and loader),
  `swap: PASS bdb-env: same output and status (0)`, output ends `lockstat ok` / `end 0`; `swap: RESULT same=57
  differ=7`, the 7 being ldd-1..7, the known loader-trace differences (RELEASE-SNAPSHOT-NIJI1).
- Evidence only: rwlock-try-ebusy-probe-before, rwlock-try-ebusy-probe-after1 (scripts in mei:~/work/RWLOCK-TRY-EBUSY/).

## Open / next

- Nothing open for this story. The "before" Berkeley DB failure is the issue's report; this run proved only the
  fixed side (the probe shows the cause before the fix).
- Do not redo: the probe matrix (above); the reference accepts only kinds 0..2.
