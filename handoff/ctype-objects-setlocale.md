# CTYPE-OBJECTS-SETLOCALE: the six ctype objects follow the global locale

State: branch `task/ctype-objects-setlocale` (from origin/next 0495c4684). Code head: 3a5ea1858 (this note and the
report are committed on top). Role keke:~/w/r1, synced at 3a5ea1858. Decision in force: D-5.13 (amended in
`records/decisions/D-5/D-5.13.toml`, view regenerated).

## Done

* Legs 1-2: reference probe (table in `spec/behaviour/ctype-objects.md`); the six objects follow the global LC_CTYPE
  only; generated hidden store in `csys/src/machine/compat_aliases.S`; weak empty store + one-call wrapper in
  `csys/src/adapter/runtime.c`; `publish_global` (setlocale only, cold) calls it; test
  `conformance/c/s05_ctype_legacy_objects.c` `setlocale_rows()`; plants file; differences row D-5.13 shrunk.
* Leg 3: gate-2 red (dynamic empty main 178292 > 173500: six new GLOB_DAT relocations). Fixed in 0651a9714: the store
  takes `(at[6], words[6])`; `locale.rs` `compat_ctype_objects()` (cold) asks the loader once on the first store
  (`niji_dl_sym(RTLD_DEFAULT, name, "GLIBC_2.2.5")`) inside `dlerr::keeping`. 35d66059a: records/clippy fixes.
* Leg 4: plants-3 on 35d66059a: 2/2 CAUGHT (`handoff/CTYPE-OBJECTS-SETLOCALE-plants.txt`). gate-3 on 35d66059a RED:
  only `hot_text_products init_startup_data_accesses_are_in_the_data1_cluster` (the dynamic start-up bound no longer
  failed): the first setlocale (`locale`, `rebind` probes) touched COMPAT_CTYPE_FOUND and LOADER_DLSYM outside the
  clusters. Fixed in 3a5ea1858 by declaring them in `crates/libc-so/startup-state.list` (FOUND and AT first-call;
  LOADER_DLSYM every-process, beside LOADER_PIN, since each dynamic start writes it). gate-3 was killed after the red
  (it had run 42 min; its orphaned xtask was ended with kill-mine).


## Leg 5 (CTYPE-OBJECTS-SETLOCALE-2): jobs on mei:s4, head e813bd4c1 (code identical to 3a5ea1858)

* Restored the train-owned s05 decisions view to its merge-base version (e813bd4c1); clippy and `cargo xtask static` clean.
* `ctype-objects-setlocale-2-plants`: 2 CAUGHT, 0 survived, no WARN lines.
* `ctype-objects-setlocale-2-full`: `NIJI_FULL_PROFILE=1 cargo xtask so` + hot_text_products 33/33, CHECKS-OK.
* `ctype-objects-setlocale-2-checks` (`xtask test`): rc 1, 7181 passed, 6 failed, all locale-environment reds of the host
  (the locale archive is absent on the gate machine: `locale_data` x2, `locale_products`, `catalogs_products`,
  `printf_corpus_products`, `s07_locale_clauses s7_2_f_0`; the known #7846 class, none in ctype code).
* `ctype-objects-setlocale-2-gate` (direct tier): rc 1, only 4 of the same reds (`locale_behaviour`, `catalogs`,
  `printf_corpus`, `s7_2_f_0`); conformance 54/54 passed, including `s05_ctype_legacy_objects`.
* SUPERSEDED: the earlier keke jobs `ctype-objects-setlocale-*-4` (gate-4 and counts-4 never started; checks-4 was
  still running its old `xtask test` and was ended).
* NOT VERIFIED: start-up instruction count head vs base (counts-4 never ran); not re-run in this leg.

## Leg 6 (CTYPE-OBJECTS-SETLOCALE-3): keke:n1, head = merge of origin/next (LOCALE-PER-DIR-LOAD) into the branch

* Merged origin/next (clean merge, no conflicts); clippy and static clean.
* ctype-objects-setlocale-3-gate (direct tier, --base origin/next): rc 0, all suites ok incl. conformance 54 programs. The four earlier locale-environment reds are gone with the merge.
* ctype-objects-setlocale-3-plants2: 2 CAUGHT, 0 survived, no WARN (first attempt refused: my own mkdir made the dir; superseded).
* ctype-objects-setlocale-3-checks: xtask test + NIJI_FULL_PROFILE so + hot_text_products (see report json for the result).
* NOT VERIFIED: start-up instruction count head vs base.

## Open

* RULING NEEDED: give the built-in "C.UTF-8" its own 32-bit tables (3 KiB rodata) to remove the last part of
  D-5.13; recommend a follow-up story.
* Exact next step: the train decides on the locale-environment reds; nothing else is pending in code.
