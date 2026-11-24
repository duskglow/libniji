# LOCALE-PER-DIR-LOAD: handoff (final, leg 3)

State: branch task/locale-per-dir-load. Code head a9e89300f; later commits touch handoff/ only. Base origin/next
0495c4684 (nothing landed on next since the brief). Role mei:~/w/s1.
Decision in force: D-7.21 (records/decisions/D-7/D-7.21.toml, amends D-7.7); spec Appendix D.1 amended.
Full report: handoff/LOCALE-PER-DIR-LOAD-report.md; record handoff/reports/LOCALE-PER-DIR-LOAD.json.

## Done
- crates/core/src/locale/mod.rs: the archive (LOCPATH unset, archive form only), then candidates x (LOCPATH dirs,
  /usr/lib/locale); the first place holding the category decides (codeset mismatch = not found).
- Tests: locale_data builds its layouts (in memory, and an archive made with the host's localedef);
  `search_order_follows_the_reference`; s07 s7_2_f_0; hot_text_products' fault probes run on a test-made archive
  bind-mounted on /usr/lib/locale (user + mount namespace), the same data model as the trace (canon_host.sh).
- Probe tables: 111 rows compared, 18 differing before, 0 after; coordinator #7885 (LOCPATH fallback) 20 rows, 0.
- Coordinator #7884 (tracer never records the directory path): cause and fix as above (measured runs now take the
  traced archive path); a directory trace data model is a follow-up (INFORMATIONAL).
- §9 layout-bins diff vs merge base: net +71 B over bins 0-8, traced locale text -6.1 KB, other changes cold fillers.
- Final-head jobs (mei): locale-per-dir-load-gate3, locale-per-dir-load-plants2, locale-per-dir-load-xtask-test3,
  locale-per-dir-load-fullprof3 (green). Earlier, before the budget tightening (SUPERSEDED, green):
  locale-per-dir-load-gate2 (6529/0), locale-per-dir-load-plants (5/5 CAUGHT), locale-per-dir-load-xtask-test2,
  locale-per-dir-load-probe-after, locale-per-dir-load-base-so (merge-base products, for the bins diff).
- fullprof2 was red only on WINDOW-BIN-BYTES "level improved" (release tool 2 windows < 3): budget tightened to 2
  (5ab4d3876, D-7.21).
- SUPERSEDED: seven1, check2, check3, locale-per-dir-load-gate, xtask-test, fullprof (older heads or killed),
  fullprof2 (budget not yet tightened).

## Open questions
None for this story. Follow-ups: a directory-layout trace data model; DYN_TRUE_BOUND (65 measured vs 73).

## Next step
Land (coordinator). Nothing left to run on this branch.
