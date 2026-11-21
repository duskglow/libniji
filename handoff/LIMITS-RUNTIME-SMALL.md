# LIMITS-RUNTIME-SMALL: handoff (leg 2)

State: branch `task/limits-runtime-small`, base origin/next 5e1903cce. Role keke:~/w/r1. HEAD: see the
report json. Story: audit HL-12, HL-8, HL-9 (owner ruling 2026-11-17: remove the caps, fallible growth).

## Done
- `crates/rt/src/chunks.rs`: `Chunks`, a table of `w`-word entries in chunks that never move (chunk k =
  one page << k, mapped on first need; 48-entry directory spans 2^60 bytes, so no limit but memory).
- HL-12 `legacy/src/exit_handlers.rs`: 2^20 cap and the 32 MiB mapping at the 33rd registration gone;
  entries past the 32 inline ones go to `Chunks`. D-1.14 (amends D-1.2).
- HL-8 `boundary/src/fmtmsg.rs`: 256-entry table replaced by `Chunks` (nothing mapped until the first
  added severity); free entries reused first; still under `LOCK`. D-2.12 (amends D-2.6, D-2.10).
- HL-9 `boundary/src/gai_async.rs`: queue = ring of 64 inline slots growing by chunks (cold `grow`);
  groups 16 inline + chunks never moved; `EAI_MEMORY` when growth fails. `MAX_WORKERS` 32 stays.
  D-16.14 (amends D-16.8).
- Tests: s01 `s1_2_i_3` (100 handlers map one page; 3 M run once each, most recent first), `s1_2_f_1`
  (three-page limit: 417th fails cleanly, 416 run in order); s02 `s2_6_c_3_table` (2000 levels);
  oracle scenario `gaimass` in `resolv_diff.c` / `resolv_run.sh` (20000 requests in one call + 5000
  one-request calls with a queued RT signal, all pending behind a server holding its first query 1.5 s),
  compared line by line with the reference by `dns_products` `resolv_equals_the_reference`.
- Plants: `handoff/LIMITS-RUNTIME-SMALL-plants.txt` (6: each cap restored alone, chunk directory capped,
  worst-case exit mapping).

## Probes (keke, black box, scratch program caps.c; reference dynamic, libniji static)
Before (origin/next products): atexit fails at 1,048,574; on_exit at 1,048,575; addseverity at 256;
getaddrinfo_a of 20000 numeric requests in one NOWAIT call returns EAI_AGAIN (-3), 0 answered.
Reference: 3,000,000 atexit / on_exit run (on_exit in order), 2000 levels printed, 20000 answered.
After (this branch's products): 3,000,000 atexit and on_exit run (on_exit in order), 2000 levels
printed, 20000 answered: the same as the reference. ("Before" = the products r1 held before this
leg's sync, an earlier next; the three caps' code there is the same as at the base 5e1903cce.)

## Results (leg 2)
- clippy (`-D warnings`) and `cargo xtask static` green on r1 (jobs limits-runtime-small-clippy1, -static1).
- Gate `limits-runtime-small-gate` (direct tier, base origin/next): rc 0, `affected: ok`, peak 12G,
  2 SKIPPED lines (not this story's).
- Plants `limits-runtime-small-plants`: 6 CAUGHT, 0 SURVIVED, 0 WARN. The gai plants fail on the
  gaimass lines (queue cap: the 20000 call `-10`, 0 answered; group cap: 256 of 5000 calls).
- Start-up checks (`cargo xtask test`, counts `cbench_startup`, full-profile hot_text_products): jobs
  limits-runtime-small-xtest2, -counts, -hottext (see the hand-back for their state).
- Layout: against r1's previous products (an earlier next, so not a clean merge-base diff) the exit
  registry's `register` grew 311 -> 384 bytes and `run_all` 369 -> 438 (bin 0 +34 bytes over all); the
  other differences come from the older tree. Possible follow-up if a start-up row moves: put
  `Chunks::reserve`/`get` behind one `#[cold]` function in exit_handlers.rs.
- SUPERSEDED: limits-runtime-small-xtest (queued only, stopped: my driver did not loop on jobwait's
  exit 3 and started it beside the gate), limits-runtime-small-probe-before (compile error: wrong
  include path).

## Informational
- `gai_cancel(NULL)` cancels at most 64 queued requests per call (fixed array in `gai_cancel`); not in
  the audit; with more queued, the rest stay queued. Candidate follow-up.
- The probe service (`tools/probe/remote.sh`, installed `~/bin/probe`) takes `--liella`, not the
  README's `--niji`, and reports "products missing" for a libniji tree even after `cargo xtask so`
  (it rebuilt the products, 20 minutes, then gave up). Probes in this story were run by hand instead.

## Next step
None if the jobs named in the hand-back are green; otherwise fix what they name.
