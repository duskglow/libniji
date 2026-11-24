# HEADERS-GEN-SHARED: typedefs of unnamed enumerations; the P1 macro check across every slice (handoff)

**State.** Branch `task/headers-gen-shared` from origin/next 3fca4673c (SL-HEADERS-S07-TAIL-3 landed). Role shiki:~/w/sh1.
Commits: 0604a9908 (extractor, enumerators.tsv, generator form, rows test), a1e494326 (shared P1, s07.py, the two
regenerated headers, plants file), then this note and `handoff/reports/HEADERS-GEN-SHARED.json`.

## What was done
1. **`typedef NAME` enumerations (ruling #7643).** `spec/tools/abi_extract.py` `enumerators()`: an enumeration without
   a tag that exactly one DWARF typedef names directly gets the enumerators.tsv tag `typedef NAME`. Regenerated on the
   reference system (`--layouts-only`, in a scratch copy of spec/): enumerators.tsv gains 86 rows, nine typedefs
   (idtype_t, ACTION, VISIT, reg_errcode_t, td_err_e, td_event_e, td_notify_e, td_thr_state_e, td_thr_type_e); no
   existing row changed. Only enumerators.tsv was taken (see INFORMATIONAL 1).
   Generator: `enums.rs` `typedef_enum` / `typedef_enum_body`; `typedefs.rs` emits `typedef enum { ... } NAME;` for a
   self-named typedef with such rows; `headers.rs` routes an enumerator of it to that typedef (as a tagged
   enumeration's enumerator routes to its tag). Rows test (`rows.rs`): each member's value and, in C++, that
   `decltype (member)` is the typedef.
   Regenerated: only `libc-include/search.h` and `libc-include/sys/wait.h` change (FIND and ENTER, missing before, now
   come with ACTION).
2. **Shared P1 (#7645).** `header_check.py` `p1_units` asserts a macro's integer value in every p1 header that shows it,
   whichever header owns its macros.tsv row; reserved names (`_[A-Z_]`) stay owner-only; MINSIGSTKSZ and SIGSTKSZ join
   NOT_CONSTANT. s07.py's `units` copy is removed (its include-guard `problems` stays); a module's `units` is now
   optional like `problems`.

## Proofs (shiki, labels `headers-gen-shared-<x>`)
* `extract`: the extractor run and table diff (86 added, 0 removed).
* `gen`: `cargo xtask headers-gen` (236 headers) + rows test: 0 failed in every table (typedefs 3123 rows).
* `hc`: `header_check.py` over every slice: 0 failed of 17006 (16777 before this change).
* Probes (`~/work/headers-gen-shared/probe`, `run.sh`): waitid with P_PID/P_PGID/P_ALL, hsearch FIND/ENTER, twalk and
  twalk_r with VISIT, built as C (gnu17) and C++ (gnu++17) against the reference's headers and libniji's: identical
  output (17 lines each); sizes 4/4/4, values P_ALL..P_PIDFD 0..3, FIND 0 ENTER 1, preorder..leaf 0..3. With
  `<linux/wait.h>` included first, both header sets fail to compile the same way (the kernel's P_* macros in the
  enumerator list).
* `lint`: clippy -D warnings, `cargo xtask static`, `headers-gen --check`: green. `affected --list`: a bounded set
  (xtask unit tests, clippy, static); FULL: the train's (generated headers, enumerators.tsv, xtask).
* `gate` (a1e494326): affected ok, 6 suites, 685 passed, 0 failed, 0 SKIPPED, peak 2.2G.
* `plants2`: 4 plants, 4 CAUGHT, 0 WARN. SUPERSEDED `plants`: its rows-test baseline failed because the plant command
  passed a relative `--rows` directory (the call-site family units are compiled from another directory).

## Open
* INFORMATIONAL 1: `abi_extract.py --layouts-only` on the reference system also produces 19 layouts.tsv rows the
  committed table lacks (nested `_st` / `_xmm` members of `struct _fpstate` and `struct _libc_fpstate`). Not part of this
  story; recommend a table story (regenerate layouts.tsv, review abi-gen output).
* Not planted: the extractor change (runs only on the reference system on demand; its output was checked by the diff
  above) and the `emit()` routing for a header with an enumerator row but no typedef row (no shipped header has one).

## Next step
Land via the CI train once gate and plants are green. Plan rows: plan/headers.md 2.4 and 3 (P1).
