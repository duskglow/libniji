# HEADERS-GEN-COMPLETE: handoff (run 1 done, leg 4, 2026-11-19)

State: branch `task/headers-gen-complete` from origin/next fb63eeec8 (next had not moved). Code head 52fef312c;
this note and `handoff/reports/HEADERS-GEN-COMPLETE.json` on top. Role mei:~/w/s2. Plants file
`handoff/HEADERS-GEN-COMPLETE-plants.txt`. Commits: c52cc8ac8 (one module per construct family, pure move),
451288a8b (rows test), 59b48cd50 (recursive emitter, `slice-contract` precheck check), d96149c6a (rows test over
every row, vector typedefs, exceptions), 1cd1a84e7 (libc-include regenerated), 52fef312c (plan/headers.md, plants).

## Proof

* Rows test (`cargo xtask headers-gen --rows DIR`; unit test `every_table_row_is_emitted_and_matches_the_tables`):
  23879 rows, C gnu17 and C++ gnu++17 and 26 call-site families: 0 failed, 99 excepted (`headers/rows-exceptions.tsv`).
  callsites 2780, constants 6891 (21 exc.), initializers 10, layouts 231 (6), macros 7189, prototypes 2782 (21),
  typedefs 3068 (39), versions 99, visibility 829 (12).
* Vector typedefs (plant s16c-vector-newtype's construct): unit test `a_vector_typedef_is_emitted_as_a_vector`.
* `headers-gen --check` (in static) green with the regenerated headers; jobs: see the report json.

## Construct inventory (next's tables; module)

| construct | rows | module |
|---|---|---|
| aggregate layouts (struct, union, typedef-keyed) | 1913 rows, 231 aggregates | aggregates.rs |
| nested members `a.b` (complete, typedef'd, named `struct/union <anon>`, `struct <anon>[N]`) | 383 | aggregates.rs |
| C11 anonymous members (`<anon>`, `size N rows K`) | 23 | aggregates.rs |
| bit-fields (`bits W@0`) | 47 | aggregates.rs |
| function-pointer members (`fn(...) *`), enum-typed members, packed, over-aligned, padding | in layouts | aggregates.rs, types.rs |
| typedefs: scalar 142, memberless 67, linkage-named 30, fn pointer 14, empty cxx 11, pointer 9, fn type 8, array 6, qualified 1 | 288 | typedefs.rs |
| vector typedefs (`T __vector(N)`) | 0 (synthetic test) | typedefs.rs |
| tags: struct 160, union 1, enum 5 | 166 | aggregates.rs `tag` |
| functions: prototypes / typedefs function | 3781 / 3397 | functions.rs |
| noexcept | 3781 | functions.rs |
| call sites: addr 3996, call 3890, macro 3 (redirections per configuration) | 7890 | functions.rs, cond.rs |
| objects (+ versions.tsv data objects 99) | 37 | objects.rs |
| macro values: integer 7224, char[N] 159, float 2, double 1 | 7387 | values.rs |
| constants | 7084 | values.rs (through macros.tsv) |
| initializers (`@@init`) | 10 | values.rs |
| enumerators | 5162 | headers.rs `Gen::item` |
| `@@enum` (enumerators by a template's prefixes) | template | enums.rs |
| feature conditions (ftm-configs 40, visibility 72278) | | cond.rs |

## Exceptions (99, each RULING NEEDED in the hand-back)

Enumerations without enumerators 19 (5 tags, 7 prototypes, 7 function types); functions a program provides 28
(la_* 12+12, ps_* 4); no default-version export 4 (bindresvport6, getumask); table depth gap 6 (siginfo_t
`_bounds`, `__atomic_wide_counter.__value32` and its users, struct __res_state `sort_list`); va_list in C++ 1;
`__SOCKADDR_ARG` in C++ 8; pointer-valued macros 18; untyped integer macros 3; type-generic macros 10; gets,
bsd_signal 2.

## libc-include changes (221 lines, 10 headers; sizes and alignments unchanged; every one from the tables)

const pointees kept (argp.h, getopt.h, `tm_zone`); `printf_va_arg_function` takes `va_list *`; named members of
anonymous union type instead of opaque objects (`__mbstate_t`, `Elf32_Dyn`, `Elf64_Dyn`, `Elf64_auxv_t`); pthread
mutex/cond/rwlock unions with their `__data` structures (stdlib.h, sys/types.h; `__atomic_wide_counter` stays
opaque: depth gap); `union epoll_data` guard names its kind; `struct __exit_status` at file scope (nested in
`struct utmpx` it is a nested class in C++).

## Not to redo / notes

* A dedup of `emit_nested` was tried and reverted: it removes 124 repeated guarded lines from next's headers
  (pidfd.h, procfs.h, utmp.h, argp.h): a cleanup story of its own.
* Bit-field rows all say `@0` (abi_extract.py reads only DW_AT_data_bit_offset); the rows test derives positions by
  psABI sequential allocation.
* When S16 lands `la_*` as data, drop their exception rows (an exception for an emittable row hides it).
* s16c-vector-newtype was on S16-3's xtask/src/abi.rs (abi-gen); the header side is covered here, abi-gen's not.

## Next step

Land (FULL gate is the train's: xtask changed). Then slices add data only (plan/headers.md 4.1); the RULING NEEDED
rows above each become a table story.
