# SL-HEADERS-S05: generator and base layer (handoff, run 1: checkpoint)

**State.** Branch `task/sl-headers-s05` (from origin/next 4a0a545ef); head: see `handoff/reports/SL-HEADERS-S05.json`.
WIP. Stopped at the context checkpoint (~160k of 200k, 45 of 110 calls). The rules file and plan alone took about 90k
of context, so a continuation should NOT re-read plan/headers.md in full: §2.3, §2.4 and §3 are the parts it needs.

**Done (compiles, clippy and static clean, 3 unit tests green on keke role n2, job `sl-headers-s05-check5`).**
* `xtask/src/headers.rs`, `cargo xtask headers-gen [--check]` (wired in `main.rs`, not yet in `static`):
  * loads visibility, macros, typedefs, prototypes, layouts, callsites, noexcept and versions;
  * `headers/levels.tsv`: what features.h must compute (isoc, posix, xsi, misc, gnu, lfs64, fob64, time64,
    dynstack, fortify, cxx, reject) in each of the 41 `ftm-configs.tsv` configurations, from feature_test_macros(7);
  * `condition()`: the smallest DNF over level tests (single literals and pairs) selecting exactly a row's configs,
    ignoring rejected configs, the header's include-error configs and configs where the name is visible with no
    header (the `-` rows). Fails loudly when no condition exists (a level model gap);
  * emission per kind: macros (`macros.tsv` literal with a type suffix, most-negative values as `(-max-1)`),
    typedefs (scalar from the `cxx` column; aggregates from `layouts.tsv`, opaque `char[size]` aligned when the
    members are nested or bit-fields; a typedef named for linkage through another emits that one first), complete
    tags, functions (prototypes.tsv with scalar typedefs resolved, `__asm__` redirects per `callsites.tsv` addr rows,
    export at a default version checked, `__LIELLA_NOTHROW` from `noexcept.tsv`), objects;
  * `headers/reserved.tsv` + `omitted()`: R-H2/R-H3 reserved-name policy (`__x` and `_X..._H` guards out unless listed);
  * markers `@@generated @@emit @@if @@when @@owns @@value() @@intc @@formats @@iso10646` (module doc);
    `--check`: stale output, a template `#define` of a `macros.tsv` name without a marker, unexported references.
* `headers/iso10646.tsv`: Unicode 15.1.0 -> 202309 for `__STDC_ISO_10646__`.

**Not done (next steps, in order).**
1. Templates `headers/*.h.in` for the 16 headers. `features.h`: compute the levels (cascade: GNU; XOPEN by value;
   `_POSIX_C_SOURCE`; `_POSIX_SOURCE`; MISC = DEFAULT/BSD/SVID/GNU or no FTM and not `__STRICT_ANSI__`), `#error`
   when `_TIME_BITS == 64` without `_FILE_OFFSET_BITS == 64`, `__LIELLA__`, `__GLIBC__` 2, `__GLIBC_MINOR__` 39,
   `__GNU_LIBRARY__` 6, `__GLIBC_PREREQ`, `__USE_MISC`/`__USE_GNU`, `__LIELLA_BEGIN_C`/`END_C`, `__LIELLA_NOTHROW`
   (C: empty, because the C asm label must precede attributes; C++: `noexcept (true)` / `throw ()`), and the
   FTM macros features.h rows list (`@@if _DEFAULT_SOURCE` ...; `_POSIX_C_SOURCE` value from the level, marked
   `/* @@computed */`). sys/cdefs.h: the R-H3 vocabulary SL-HEADERS-1 found (`__BEGIN_DECLS`, `__THROW`, `__P`,
   `__nonnull`, `__glibc_likely`, `__wur`, `__REDIRECT`, `__ASMNAME`, `__NTH`, ...), Liella-written from GCC docs.
2. Add a generated per-configuration TU asserting features.h's levels equal `levels.tsv` (P1 for the model), the P1
   `_Static_assert` TUs (C: sizes, offsets, values, `_Generic` types; C++: prototype identity with `decltype`), and
   the P5 hygiene loop, as xtask tests compiling with `-nostdinc -isystem include -isystem $(gcc -print-file-name=include)`.
3. `header_probe.py`/`callsite_probe.py`: an extra-flags option (`-nostdinc -isystem ...`) and a Rust comparator
   applying `omitted()` (P2, P3), run as a job; `headers-gen --check` into `static`; `so` staging of
   `target/liella/include` and the UAPI copy; `conformance/run.sh --liella-headers`; plants file (template condition,
   a `levels.tsv` cell, a `macros.tsv` value, a redirect); pending decision record (`records new decision D-NEW
   --series D-X --story SL-HEADERS-S05`) for the marker format and the `__liella/` layout (proposal: definitions are
   inlined with `__LIELLA_D_<name>` once-guards; `__liella/` reserved for internal fragments, first used by the
   multiarch split in slice 20).

**Gaps found (INFORMATIONAL).**
* `PTHREAD_STACK_MIN` outside `_DYNAMIC_STACK_SIZE_SOURCE` has no value in `macros.tsv`/`constants.tsv` (slice 3 table).
* No table lists Liella's ENOSYS-only functions, which `gnu/stubs.h` needs: proposal: a generated list from the code.
* `gnu-versions.h` has no visibility rows (its names are not probe candidates): its content needs a table row source.
* `alloca` is a function row (no macro) whose call has no symbol in every configuration: P3 must show how a plain
  declaration behaves in strict modes.
* Most `PRI*`/`SCN*`/`INTn_C`/`INT_FASTn_MAX` names are not probe candidates; the generator derives them from
  `typedefs.tsv` types (C 7.8.1, 7.20.4) and only three PRI rows exist to check against.

**Do not redo.** The level table (checked against `ftm-configs.tsv` order by the generator), the condition search.
