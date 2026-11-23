# LOCALE-PER-DIR-LOAD: handoff (leg 2 -> leg 3)

State: branch task/locale-per-dir-load. Code head a9e89300f (pushed; this note is committed on top). Role mei:~/w/s1,
synced at a9e89300f. origin/next = 0495c4684 = merge base (nothing landed since): "merged with next" holds.
Decision in force: D-7.21 (records/decisions/D-7/D-7.21.toml, amends D-7.7). Spec Appendix D.1 amended (leg 1).

## Done
- Leg 1: probes (reference rules in D-7.21), crates/core/src/locale/mod.rs search order, locale_data tests
  (`search_order_follows_the_reference` + the two archive-assuming tests on a private layout), s07 s7_2_f_0.
- Leg 2: job seven1 (b08ef95cf): so, clippy, static and the 7 tests + new test all green.
- Leg 2: hot_text cause and fix (commits 5790b0304, 73082e031, a9e89300f). With per-locale directories, the
  LANG=en_US.UTF-8 probes map about 13 locale files: tool 58 / locale 57 faults against the archive's 44 / 43, and the
  tool count read 58 or 59 by text pad (init_data_clusters red). faults_env now bind-mounts a test-made archive
  (host `localedef --prefix -i en_US -f UTF-8`) on /usr/lib/locale in `contain::user("unshare")` + `unshare -r -m`;
  sh execs the probe via an unexported PATH (argv[0] page_faults, env LANG + caller vars, same exec-path length).
  Falls back to the host layout with a NOTE when localedef, user namespaces or a clean sh are missing.
  Measured (one static probe, true / locale / tool): mei (Skylake-SP, directories) host 10/57/58, probe archive
  10/43/44; keke (Zen 1, archive) host 10/43/44, probe archive 10/43/44. Data-phase run: tool 44 at all 10 pads.
- Dynamic start-up probe: 65 faults with the host path, the contained namespace and a plain namespace alike
  (bound 73, "more than 4 under" report): it predates this branch (INFORMATIONAL, not chased).
- Plants: handoff/LOCALE-PER-DIR-LOAD-plants.txt, job `locale-per-dir-load-plants` (on 73082e031; mod.rs and
  locale_data.rs identical at a9e89300f): 5 plants, 5 CAUGHT, 0 WARN.
- check3 (73082e031): clippy, static, hot_text_products 33/33 green. Affected tier: Direct (no FULL escalation).

## Jobs queued on mei (run in this order; each waits for the previous)
1. `locale-per-dir-load-gate2`: direct gate (`cargo xtask affected --run --base origin/next`), head a9e89300f.
2. `locale-per-dir-load-probe-after`: setlocale name tables vs this head's products -> mei
   ~/work/locale-per-dir-load/after{1,2,3}.txt and out-after-*.txt (compare with out-before-*, out2-before.txt).
3. `locale-per-dir-load-xtask-test2`: `cargo xtask test` (start-up check, §9).
4. `locale-per-dir-load-fullprof2`: NIJI_FULL_PROFILE=1 so + hot_text_products (start-up check, §9).
SUPERSEDED: seven1 (b08ef95cf, before the hot_text fix), check2 (static red: harness-contain, fixed), check3
(73082e031), locale-per-dir-load-gate (started on an unsynced head, killed), xtask-test / fullprof (killed while
queued, requeued as *2).

## Exact next steps
1. `~/bin/jobwait locale-per-dir-load-gate2 locale-per-dir-load-probe-after` (rc 3 = call again); on a gate red,
   fix, rerun only the failing suite. Read the "== ... lines that differ" sections of after1.txt: ref vs niji
   should now agree (before: en_US.utf8 / en_US NULL in libniji; de_DE.UTF-8@euro accepted). Put the table summary
   in the report.
2. jobwait xtask-test2 and fullprof2; both must be green (§9).
3. On mei: `cargo xtask precheck --jobs <labels> --head <final sha>`; write handoff/LOCALE-PER-DIR-LOAD-report.md
   and handoff/reports/LOCALE-PER-DIR-LOAD.json (+ "jobs"); push; `ops handback-check`.
   JOBS: mei:locale-per-dir-load-gate2, mei:locale-per-dir-load-plants (73082e031, plant targets unchanged),
   mei:locale-per-dir-load-xtask-test2, mei:locale-per-dir-load-fullprof2.
4. INFORMATIONAL for the hand-back: (a) the WL gate's start-up counts on mei now see per-locale directories
   (about 14 more faults per en_US.UTF-8 program than with an archive); (b) the dynamic start-up probe reads 65,
   8 under DYN_TRUE_BOUND 73 (fast), with or without this change: a bound to tighten; (c) reference errno 0 vs
   libniji ENOENT on a repeated failed normalized name: existing D-7.18, not chased.
   IMPROVEMENT: kill-label on a gate job waiting in `slots` did not end the inner command, which went on to run
   the gate on an unsynced head (found by sync_role BUSY); kill-label should end the whole tree.
