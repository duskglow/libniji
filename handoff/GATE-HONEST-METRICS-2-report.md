# GATE-HONEST-METRICS-2 report (second leg of GATE-HONEST-METRICS)

Branch `task/gate-honest-metrics-2`, role mei:~/w/s1 (Skylake-SP host). One-page state:
`handoff/GATE-HONEST-METRICS.md`. First leg's report: `handoff/reports/GATE-HONEST-METRICS.json`.

## Commits (this leg)

- e93d836d4 Merge origin/next (7a0ba4cb2) into task/gate-honest-metrics-2 (ruling #8197). No conflicts;
  `hot_text_products.rs` auto-merged with LOCALE-PER-DIR-LOAD's probe locale archive.
- 57aac469c Trace both locale load forms in the start-up text check (#8200).
- 12f1f42ee Name the reference probe's link file per call, not per process (race under plain `cargo test`).
- then: handoff and report only.

## #8200: the surviving plant

The first leg's `perdir-trace-dropped` (regen.sh no longer appends the per-locale-directory trace's names) passed
`init_data_clusters_hold_at_every_text_phase`. Evidence (mei, the first leg's plant dir):
- The mutant's `hot-text.list` lacked exactly 4 keys that the branch build has, all of the `locale` mode:
  `name::candidates<..find_named<global_locale::Files ..>..>`, `global_locale::note_failure`,
  `hot::mem::equal_rest`, `niji_hot_equal_big`. Nothing else in the build had grouped them.
- `/usr/lib/locale/locale-archive` on that host has mtime 2026-11-23 18:10:18Z. The plant job ran after that
  (log 18:19). The check traced with the host's own files (no redirection), so it took the archive path and
  never ran the four functions.

So no second route groups the per-directory functions. The check depended on the host's locale layout. Fix
(57aac469c): `startup_text_outside_the_hot_group` keeps its four host-file traces and adds `locale` and `tool`
under `canon_host.sh`'s two redirections (archive: its last output line; per-locale directory:
`perdir.redirect`). Proof: the same plant is CAUGHT on the same host, now with an archive (plants1, plants2). The
failure names `locale (per-locale directory)`: `locale::in_dir<Files>`, `name::candidates<Option..>`,
`global_locale::note_failure` (3 outside, 180 functions traced, 177 of libc.a). These are the names as of next's
LOCALE-PER-DIR-LOAD. Cost: the test takes 76 s (failing mutant run) where it took 48 s before (first leg's surviving mutant).

## The reference_probe race (found by the release run)

htp-release1 (plain `cargo test`, the tests as threads of one process): 31 passed, 2 failed, both
`rename the reference probe into place: No such file or directory` (hot_text_products.rs:426). The link file was
`<name>.<pid>.part`. Under nextest each test is its own process, so leg 1's fix held there; here two threads
shared the name and one moved the file away before the other's rename. Fixed with a per-call counter in the
name (12f1f42ee). The cause is explained and the fix is in the branch's own code. No plant: it is test-harness
code. The release run of the final head is the proof (htp-release2).

## Jobs (mei)

Final head 12f1f42ee (code), then prose-only commits:
- gate-honest-metrics-2-lint2: clippy (workspace, all targets and features, -D warnings) + static + `cargo test -p xtask`.
- gate-honest-metrics-2-htp-release2: `NIJI_FULL_PROFILE=1 cargo xtask so` + hot_text_products.
- gate-honest-metrics-2-gate2: direct tier (`cargo xtask affected --run --base origin/next`).
- gate-honest-metrics-2-xtest2: `cargo xtask so && cargo xtask test`.
- gate-honest-metrics-2-plants2: 3 plants (`handoff/GATE-HONEST-METRICS-2-plants.txt`).
Results: see "Final results" below.

Superseded (older head 57aac469c or ee996a26c):
- lint0: inherited head, green.
- lint1: green.
- gate1: 7 suites, 660 passed / 0 failed.
- plants1: 3 CAUGHT, no WARN.
- xtest1: green. Pass 1 7191 passed / 0 failed; native pass 4879 passed / 0 failed. The first leg's four reds
  are gone after the merge, as #8197 expected.
- htp-release1: red. It found the race above.

## Final results

All on 12f1f42ee, mei role s1, all rc 0:
- lint2: clippy, static and `cargo test -p xtask` green. Static notes one records view pending regeneration
  by the train (the D-X.165 edit).
- htp-release2: 33 passed / 0 failed (62 s).
- gate2: 7 suites, 660 passed / 0 failed, peak 2.0G.
- xtest2: pass 1 7191 passed; native pass 4879 passed; interposed 129 + 46 passed; 0 failed. Peak 12G.
- plants2: 3 plants, 3 CAUGHT (perdir-trace-dropped, setlocale-keys-dropped, kinstr-lost), no WARN. The
  unmutated baseline test run took 78 s.

## Rulings / informational

- None new. #8198 / #8199 are answered (follow-up story: min-of-N kernel count stability, release ratios on the
  three classes).
- INFORMATIONAL: xtest's second-pass build warns `niji_machine_running_on_valgrind is never used`
  (crates/boundary/src/ffi.rs:77). That is next's code, not this branch's.
