# GAI-MASS-LOSS: handoff

State: branch `task/gai-mass-loss`, base origin/next 97bf3542f, role keke:~/w/r1. HEAD: see
`handoff/reports/GAI-MASS-LOSS.json`. Decision in force: D-16.16 (amends D-16.14; D-16.8).

## What the CI failure was
Train 886: `dns_products resolv_equals_the_reference`, scenario `gaimass`, libniji 85..422 of 25000
requests not done and 61..421 signals missing in four runs; the reference all done. The premise
("libniji loses completions") was challenged (flag #7378) and the coordinator agreed: the missing
signals track the unfinished requests, and those are always the last queued (the one-request
signal calls), so the probe's fixed 60 s + 5 s wait ran out, it did not lose requests.
- Black-box stress of the queue and the groups (numeric requests, 28000..36000 per run, 4..8
  threads submitting, signal and thread notifications, pinned to 1 and 2 CPUs and unpinned;
  18 runs each for libniji and the reference, Zen 1 host): 0 lost, 0 duplicate, 0 wrong.
- Measured with the probe's new `gaimass.timing` (Zen 1 host at its usual load, ~28 on 60
  threads), before the fix: drain (25000 DNS queries through one serial server) 2.3 s libniji,
  2.5 s reference; CPU per request 98 (cache on) / 102 (off) us vs 89 us; submission CPU 0.17 s
  vs 0.012 s. The submission cost was `getaddrinfo_a` scanning every group from 0 for a free one
  (O(lists waiting) per call), unbounded since D-16.14 removed the 256-group cap.
- Not explained: why the CI host needed over 65 s when this host needs 2.5 s (a 25x stretch from
  load alone). Not reproducible here without adding load (the host is at half its threads); with
  the new wait no verdict depends on it.

## Done
- `boundary/src/gai_async.rs`: `GROUP_HINT`, where the free-group search starts (past the last
  group taken, lowered by `free_group`); goes round to 0 once before mapping more, so mapped groups
  stay bounded by the most lists waiting at once. Fork child resets it. D-16.16.
- `oracle/resolv_diff.c` `gaimass`: waits while requests keep completing (30 s without one ends
  the wait), then for the signals the same way (10 s); writes `<scratch>/gaimass.timing`
  (submit/drain wall and CPU, CPU per request, not-done codes). Stdout unchanged.
- New `gai_queue_products` + `oracle/gai_queue.c` (libniji only, no root, ~1 s): rounds that start
  part-way round the ring and outgrow it (heads 1..3001+, up to 40000 requests), then 4 threads x
  2000 one-request calls notified by signal or thread; every answer exact, none lost, every
  notification once. Expected lines follow from the man page.
- Plants: `handoff/GAI-MASS-LOSS-plants.txt` (grow without moving, hint search without going
  round, thread notification dropped).

- `dns_products` hang (coordinator note, DL-SLOT-REUSE: 29 min, timeout child exited unreaped,
  a thread in poll): `Command::output()` waits for the stdout pipe's end before reaping, and a
  probe's forked DNS server left behind by an ended probe holds it. The runner now writes output
  to files and waits for its child only; `resolv_diff.c` and `dns_diff.c` servers set
  `PR_SET_PDEATHSIG` (SIGKILL). Which scenario ran into the limit there is not known (its log is
  not on this host).

## Results
- Plants `gai-mass-loss-plants`: 3 CAUGHT, 0 SURVIVED, 0 WARN (gai_async.rs unchanged since).
- Gate `gai-mass-loss-gate` (0faf848): clippy, static ok; affected ok, 0 SKIPPED, peak 4.5G.
  Final head: `gai-mass-loss-gate2`.
- After the fix (gate's run, same host): gaimass submission CPU 0.017 (off) / 0.020 (on) s vs
  reference 0.010 s (before 0.169 / 0.174); drain 3.7 s vs 3.6 s; CPU per request 77..78 us vs
  62 us (DNS path; informational).

## NEXT LEG (refresh 1, ~21:50Z): exact next steps
1. Collect the final-head gate: `~/bin/jobwait gai-mass-loss-gate2` on keke (started 21:28Z on
   67ec7b6f6, clippy + static + affected). Plants job `gai-mass-loss-plants` is done (3 CAUGHT).
2. Coordinator note 21:37Z (in force): hung or killed dns_products runs leave ROOT-owned oracle
   children (resolv_diff_host/_niji under `sudo -n unshare`) orphaned for days (46 on the
   Skylake-SP host, 16 on the Zen 1 host). Make the harness unable to leave anything behind: run
   the oracles in their own PID namespace whose init dies with the test (`unshare --pid --fork
   --kill-child` or equivalent), on timeout end the whole namespace, then reap. Add a test that
   kills the harness mid-run and checks no process of it remains. Do NOT kill the existing
   orphans (root; the owner does). Files: `crates/core/tests/oracle/resolv_run.sh`,
   `dns_run.sh` (same `sudo -n unshare -m -n` line), `crates/core/tests/dns_products.rs` (`run`).
   Design point to settle: the chain is test -> timeout -> sh -> sudo -> unshare -> ns init; the
   namespace must die when the TEST dies, not only on timeout (e.g. the PID namespace outermost,
   created by the test's own command, with `--kill-child`, and the test's child given a parent-
   death signal; `setpriv --pdeathsig KILL` exists in util-linux). Check `unshare --help` on both
   hosts for `--kill-child`.
3. Then clippy/static, a direct gate, plants for the harness change if testable, precheck
   (`cargo xtask precheck --jobs <labels> --head <sha>`), handback-check, hand back. The hand-back
   must carry a SECURITY-PRIVATE section: ask the previous leg's final message (the refresh
   message) for its content; it is not written in the repo.

## Not done / next step (leg 1)
- Pinned (1 CPU) run of the harness for a contention measurement: started, sat 20 min queued for
  a slot, stopped to free the role for the gate (`gai-mass-loss-pin1`, superseded).
- No pre-LIMITS-RUNTIME-SMALL comparison on gaimass: that build refuses the scenario (20000 in one
  call: EAI_AGAIN; groups capped at 256), so there is no per-request figure to compare.
- Next: if CI shows gaimass slow again, read `target/test-oracles/resolv_diff-*/gaimass.timing`
  on the CI role to see where the time goes.
