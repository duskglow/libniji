# TIMED-WAIT-EDGE-ORDER(-2): order of the deadline and tv_nsec checks in timed waits; prio-protect locking

State: branch `task/timed-wait-edge-order-2` on origin/next 0495c4684; code commit 7d6d6207d, this note and
`handoff/reports/TIMED-WAIT-EDGE-ORDER-2.json` in the commit after it. Role mei:~/w/s1 (Skylake-SP). The first run
(TIMED-WAIT-EDGE-ORDER, shiki:~/w/sh1) pushed nothing; its probes (~/work/timed-wait-edge-order on shiki) were reused.
Records: D-22.26 (edge order), D-22.27 (prio-protect), COMPAT-42 (registered); two rows in docs/differences-from-glibc.md.

## Done

- Ruling #7554 (D-22.26), `crates/rt/src/pthread_sync.rs` `Mutex::lock`: an invalid deadline turns the wait into the kind's
  own try path (plain, robust incl. owner-died, PI, PP), EBUSY becomes EINVAL. The PI kernel try-lock's EAGAIN is EBUSY.
  `Sem::wait`: EINVAL for an invalid deadline before taking (as the reference; POSIX allows). `boundary thread.rs join`:
  an ended thread is joined whatever the deadline; the tv_nsec check moved there from `pthread.rs join_clock`.
- #7555 (D-22.27): black-box, the reference refuses a thread's FIRST lock of a PP mutex (EINVAL; main and new threads,
  every form, ceiling 1, RLIMIT_RTPRIO 0) and takes it on the next attempt. Not POSIX's EINVAL case (priority 0 is never
  above a ceiling): libniji keeps taking it; differences row, no compat option (reason in the row).
  `pthread_mutex_setprioceiling` now rejects ceilings outside SCHED_FIFO's range (was: 0 and 100 stored).
- Tests: rt `an_invalid_deadline_is_an_error_only_when_the_mutex_would_block` (8 kinds x 3 bad deadlines x 2 clocks, free
  and held, sem at 1); conformance `s22_past_deadline` checks the free invalid-deadline cells and the 24 PP cells again
  (each locking thread makes one PP attempt first), 204 cells, 0 failed on the reference and on libniji; `s22_more` checks
  the ceiling range.

## Probe tables (`s22_past_deadline --probe`, mei; outputs in mei:~/work/timed-wait-edge-order-2/)

- Before (shiki, first run): 46 invalid-tv_nsec cells + 24 PP cells differ.
- After: 24 cells differ, all the ruled EINVAL case (plain mutex kinds x5 forms and `mtx_timedlock` x2 held, joins x3
  running; {-1, 1e9} and {-1, -1}); the other 22 equal the reference; PP cells equal after the one-attempt warm-up.
- pp3 probe (same dir): first-lock refusal per thread; setprioceiling range; PI trylock held = EBUSY in both.

## Open

- INFORMATIONAL (pre-existing, not changed): the ceiling of a fresh mutex attribute, and of a PP mutex whose ceiling was
  never set, reads 0 in libniji and 1 (SCHED_FIFO minimum) in the reference. Next story: match it (init value 1).
- libniji's PP lock never raises the thread's priority to the ceiling (spec §22.4 names sched_setscheduler); untestable
  unprivileged here; a story with an RT-capable host.

## Jobs (mei)

- timed-wait-edge-order-2-lint0: step 0, clippy + static on the inherited head: rc 0.
- timed-wait-edge-order-2-probe1, -lint2 (clippy, static, records check, rt test, probe): evidence; lint1 red (record
  bodies with newlines, fixed).
- timed-wait-edge-order-2-gate1 (clippy, static, `affected --run --base origin/next`, head 7d6d6207d): rc 1; 15 suites,
  13 ok; 10275 passed, 14 failed = 7 tests x2, all the mei host locale set of #7846 (catalogs_equal_the_reference,
  locale_data x2, locale_behaviour_equals_the_reference, printf_corpus_equals_the_reference, s7_2_f_0,
  hot_text init_data_clusters_hold_at_every_text_phase, which printf-no-heap-3's mei gate also failed today);
  conformance 59 passed; rt pthread_sync green. Peak 8.1G.
- timed-wait-edge-order-2-plants2 (`handoff/TIMED-WAIT-EDGE-ORDER-2-plants.txt`): 6 CAUGHT, 0 SURVIVED, 0 WARN.
  Superseded: plants1 (baseline aborted: a `lit:` pattern containing ` | ` split the line; plant rewritten).

## Next step

Land. Then the INFORMATIONAL default-ceiling item as its own small story.
