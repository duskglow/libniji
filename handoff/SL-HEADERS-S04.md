# SL-HEADERS-S04: call-site and noexcept tables (slice 4 of plan/headers.md)

Branch `task/sl-headers-s04-2` (continues `task/sl-headers-s04`, from origin/next). Role mei:~/w/s1.
Briefs: ~/state/briefs/SL-HEADERS-S04.md (the change), ~/state/briefs/SL-HEADERS-S04-2.md (this run).
Plan rows: plan/headers.md §2.2, §3 (P3, P4), §4 slice 4.

## State (done)
* `spec/tools/callsite_probe.py` writes `spec/abi/callsites.tsv` (7889 rows) and `spec/abi/noexcept.tsv`
  (3780 rows). It reads back only error line numbers in its own units, relocations of its own object and
  sizes and demangled names of its own symbols. About 35 s per run with 20 threads on mei.
* `xtask/src/callsites.rs`: static check `callsites` (in `cargo xtask static`, also `cargo xtask callsites`),
  6 unit tests (`cargo test -p xtask callsites`), green.
* Plants: `handoff/SL-HEADERS-S04/plants.txt`, 27 plants on the checker (job `sl-headers-s04-2-plants`).
* Appendix C.13 written, C.1 rows and the `spec/abi/PROVENANCE` line added.
* Coverage (the brief's proof): all 44 `__isoc*` and 81 of 83 `*_chk` exports reached; `__gets_chk` (no `gets`
  row in prototypes.tsv) and `__vprintf_chk` (fortified `vprintf` reaches `__vfprintf_chk` + `stdout`) are
  exempted with reasons in the checker. Every prototypes.tsv function has a noexcept row.

## This run (SL-HEADERS-S04-2)
* Unit tests were red: the fixture wrote `c89,c2x` where C.12's form is the shorter `!cxx98`. Fixed; added a
  case that a `*` row overlaps a second row of the same mode (the `cfg-star` plant needed it).
* The 241 cxx98 `error` rows were probe faults, found with exit codes and own-file error counts only
  (scripts in the run's scratch, not in the tree): (1) a function-like macro of the same name expanded the
  redeclared name (stdbit.h, ctype `_l`/ascii names): now written `R (f)(A...)`; (2) the redeclaration's
  `extern "C"` conflicted in thread_db.h, sys/pidfd.h and for at_quick_exit: now no linkage specification
  (a redeclaration keeps the earlier linkage); (3) a tag hidden by a same-named function was demangled bare
  (`stat*`, ftw's callback): now `struct stat*` for any prototypes.tsv name; (4) a unit failing outside its
  lines now splits into one unit per name instead of losing the whole header.
* Result: 45 `error` rows left (42 that are C++17 overload sets too, plus abs, modf, register_printf_type,
  whose `&f` does not resolve in C++98 with the system C++ headers). Only `error` rows and one `unknown`
  changed (196 error -> value, 1 unknown -> throw()); callsites.tsv unchanged byte for byte against the
  keke tables, so mei and keke agree. Every row with both values set pairs noexcept/throw() (2826) and
  none/none (500). Runs `sl-headers-s04-2-probe2` (run3, run4) are byte-identical.

## Open (informational)
* 368 rows are cxx17 `-` but cxx98 `throw()` (e.g. `__cabs*`): a visibility.tsv fact, not checked further.
* 3 rows noexcept/error (abs, modf, register_printf_type): a later probe could read the C++98 type from a
  unit with the plain C headers (-nostdinc) instead of the system C++ wrappers.

## Next step
Land. Slice 5 of plan/headers.md uses these tables; nothing here is left to redo.

## Jobs (mei)
sl-headers-s04-2-lint2 (clippy + static, rc 0); sl-headers-s04-2-plants (rc 0: 27 CAUGHT, 0 SURVIVED,
0 WARN; results in handoff/SL-HEADERS-S04-2-plants.txt); sl-headers-s04-2-gate (`cargo xtask affected
--run --base origin/next`: the tool escalated it to FULL itself because xtask and generated spec/abi tables
changed; still running at hand-back, so PENDING for the dispatcher). Earlier: sl-headers-s04-2-lint1 (unit tests red, fixed),
sl-headers-s04-2-xtest/-xtest2, -probe1/-probe2 (diagnostic and table runs).
