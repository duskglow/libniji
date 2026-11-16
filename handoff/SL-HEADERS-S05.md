# SL-HEADERS-S05: generator and base layer (handoff after run 2, SL-HEADERS-S05-2: checkpoint)

**State.** Branch `task/sl-headers-s05-2` (cut from origin/next 4a0a545ef; carries run 1's commits). Head: see
`handoff/reports/SL-HEADERS-S05-2.json`. WIP, not for landing: stopped at the coordinator's clean boundary
"generator + base templates" (~135k of 200k context, 72 of 110 calls). A continuation reads only this page,
plan/headers.md §2.3, §2.4 and §3, and `xtask/src/headers.rs` by range (module doc first).

**Done.**
* Run 1: the generator core `xtask/src/headers.rs` (levels, exact condition search, emission per kind, reserved-name
  policy, markers), `headers/levels.tsv`, `headers/reserved.tsv`, `headers/iso10646.tsv`.
* Run 2:
  * Templates `headers/*.h.in` for all 16 slice-5 headers; `include/` generated and committed.
    * `features.h`: the level cascade, `__LIELLA__`, the R-H1 identity macros, `__USE_MISC`/`__USE_GNU`, `__LIELLA_BEGIN_C`/`END_C`,
      `__LIELLA_NOTHROW`, `__LIELLA_WCHAR`, and the implied FTMs under `@@if` conditions.
    * `sys/cdefs.h`: the R-H3 vocabulary.
    * `stdc-predef.h`: `__STDC_IEC_559__`, `__STDC_IEC_559_COMPLEX__`, `__STDC_ISO_10646__` from the Unicode tables.
    * Include guards are Liella's own (`__LIELLA_H_<path>`).
  * The generator:
    * a template's `#include <x>` of another template marks x's names as covered (not emitted again);
    * same-column range terms are allowed (`__LIELLA_POSIX >= 1 && __LIELLA_POSIX < 200112`);
    * `int volatile` typedefs stay scalar;
    * `__gwchar_t` maps to `__LIELLA_WCHAR`;
    * the PRI least/fast names are fixed;
    * types without a row: the signed type is derived from its unsigned row (C 6.2.5p6), and
      `@@intc`/`@@formats` fall back or say what is missing, in a comment in the output.
  * `headers-gen --check` runs in `cargo xtask static` (main.rs).
* Evidence (keke role n2; scratch script, not yet a repo test):
  * job `sl-headers-s05-2-smoke2`: 664 compiles, 0 failed. Each header compiles on its own and when included
    twice, in the 40 accepted configurations, C and C++, `-Wall -Wextra -Wsystem-headers -Werror`,
    `-nostdinc -isystem include -isystem <gcc include>`.
  * features.h's levels equal levels.tsv in all 41 configurations, and time64only is rejected.
  * The script is `~/work/SL-HEADERS-S05-2/smoke.py` on keke. Its logic is the model for the repo test in next step 1.

**Next steps, in order.**
1. Move the smoke check into a repo test: `xtask` test or `crates/.../tests`, `-nostdinc`. Add the generated P1 TUs:
   sizes, offsets, values and `_Generic` types (C), and `decltype` prototype identity (C++). Add P5 `-std=c89 -pedantic-errors`
   and the strict-mode namespace check.
2. P2/P3: run `header_probe.py`/`callsite_probe.py` on `include/` with an extra-flags option, and compare with a Rust
   comparator that applies `omitted()`.
3. `so` staging (`target/liella/include` plus the UAPI copy); `conformance/run.sh --liella-headers`.
4. Plants file (`cargo xtask plants`): a `levels.tsv` cell, a template condition (features.h cascade), a `macros.tsv`
   value, a redirect. Then a pending decision record (`records new decision D-NEW --series D-X --story SL-HEADERS-S05`)
   for the marker format, the `__LIELLA_D_<name>` once-guards, the `__LIELLA_H_<path>` include guards, `__liella/`
   reserved for internal fragments, and the compiler-macro fallback below.
5. Add a differences row for `__STDC_ISO_10646__` (Liella 202309L, from Unicode 15.1, against 201706L in `macros.tsv`).
   Plan §0 and the brief ask for Liella's own value.

**Gaps (the slice-2/3 tables; need a probe extension, not a template).**
* `typedefs.tsv`/`visibility.tsv` have no rows for `int8_t`, `int16_t`, `int64_t` or any `int_leastN_t` or
  `int_fastN_t`. `stdint.h` uses the compiler's predefined `__INTn_TYPE__`/`__INT_LEASTn_TYPE__`/`__INTn_MAX__` for
  them. A table row wins when it exists, and `inttypes.h` prints which PRI/SCN sets are missing (least, fast).
  Recommendation: add C 7.20's names to the SL-HEADERS-2/3 probe candidates.
* `PTHREAD_STACK_MIN` outside `_DYNAMIC_STACK_SIZE_SOURCE`: no table value, so it is not defined there yet.
* `gnu/stubs.h` (Liella's ENOSYS-only functions) and `gnu-versions.h` (the interface versions): no table, so the
  headers ship with guards only. `gnu-versions.h` ships per §5.1.
* `FD_SET` and friends under fortify: plain macros. P3 will show whether the `__fdelt_chk` call sites matter.

**Do not redo.** The level table, the condition search, the templates' level cascade (verified against levels.tsv in
all 41 configurations).
