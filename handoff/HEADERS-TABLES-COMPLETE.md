# HEADERS-TABLES-COMPLETE: handoff (leg 1, 2026-11-19)

State: branch `task/headers-tables-complete` from origin/next 2b5604f9c. Role mei:~/w/s2. Commits: 570b86928
(groups 1, 3, 6 and 7), 67c69b1a0 (plants file `handoff/HEADERS-TABLES-COMPLETE-plants.txt`). The rows test
exceptions went from 99 to 74 (`headers/rows-exceptions.tsv`).

## Done (leg 1)

* Extraction (`spec/tools/abi_extract.py --layouts-only`, run on mei, the reference system; the unchanged script
  reproduced the committed layouts.tsv byte for byte first): `spec/abi/enumerators.tsv` (tag, name, value; 310 rows
  of the named enumerations), layouts.tsv members at every depth and through the first element of arrays of
  aggregates (+48 rows), bit-field positions `bits W@P` (P from the low bit of the unit at the row's byte; 38 rows
  changed). PROVENANCE line added. One extraction run, so groups 1, 3, 6 are one commit (the brief asked one per
  group; splitting the shared regeneration was not worth three regenerations).
* Generator: `enums.rs` (`declared_enum`, `enum_body`; `@@enum TAG` takes no prefixes now), `Gen::tag` for `enum`,
  `Gen::item("enumerator")` and `emit()` route an enumerator of a typedefs.tsv-declared enumeration to the whole
  enumeration; `derive_layouts` derives an array element's layout; `level()` reads `<anon>` `rows K` as K rows at its
  level (abi-gen's reading; deeper rows follow). mcheck.h.in lost its hand-written enum (data now).
* Rows test: bit-field run-time checks use the recorded positions (the psABI derivation and its helpers removed);
  each declared enumeration's enumerators are checked for value and (C++) `decltype` = the enumeration.
* Group 7: the va_list C++ row is marked `PERMANENT (ruling #6721)`; the list header states the ratchet.
* libc-include changes (every one from the new rows): mcheck.h (enum mcheck_status from enumerators.tsv),
  sys/ptrace.h (enum __ptrace_request now holds only its own 43 enumerators; the PTRACE_O_/PTRACE_EVENT_ names
  that the prefix rule put in it stay anonymous enumerators), stdlib.h and sys/types.h (`__atomic_wide_counter`
  complete: the `__value32` rows), sys/pidfd.h (siginfo_t complete: `_bounds` rows; it now needs `union sigval`).
  abi-gen output (crates/abi/src/layout.rs, tests/appendix_c_layouts.rs) regenerated for the new rows.

## Remaining (next leg): groups 2, 4, 5 of the brief, in this order

2. Provider column in prototypes.tsv (`libc` default, `program` for la_* (12 + 12 function types), `debugger` for
   ps_* (4 function types), `none` for bindresvport6 and getumask: ruling #6715 says declare them). Read
   `xtask/src/headers/functions.rs` (the "no default-version export" refusal) and `Tables.linked_elsewhere` in
   headers.rs (S16's slice field; drop it for the column if S16 has not landed, check `git log origin/next` is not
   allowed: check `headers/slices/s16*` on the branch). 32 exception rows.
4. `__SOCKADDR_ARG` / `__CONST_SOCKADDR_ARG`: layouts rows for the transparent union and a macro row for the C++
   pointer form; 8 C++ exception rows (accept, accept4, getpeername, getsockname, recvfrom, bind, connect, sendto).
5. macros.tsv pointer type (18 constants rows: NULL, SIG_*, MAP_FAILED, ...), rows for FNM_NOSYS,
   DLFO_EH_SEGMENT_TYPE, TD_SIGNO_MASK (probe whether TD_SIGNO_MASK is function-like), function rows for the 10
   type-generic macros, prototypes rows for gets and bsd_signal (by type_probe.py / header probes on mei).

Each group: extraction or table rows on mei, generator change, drop its exception rows, `cargo xtask headers-gen`
then `cargo xtask headers-gen --rows /dev/shm/claude/<dir>` (jobrun kind test), copy libc-include and crates/abi
back with `rsync -rc`, commit, add plants to the plants file.

## Proof so far (head 67c69b1a0)

* `cargo xtask headers-gen --rows`: ok, 0 failed (callsites 2780, constants 6891 (21 exc.), initializers 10,
  layouts 235 (1), macros 7189, prototypes 2782 (14), typedefs 3068 (26), versions 99, visibility 829 (12)).
* clippy -D warnings and `cargo xtask static` clean (job headers-tables-complete-lint1).
* `cargo xtask affected` escalates to FULL (xtask, spec/abi): the train's. Crate tests: job
  headers-tables-complete-test1 (`cargo test -p xtask -p niji-abi`; at the refresh every finished suite was ok,
  xtask 497 passed incl. the rows unit test); plants: headers-tables-complete-plants1 (6 plants, still running at
  the refresh). FIRST STEP of the next leg: `~/bin/jobwait headers-tables-complete-test1
  headers-tables-complete-plants1` on mei, read `~/logs/<label>.log` (grep CAUGHT/SURVIVED/WARN), fix a survivor.
  Both jobs prove 67c69b1a0's code; later code commits need a new test and plants run (append plants for the new
  groups to the same file).

## Open / informational

* sys/pidfd.h now defines `union sigval` under siginfo_t's condition (`!time64only`), while visibility.tsv gives
  the tag in fewer configurations (not c89..c2x, posix1/2, xopen, isoc*). The nested emitter's narrower-tag
  handling (`@@hidden-member`, `narrower_tag`) covers structures only. Recommend: extend it to unions (a member
  of a union type whose tag is narrower is spelled through an anonymous union there).
* Process slip in leg 1: one `sed -i` on headers/sys/ptrace.h.in (a one-line marker change) and two empty
  heredocs; not refused, reported here.
