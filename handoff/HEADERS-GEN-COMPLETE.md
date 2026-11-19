# HEADERS-GEN-COMPLETE: handoff (after leg 2 of run 1, 2026-11-19)

State: branch `task/headers-gen-complete`, cut from origin/next fb63eeec8 (next has not moved since); code head
451288a8b (WIP), this note on top. Role mei:~/w/s2 (`cargo` runs only there; no local cargo or gcc). Brief:
`~/state/briefs/HEADERS-GEN-COMPLETE.md`. Scratch `~/w/.scratch/HEADERS-GEN-COMPLETE/`: `diffs/<branch>.diff` (slice
emitter prior art, read those, do not re-run), `rows4-failures.txt` (the full failure list of the last run).

## Done

* Leg 1, c52cc8ac8: generator split into one module per construct family under `xtask/src/headers/` (cond, values,
  types, aggregates, typedefs, functions, objects, enums); pure move, 83 headers byte-identical.
* Leg 2, 451288a8b: `xtask/src/headers/rows.rs`, the placement-independent rows test (module doc lists every check):
  `cargo xtask headers-gen --rows DIR` and the test `headers::tests::every_table_row_is_emitted_and_matches_the_tables`.
  Units: `rows.c` (C gnu17 and C++ gnu++17 static asserts), `rows-run.c` (bit-fields, initializers), one call-site
  unit per family of configurations with equal feature levels (26 families; `nm -u` vs callsites.tsv addr rows).
  Reserved names the policy omits (`omitted`, R-H2/R-H3) are skipped for functions, objects, macros, enumerators.
  Exceptions: `headers/rows-exceptions.tsv` (table, name, lang c|c++|*, reason), empty. `DIR/failures.txt` lists all.
  Emitter additions: `Gen::item` (shared by `@@emit` and rows), float literals inf/nan/hex + `pointer` (values.rs),
  `@@init(NAME)` + `Gen::init`/`init_of` (port of S08-4's, signed values, no `<anon>`/bit-fields), `complex` ->
  `_Complex` in declarations (S11-5's construct). `headers-gen --check` ok (83 headers unchanged); xtask clippy clean.
* Dev jobs on mei (kind build, superseded, not proof): headers-gen-complete-rows1..rows4.

## Rows test now: 270 of 23865 rows fail (macros 0; run rows4)

36 typedefs `struct X has no usable layouts.tsv rows` (sigaction, sigevent, tcphdr, tcp_info, udphdr, __res_state,
__pthread_mutex_s, __cancel_jmp_buf_tag, ...): members_nested gives up on nested/anon/bit-field rows -> write the ONE
recursive member emitter in aggregates.rs (design below). 42 layouts (18 static asserts, 13 incomplete type, 10 no
member): same cause mostly. 29+3 C++ `Q was not declared` and 24 `expression list ... functional cast`, 10 `template
argument invalid`: C++ identity asserts whose cxx text names a tag/typedef C++ cannot see -> inspect, fix the emitter
or the test's spelling. 8 `unknown type name` (`__jmp_buf_tag`, `_IO_FILE`: array/linkage typedefs need `struct`;
S08-4's jmp_buf construct), 4 `expected before`, 2 `two or more data types`, 3 `flexible array member`, 2
`redefinition`. 21 constants with no macros.tsv row (table gap: exception or recommend a macros row). 12 visibility
function names with no prototype/function row (exception each). prototypes 29 (4+4 conflicting types = emitter vs
table, inspect; 2 `la_symbind*` = audit hooks, S16-3 prior art: libniji exports none -> exception or linked_elsewhere
data). initializers 9 (init_of on union typedefs). callsites 7 (C++ families: undeclared in C++). versions 3 (object
size asserts).

## Next steps (in order)

1. Recursive member emitter in aggregates.rs replacing members_nested's one-level code: walk rows in table order;
   `<anon>` row = C11 anonymous member of its `rows K` following rows (members at the SAME indent as `union {`, as
   sys/user.h on next); named member of type `struct|union <anon>` (or a tag with no own rows, `struct __exit_status`)
   = nested aggregate from its `m.` rows, ONE-LINE `  struct { a; b; } m;` when all inner members are plain (keeps
   utmp.h/utmpx.h/obstack.h byte-identical), multi-line otherwise; bit-fields via dwarf_scalar or a scalar typedef
   (tcphdr's `uint8_t`); `enum <anon>` -> unsigned int; kernel `__u8..__s64` (S07-4); union iff >1 member and all at
   one offset (count members, not lines); keep next's packed rule (`wide`); add S08-4's over-aligned first member;
   typedef-keyed aggregates allowed (S16 generalisation); `fn(...) *` keeps `void (*m) (void)` unless an optional
   `headers/fnmembers.tsv` row (S08-4) gives the declarator. Rerun; `headers-gen --check` must stay ok (diff any change).
2. typedefs.rs: array of struct (`struct __jmp_buf_tag jmp_buf[1]`), empty size -> incomplete `struct X;` (S07-4 DIR),
   layout-only typedef (S16-3 layout_typedef) incl. vector types; then the C++ identity failures.
3. Exceptions for true table gaps (each RULING NEEDED in the hand-back with a recommendation); test green.
4. Synthetic-row unit test for the vector construct of plant s16c-vector-newtype: `rows::check(root, &t, dir,
   Some(&only))` with a layouts-only typedef (size 16, align 16) injected into `Tables`; plant drops `__aligned__`.
5. Contract check in precheck.rs (new check `slice-contract`, list at line ~255, run like `train-owned`): story id
   (opts.story or `.synced` story, lower-case) starting `sl-headers-` and changed files (`Changes.files`, minus
   `same_as_base`) under xtask/src/headers.rs, xtask/src/headers/*.rs except slices/, or spec/tools/header_check.py
   -> Red, line `RULING NEEDED: ...` naming the files (plan/headers.md 4.1). Unit test with a fake Changes.
6. Plants in a new `handoff/HEADERS-GEN-COMPLETE-plants.txt` (one per module: aggregates, typedefs, functions,
   objects, values, enums/item, types, cond; the vector one; the contract check), cmd
   `cargo test -p xtask --bin xtask headers::tests::every_table_row` (+ the contract unit test); `cargo xtask plants`.
7. plan/headers.md 2.4 "Generated tests" and 4.1 (slice contract), D-row if needed (reserve via `ops reserve`).
8. Gate (`cargo xtask affected --run`), plants, clippy+static, precheck --jobs, report json, handback-check.

## Not to redo / notes

Inventory of next's tables (leg 1): layouts 1913 (383 nested `a.b`, 23 `<anon>` `size N rows K` with K = all
descendant rows, 47 bit-fields `bits W@0` whose position is always 0: extractor gap, INFORMATIONAL; 13 `fn(...) *`),
typedefs 3889 (function 3397, typedef 288, struct 160, object 37, enum 5, union 1), macros 7387 (int 6916, char[N]
159, unsigned 144, unsigned long 90, long 67, float 2, double 1, ...), constants 7084, prototypes 3781, callsites 7890
(addr 3996, call 3890, macro 3), visibility 72278, initializers 10, noexcept 3781, ftm-configs 40. Prior art per
branch (S07-4 .. S19-2, what to absorb): leg 1's note, `git show 5ad788f0d:handoff/HEADERS-GEN-COMPLETE.md` (this
story's own commit), section "Branch prior art". For the report: INFORMATIONAL,
self-reported: leg 2 ran `git show origin/task/sl-headers-s16-3:spec/abi/typedefs.tsv | grep xmm` (one row seen,
`struct _libc_xmmreg`), a file the brief does not name; not repeated.
