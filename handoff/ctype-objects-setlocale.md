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

## Jobs (keke) on 3a5ea1858, started 16:45Z

* Driver `sh ~/work/ctype-objects-setlocale/final4.sh` (setsid) runs in order `ctype-objects-setlocale-checks-4`
  (clippy, static, `xtask so` + hot_text_products, the full-profile pair, `xtask test`), `-counts-4`
  (`~/work/ctype-objects-setlocale/counts.sh`: cbench_startup head vs base, ctype percall), `-gate-4` (direct tier).
* `ctype-objects-setlocale-plants-4` (/dev/shm/claude/ctype-objects-setlocale/plants-4).
* SUPERSEDED: plants-3 (2/2 CAUGHT, 35d66059a), gate-3 (red, fixed above), checks-3/counts-3 (never started), and
  all leg-2 jobs.

## Exact next step

1. `~/bin/jobwait ctype-objects-setlocale-checks-4 ctype-objects-setlocale-plants-4 ctype-objects-setlocale-counts-4
   ctype-objects-setlocale-gate-4`. Expect CHECKS-OK, 2/2 CAUGHT, startup head = base.
2. Precheck with those labels on 3a5ea1858+; handback-check.

## Open

* RULING NEEDED: give the built-in "C.UTF-8" its own 32-bit tables (3 KiB rodata) to remove the last part of
  D-5.13; recommend a follow-up story.
* Seen in checks-4 before hand-back (17:05Z): the default-profile `cargo test -p niji --test hot_text_products` on
  3a5ea1858 passed 33/33 (the gate-3 red test included); plants-4 baselines ok.
* NOT VERIFIED at hand-back: the rest of checks-4, the start-up count and the gate on 3a5ea1858 (jobs above).
