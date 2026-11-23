# LOCALE-PER-DIR-LOAD: handoff (leg 1 -> leg 2)

State: branch task/locale-per-dir-load, HEAD b08ef95cf (pushed). Role mei:~/w/s1 (synced at b08ef95cf).
Decision in force: D-7.21 (records/decisions/D-7/D-7.21.toml, reserved id, amends D-7.7). Spec Appendix D.1
amended (Sources / Order bullets, codeset bullet).

## Done (leg 1)
- Reproduced all 7 failures on mei (job locale-per-dir-load-seven0, head 0495c4684 = next).
- Probes (scripts on mei ~/work/locale-per-dir-load/: names.c, probe.sh, probe2.sh, probe3.sh; local copies in
  the scratch dir). Reference rules found:
  1. LOCPATH unset: archive first, looked up under the archive form ONLY (no candidate walk in the archive;
     `en_US.UTF-8@zz` does not find archived en_US.utf8). A record holding the category decides.
  2. Then candidates (D.1 order), candidate-major across dirs: every LOCPATH dir, then /usr/lib/locale ALWAYS
     (also when LOCPATH is set; the archive is skipped when LOCPATH is set). LOCPATH empty = unset.
  3. First place holding the category file decides; codeset mismatch there = category not found (no fallback).
     A dir lacking the category file is passed over for that category.
  libniji before: archive candidate walk, dir-major LOCPATH, no default-dir fallback under LOCPATH, skipped
  mismatches. Probe table "before": ~/work/locale-per-dir-load/out-before-*.txt, out2-before.txt on mei.
- Code: crates/core/src/locale/mod.rs find_named/in_archive/in_dir rewritten; FindCache now caches one archive
  record per name (`Looked`), `settled()` = record with matching codeset.
- Tests: locale_data: new `search_order_follows_the_reference` (in-memory MemFiles + private archive made by
  host `localedef --prefix=<dir> --add-to-archive`), the two archive-assuming tests rewritten on it.
  s07 s7_2_f_0: zh_HK.big5hkscs -> qq_QQ.big5hkscs (installed by locales-all).
- Not yet verified: nothing compiled yet (no local cargo). Job started: mei `locale-per-dir-load-seven1`
  (so, clippy, static, the 7 tests + new test, niji --lib locale, all of hot_text_products).

## Remaining (exact next steps)
1. `~/bin/jobwait locale-per-dir-load-seven1` on mei; fix compile/lint errors (clippy -D warnings, static).
2. printf_corpus_products and catalogs_products (line 541 `LC_ALL=de_DE.UTF-8@euro`) should pass from the code
   fix (reference now fails that name on mei because /usr/lib/locale/de_DE@euro is ISO-8859-15). Re-check.
3. hot_text_products: cause found: page_faults.c runs with LANG=en_US.UTF-8 and the host's locale data; with
   per-locale dirs setlocale maps ~13 files (tool mode 58 faults vs TOOL_BOUND 44; init_data_clusters saw
   58/59 moving with the text phase because the directory load path is cold text). Fix at the cause without
   touching bounds: give faults_env a controlled archive layout, e.g. run the program under
   `unshare -r -m` with a bind mount of a test-made /usr/lib/locale (archive from localedef --prefix, as
   locale_data's private_archive) - measure counts equal the bounds on mei; skip with a printed reason when
   user namespaces are absent. Alternative needing a RULING: measure with C.UTF-8. Check also
   init_short_programs / init_coreutils bound tests (they assert `locale -a` has en_US.utf8).
4. Plants via `cargo xtask plants` (docs/plants.md lines): (a) drop the `.chain(core::iter::once(LOCALE_DIR))`
   (default dir under LOCPATH) -> search_order test must fail; (b) make in_dir skip a mismatch
   (`codeset_matches(..).then_some(..)?` style) -> search_order fails; (c) per-dir order (dir-major) -> fails.
5. Direct gate (`cargo xtask affected --run`, check for FULL escalation: spec changed -> may escalate), start-up
   checks (`cargo xtask test`; NIJI_FULL_PROFILE=1 so + hot_text_products), precheck, report json, handback-check.
6. INFORMATIONAL for the hand-back: the WL gate's start-up counts on mei change with the host's locale layout
   (dirs instead of archive: ~13 more mappings for en_US.UTF-8 programs). Reference errno=0 on a repeated failed
   normalized name vs libniji ENOENT (D-7.18) - existing decision, not chased.
