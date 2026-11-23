# CTYPE-OBJECTS-SETLOCALE: the six ctype objects follow the global locale

State: branch `task/ctype-objects-setlocale` (from origin/next 0495c4684). Code head at the end of leg 2: e5fd864b1
(this note is committed on top). Role keke:~/w/r1, synced at e5fd864b1. Decision in force: D-5.13 (amended in
`records/decisions/D-5/D-5.13.toml`).

## Done (legs 1 and 2)

* Reference probe (black box, glibc 2.39, Zen 1 host): table in `spec/behaviour/ctype-objects.md` ("setlocale probe").
  The six objects follow the global LC_CTYPE only (LC_ALL, LC_CTYPE, ""), not LC_NUMERIC and not uselocale; 16-bit =
  `*__ctype_*_loc()`, 32-bit = `nl_langinfo` items 5/16/15 (CLASS32/TOLOWER32/TOUPPER32), "C" = the start pointers.
* Code: `xtask/src/compat_objects.rs` emits hidden `niji_compat_ctype_store(words[6])` into the generated
  `csys/src/machine/compat_aliases.S` (regenerated and committed in leg 2); `csys/src/adapter/runtime.c` section 4: a
  WEAK empty `niji_compat_ctype_store` (libc.a keeps it; libc.so.6's generated strong one replaces it) and the
  one-call wrapper `niji_csys_compat_ctype_store` (leg 2: the earlier `if` on a weak reference broke csys-audit:
  the C layer holds single-statement wrappers only); `crates/boundary/src/locale.rs` `publish_global` (cold) calls it.
* Test `conformance/c/s05_ctype_legacy_objects.c` `setlocale_rows()`; leg 2: it unsets LD_LIBRARY_PATH before its
  first `system()` (dynamic-link mode names libniji's directory there; the host sh/localedef inherited it).
* Plants file `handoff/plants/CTYPE-OBJECTS-SETLOCALE.plants` (store-dropped in runtime.c; upper32-not-stored in
  locale.rs), msg `FAIL line N (file`.
* Docs: differences row D-5.13 shrunk to two residuals (pristine-start addresses; built-in C.UTF-8 32-bit tables).

## Jobs (keke)

* `ctype-objects-setlocale-suite-4`: GREEN on 03bab77+test fix (compat-gen, so, s05_ctype_legacy_objects on reference
  and libniji, `cargo test -p xtask compat_objects`, `-p niji-boundary --test s07_locale_symbols`). Before the
  runtime.c change; the change only replaces the `if` with a weak default.
* SUPERSEDED: suite, suite-2, suite-3 (test locale/env fixes), gate (csys-audit red: the `if`), plants and checks
  (killed: old head).
* RUNNING on e5fd864b1, started 15:11Z: `ctype-objects-setlocale-plants-2` (plants, own /dev/shm dir) and the
  driver `sh ~/work/ctype-objects-setlocale/final.sh -2`, which runs in order `ctype-objects-setlocale-gate-2`
  (direct gate; FULL items are "the train's"), `ctype-objects-setlocale-checks-2` (clippy -D warnings, static,
  `cargo xtask test`, full-profile so + hot_text_products), `ctype-objects-setlocale-counts-2` (counts.sh:
  cbench_startup at head and at base 0495c46 in a scratch copy, wl_percall isalpha_c/ctype_b_loc/toupper_ascii
  with --base). Outputs `~/logs/<label>.log`, start-up tables in `~/work/ctype-objects-setlocale/startup-*.txt`.

## Exact next step (leg 3)

1. `ssh keke '~/bin/jobwait ctype-objects-setlocale-plants-2 ctype-objects-setlocale-gate-2'` (repeat on rc 3);
   then checks-2 and counts-2. Plants results: copy the summary into `handoff/CTYPE-OBJECTS-SETLOCALE-plants.txt`.
2. A red: fix, rerun only that job with a `-3` label.
3. Precheck on the role: `cargo xtask precheck --jobs <labels> --head <sha>` (job kind test), report
   `handoff/reports/CTYPE-OBJECTS-SETLOCALE.json` (+ "jobs"), push, `ops handback-check`.
4. Expect start-up and ctype per-call counts unchanged (publish_global is cold; no hot code touched).

## Open

* RULING NEEDED candidate: give the built-in "C.UTF-8" its own 32-bit tables (3 KiB rodata) to remove the last part
  of D-5.13; recommend a follow-up story.
