# CTYPE-OBJECTS-SETLOCALE: the six ctype objects follow the global locale

State: branch `task/ctype-objects-setlocale` (from origin/next 0495c4684). Code head at the end of leg 3: 35d66059a
(this note and the report are committed on top). Role keke:~/w/r1, synced at 35d66059a. Decision in force: D-5.13
(amended in `records/decisions/D-5/D-5.13.toml`, view regenerated).

## Done

* Legs 1-2: reference probe (table in `spec/behaviour/ctype-objects.md`); the six objects follow the global LC_CTYPE
  only; generated hidden store in `csys/src/machine/compat_aliases.S`; weak empty store + one-call wrapper in
  `csys/src/adapter/runtime.c`; `publish_global` (setlocale only, cold) calls it; test
  `conformance/c/s05_ctype_legacy_objects.c` `setlocale_rows()`; plants file; differences row D-5.13 shrunk.
* Leg 3: gate-2 (e5fd864b1) went RED: `hot_text_products::init_dynamic_empty_main_stays_within_its_instruction_bound`,
  178292 > 173500. Cause (readelf on the role's libc.so.6): the store reached the objects through the GOT of their
  GLIBC_2.2.5 names = six new `R_X86_64_GLOB_DAT` relocations, resolved by the loader in every start-up. Fix
  0651a9714: the generated store now takes `(at[6], words[6])` and has no symbol reference; `locale.rs`
  `compat_ctype_objects()` (cold) asks the loader once (`niji_dl_sym(RTLD_DEFAULT, name, "GLIBC_2.2.5")`, 0 in a
  program ld.so did not start) inside the new `dlerr::keeping` (keeps the thread's dlerror pending word).
  35d66059a: D-5.13 body back to one table cell (records check), `c""` literals (clippy). clippy and static clean on
  the role at 35d66059a.

## Jobs (keke) on 35d66059a, started 16:02Z

* `ctype-objects-setlocale-plants-3` (plants, /dev/shm/claude/ctype-objects-setlocale/plants-3; plant store-dropped
  updated to the two-argument call).
* Driver `sh ~/work/ctype-objects-setlocale/final.sh -3` (setsid) runs in order `ctype-objects-setlocale-gate-3`,
  `-checks-3`, `-counts-3`. Outputs `~/logs/<label>.log`.
* SUPERSEDED: plants-2 (2/2 CAUGHT, old head e5fd864b1), gate-2 (red: start-up bound, fixed above), checks-2 and
  counts-2 (killed: old head), and the leg-1/2 jobs listed before.

## Exact next step

1. `~/bin/jobwait ctype-objects-setlocale-plants-3 ctype-objects-setlocale-gate-3 ctype-objects-setlocale-checks-3
   ctype-objects-setlocale-counts-3`. Check gate-3 for the start-up test (must be green: GLOB_DAT for `__ctype*` gone:
   `readelf -rW target/niji/libc.so.6 | grep __ctype` empty). Copy plants results to
   `handoff/CTYPE-OBJECTS-SETLOCALE-plants.txt`.
2. Precheck on the role, update the report's gate/impact, push, `ops handback-check`.

## Open

* RULING NEEDED candidate: give the built-in "C.UTF-8" its own 32-bit tables (3 KiB rodata) to remove the last part
  of D-5.13; recommend a follow-up story.
* NOT VERIFIED yet: the fix's start-up count (gate-3 / counts-3 pending).
