# DL-TRACE-INTERP: the loader's trace list matches the reference

**State.** Branch `task/dl-trace-interp`, code `a754e3c0c` + records/docs/plants `8152fa8df` (this note and
`handoff/reports/DL-TRACE-INTERP.json` the commit after). Role mei `~/w/s3`. Stopped at the context checkpoint
with the gate and plants running (PENDING JOBS).

## Done

* Probes (mei, reference loader black box; scripts in scratch, results in `spec/behaviour/dynlink.md` "Trace lists"):
  the interpreter is listed only when an object names it in `DT_NEEDED` (soname or path), at its breadth-first
  place; vDSO first, after the interpreter when that comes first; `NAME => PATH` when the program's `PT_INTERP`
  differs from the loader's path as run; library operand: the loader's path as run; no `DT_NEEDED`:
  `statically linked`, status 0. The reference's libc names the loader; its libm names libc then the loader.
* `crates/boundary/src/ld.rs` `list()`: that rule for all three entry points (`invoked` = the loader's `AT_EXECFN`
  when run as a program); libniji's own libraries (`search::PINNED`) name the interpreter after their own needs.
  This fixes (b) and (c) of ruling #6056.
* (a): only libc and libm differed (pinned sonames resolve to `<prefix>/<libdir>`, `usr/lib/x86_64-linux-gnu` in
  the system build). `tools/deb/build.sh --flavour system` now builds with `NIJI_LIBDIR=lib/x86_64-linux-gnu`
  (same file on merged /usr, which the reference's `/lib64` PT_INTERP needs too). Coexist flavour unchanged.
* `csys/util/ldd.c`: interim filter and `has_needed` removed. D-31.15's interim note replaced; pending record
  `records/pending/DL-TRACE-INTERP-1.toml` (D-NEW, series D-27, amends D-27.16 and D-31.15).
* `docs/differences-from-glibc.md`: `ldd -v` row (libc/libm version-need blocks), no compat option (reason in row).
* Test `list_interpreter_line_matches_reference` (crates/ld/tests/search_cache.rs): 11 operands x 2-3 entry points,
  whole outputs vs the reference, addresses/loader paths/libc dir masked. `list_matches_reference`'s `deps()` now
  drops the interpreter line as its doc says (it is `NAME => PATH` there, correctly).
* Plants `handoff/DL-TRACE-INTERP-plants.txt` (6): `ldd-several-header` fixed to compile (`several < 0`).

## Results

* mei `dl-trace-interp-check-3` rc 0: both list tests pass; ldd transcripts 28 ok, 0 mismatched (filter removed).
  (`-check-1` compile error, `-check-2` old test's filter: superseded.)
* Running at hand-back: mei `dl-trace-interp-gate-1` (xtask clippy, workspace clippy -D warnings, static, direct
  tier --base origin/next, `cargo xtask test`, NIJI_FULL_PROFILE so + hot_text_products), mei `dl-trace-interp-plants-1`.

## Open / next step

1. Collect the two jobs; a red is fixed by a continuation.
2. Not verified: the keke swap-root run (UTIL-LDD-3's `swap.sh`, ldd-1..7) with a system build of this branch and
   the normalisation for (a)-(c) removed; expected same=61. Needs `tools/deb/build.sh --flavour system` + keke.
3. Gap: the `NAME => PATH` spelling is exercised only by `list_matches_reference` (filtered); no plant on it.

Read: spec/behaviour/dynlink.md "Trace lists", records/pending/DL-TRACE-INTERP-1.toml, D-27.16, D-31.15.

## Rule slips

One local here-doc (`python3 - <<X`, patching a scratch probe script outside the tree); one `git commit --amend`
of an unpushed local commit of mine.
