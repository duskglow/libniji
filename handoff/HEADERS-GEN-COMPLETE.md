# HEADERS-GEN-COMPLETE: handoff (leg 1 of the run, 2026-11-19)

State: branch `task/headers-gen-complete`, cut from origin/next fb63eeec8; role mei:~/w/s2 (`cargo` runs only
there). Brief: `~/state/briefs/HEADERS-GEN-COMPLETE.md`. Scratch: `~/w/.scratch/HEADERS-GEN-COMPLETE/`
(`diffs/<branch>.diff` = `git diff <merge-base> origin/task/sl-headers-<branch> -- xtask/src/headers.rs`, saved
for s07-4 s08-4 s10-3 s11-5 s12-3 s15-2 s16-2 s16-3 s19-2; read those files, do not re-run the diffs).

## Done

* c52cc8ac8 `Split the header generator into one module per construct family`: pure move. `xtask/src/headers/`
  `cond.rs` (levels, condition, mask_of; Gen is_covered/rows/dont_care/cond/narrower_tag/fortify_mask),
  `values.rs` (literal, Gen::value), `types.rs` (SCALARS, scalar, dwarf_scalar; resolve, type_size, type_words),
  `aggregates.rs` (members_nested, emit_nested, tag_of_typedef, tag), `typedefs.rs` (typedef, cxx),
  `functions.rs` (declaration, function), `objects.rs`, `enums.rs` (`@@enum`). headers.rs keeps tables/load,
  registry, Gen struct, guard, emit (placement), generate/run/stage, tests. Proof: `headers-gen --check` ok (83
  headers, byte-identical), `cargo test -p xtask --bin xtask headers::` 8/8, clippy -p xtask clean.

## Inventory of next's tables (rows; construct; module)

layouts 1913: aggregates keyed `struct X`, `union X` or a typedef name; members scalar/pointer/array; 383 nested
`a.b` rows (complete struct members, typedef'd aggregates, named members of `struct|union <anon>` type incl.
arrays `struct <anon>[10]`); 23 unnamed `<anon>` rows `size N rows K` (K = ALL descendant rows that follow, in table
order: tcphdr union rows 28 = struct 10 + struct 16 + 2); 47 bit-fields `bits W@0` (position always 0: extractor
gap, spec/tools/abi_extract.py:350 defaults DW_AT_data_bit_offset to 0: RULING/INFORMATIONAL); 13 `fn(...) *`;
`enum <anon>` members; packed (align 1), over-aligned, trailing padding -> aggregates.rs.
typedefs 3889: function 3397 (resolved prototypes -> functions.rs), typedef 288 (scalar 142, self/memberless 67,
named for linkage 30, fn pointer 14, empty cxx 11, pointer 9, fn type 8, array 6, qualified 1 -> typedefs.rs),
struct 160 (tag + C++ linkage name), object 37 (-> objects.rs), enum 5, union 1. macros 7387: int 6916, char[N] 159,
unsigned int 144, unsigned long 90, long 67, long long 4, unsigned long long 3, float 2, double 1 (-> values.rs;
next has no floating literal). constants 7084. prototypes 3781. callsites 7890 (addr 3996, call 3890, macro 3).
visibility 72278 (macro 42572, function 12959, typedef 9977, enumerator 5162, struct 1156, include-error 216,
object 214, union 11, enum 10). initializers 10 (no emitter on next). noexcept 3781. ftm-configs 40.

## Branch prior art to absorb (one implementation each; keep next's 83 headers byte-identical)

* S07-4: enum parameters -> `unsigned int` (enum_params); kernel `__u8..__s64` member types (kernel_int); FOB64
  redirect from `call` rows whose configs all end `+gnu64` (Tables.lfs, extern_lines `#if __NIJI_FOB64`); `<anon>`
  union of a name and its `__` alias (keep the first); member of a typedef of an anonymous struct (`__fsid_t`;
  next's derive_layouts may cover it); typedef with empty size (`DIR`) -> incomplete `struct __dirstream;`; its
  stat/stat64 hard-code is superseded by next's `@@hidden-member` (do not take).
* S08-4: LANGUAGE_MODE (max_align_t: `__STDC_VERSION__ >= 201112L || __cplusplus >= 201103L`); headers/fnmembers.tsv
  (type, member, declarator with `@`) for `fn(...) *`; literal("pointer"); `@@init(NAME)` from initializers.tsv via
  init_of (layout + bytes, union by first member); union_tag (union typedef with empty cxx: C++ `union N {..}`, C
  `typedef union {..} N;`, condition | cxx_mask); recursive members (members_why/member_lines/member_decl with
  Track{end,offs,count,align}; `<anon>` `rows K`; declared align > natural -> `__aligned__` on first member; packed
  if declared 1 < natural); typedef of array of struct (`jmp_buf`); `typedef struct ucontext_t {..} ucontext_t`;
  linkage-named rows under `union|struct {cxx}`; emit_nested `union TAG`; `#include` after `#if` is conditional.
* S10-3: `FILE` where the header does not show `struct _IO_FILE` -> incomplete tag (generalise, no name test).
* S11-5: `@@generated-fortran`, `@@emit-c` (c_only), `@@if-macro`; hex float -> correctly rounded decimal (9/17/21/36
  digits, `_Float32.._Float64x` suffixes) with tests; bitfield_aggregate (struct <anon> of unsigned bit-fields);
  `complex` -> `_Complex`.
* S12-3: `@@needs`, `@@needs-if`; headers/aliases.tsv; struct-tag layouts synthesized from member sub-rows; `tree()`
  recursive members (`<anon>` rows K, `<anon>[N]`, `const`, bit-fields); objects via type_words.
* S15-2: float `inf`/`nan` -> `__builtin_inf*`/`__builtin_nan*`; enum without rows -> one placeholder enumerator;
  enumerator inside `#ifndef NAME`.
* S16-3: `enum <anon>` -> unsigned int; anonymous members for any typedef-keyed aggregate (it tests `Elf`/`Dl_`
  prefixes: generalise); layout-only member types -> layout_typedef; pointer to a covered typedef; `la_*` audit hooks
  (belongs in s16's `linked_elsewhere`); struct objects (`_r_debug`). Surviving plant `s16c-vector-newtype` is on
  xtask/src/abi.rs `[u8; {s}]` (S16-3 only) with cmd `headers-gen --check`, which never runs abi-gen: the rows test
  must cover vector typedefs (S16-3 tables `La_x86_64_xmm`..; none on next: use a synthetic-row unit test).
* S19-2: spec/abi/diagattrs.tsv -> ` __attribute__ ((...))` on extern declarations, `@@attrs(NAME)`.

## Design in force (steps 3-4)

* `headers/rows.rs`: a Gen with no placement emits every row into one TU: `#include <features.h>` (levels,
  `__NIJI_NOTHROW`, `__NIJI_WCHAR`, `__NIJI_BEGIN_C`), tags, typedefs, objects, functions (+ a `used` static holding
  `&f` per addr row), enumerators, then each macro `#define` + its asserts + `#undef`. Asserts: layouts (sizeof,
  _Alignof, offsetof per non-bit-field path, scalar member types), typedefs (size, align; C++ `niji_same<T, cxx>` with
  the table's raw C++ text), prototypes (C++17 `decltype(f)` vs cxx + noexcept.tsv), objects, macro values and C
  types (`_Generic`), strings/floats in C++ constexpr, constants.tsv, enumerators. One runtime C program: bit-fields
  (all-ones probe; expected bits from psABI allocation order) and initializers (memcmp bytes_hex). Call sites:
  configurations grouped into families by identical addr expectations; one compile per family with its ftm-configs
  flags; `nm -u` must equal the expected set. Exceptions: `headers/rows-exceptions.tsv` (table, name or class,
  reason), each RULING NEEDED. Entry: `headers-gen --rows DIR`, `--inventory`; a `#[test]`; a cheap static step.
* Contract check: cheap static step (main.rs `cheap_step`) or precheck: a `task/sl-headers-*` branch whose diff
  against origin/next touches xtask/src/headers.rs, xtask/src/headers/*.rs outside slices/, or header_check.py's
  shared parts -> `RULING NEEDED` naming the files.

## Next step

Build rows.rs (TU + compile test + inventory) on next's tables; then unify aggregates.rs members (one recursive
emitter covering tcphdr, rusage, siginfo_t, `sort_list`, `enum <anon>`, `fn(...) *`) driven by its failures; then
values (floating, pointer, initializers), contract check, exceptions, plants (one per module + contract), plan/headers.md
2.4/4.1, gate, precheck. Nothing pending on the host; no jobs started.
