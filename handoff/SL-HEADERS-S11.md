# SL-HEADERS-S11 / -2 / -3: slice 11, math (handoff after run 3)

**State.** Branch `task/sl-headers-s11-3` (stacks on `task/sl-headers-s11-2`). Say "slice 11" in commits and handoffs.
Run 3 began with clippy + static green on the inherited head (job `sl-headers-s11-3-lint0`, rc 0).

**Done in run 3.**
* **ieee754.h written, generated, nothing hand-typed.** `headers/ieee754.h.in` is a frame plus `@@emit`; the three unions
  come from `layouts.tsv` member rows. `abi_extract.py` now probes `union ieee754_float|ieee754_double|ieee854_long_double`
  (`--layouts-only` on the build host regenerated `layouts.tsv`: +40 rows). The extractor wrote `bits W@0` for every bit-field
  (DWARF 4 `DW_AT_bit_offset` counts from the top of the storage unit); a row is now `bits W@K`, K = first bit from the start of the
  type (46 old rows of 9 other types changed in K only; the abi generator reads W only, so `layout.rs` changed only by the three unions).
  `headers.rs` `bitfield_aggregate`: writes the bit-fields in table order and checks each K against the allocation of
  consecutive `unsigned int` bit-fields (no field across a 32-bit unit); a mismatch is a generation error.
  `cargo xtask headers-gen` + `abi-gen` run (`crates/abi/src/layout.rs`, `crates/abi/tests/appendix_c_layouts.rs` regenerated;
  `cargo xtask stubs` and `cargo xtask test` in job `sl-headers-s11-3-chain`).
* **Proof of the field positions:** `spec/tools/ieee754_fields.c` (P6, gnu config in `header_check.py`): 26 checks that build a value
  from one field and compare it with the number the format gives it (binary32, binary64, x87 extended; quiet bit, integer bit,
  sign, exponent, mantissa halves). A union read is not constant in C, so the unit is compiled `-O1 -S` and each check calls an
  `error`-attribute function that survives only when the comparison is not folded true (`niji: codegen` marker in the file).
  `header_check.py` also covers `union` kinds in P1 (sizes, alignments) and lists `ieee754.h` in P1/P5: `0 failed of 2009`.
* **P2/P3 classified (job `sl-headers-s11-3-p23` then `-gen2` on the final generator).** The first run found one real defect: the
  reference declares the functions `int isinf (double)` and `int isnan (double)` beside the macros (default/BSD/SVID/GNU configs;
  xopen for `isnan`); `@@if isnan` owned the whole name so the function rows were skipped. New directive `@@if-macro NAME` (owns
  the macro only); math.h.in uses it for `isnan`/`isinf`; math.h regenerated; tgmath.h follows via math.h. After the fix every
  remaining P2 row (3659) and P3 row (116) differs only in C++ configurations or is a reserved name (`_LIBC_LIMITS_H_`): the
  table was probed with the reference's g++ and libstdc++'s wrapper headers (`<cmath>`/`<ccomplex>` pull names in), the
  libniji probe uses `-nostdinc` and has no wrappers. Classifier: `cls.py` (config sets with the cxx ids removed, per row;
  scratch, not in the tree). C configurations: 0 differences.
* Pending record `records/pending/SL-HEADERS-S11-3-1.toml` (D-NEW, D-10): ieee754.h, bit offsets, field proof, `@@if-macro`.
  (Run 2's `SL-HEADERS-S11-2-1.toml` stays.)
* Plants: `handoff/SL-HEADERS-S11-3/plants.txt` (run 2's 11 plus 12 new: header text, table row, generator allocation check,
  the proof's expectations, `@@if-macro`).

**Not done / open.**
1. **`<cmath>` and `<complex>` in every -std against libniji's headers, and the C++ visibility rows.** Not provable yet: libstdc++'s
   wrappers need headers other slices own (a build with libniji's `features.h` in front of the system's other headers fails inside
   the system's `bits/`). Re-run once the C headers of all slices exist: `g++ -std=<each> -isystem libc-include -fsyntax-only` on a
   TU with `<cmath>`, `<complex>`, `<math.h>`, `<complex.h>` (script `cxx.sh`, scratch). The P2/P3 C++11+ rows should then vanish.
2. **P6 libm conformance programs:** only 5 conformance programs include slice-11 headers (`fpu_control_default`, `fpu_control_own`,
   `s10_sqrt_env`, `s10_sqrtl_entry`, `s08_strto_fenv_flags`); run `--niji-headers` in the chain job (result in the report).
3. `iseqsig` (the relational operators raise "invalid" for a quiet NaN) was not probed against the reference.

**Decisions in force.** Ruling #6395 (iscanonical): no difference from the reference, no compat flag (run 2, see git history of this
file's earlier text: the pseudo-denormal is non-canonical in spec s10.2, the probe and `__iscanonicall`). D-NEW rows in `records/pending/`.

**Next step.** Item 1 when the other slices' headers land; otherwise the slice is complete.

**Run 4 (close-out).** math.h.in `@@owns-macro` no longer lists isnan/isinf (the `@@if-macro` lines own them; the s11-ifmacro-owner plant now fails). Three float plants take msg `headers-gen: stale under|panicked at`. Plants job mei:sl-headers-s11-4-plants 23 CAUGHT/0 WARN; gate mei:sl-headers-s11-4-gate rc 0. Next: item 1 above.

**Run 5 (merge-only).** Merged origin/next (HANDBACK-INLOOP-CHECKS-2 landed). Conflicts: header_check.py kept both (HEADERS = ... + SLICE9 + slice-11 list; P1 kinds include union with SLICE6+SLICE9+SLICE14), headers.rs kept both cxx_mask and is_covered. headers-gen, clippy, static green on the merge; gate mei:sl-headers-s11-5-gate rc 0; plants mei:sl-headers-s11-5-plants 23 CAUGHT / 0 WARN. Next: item 1 above when other slices land.

**Run 5 end state.** Green against the origin/next of 11-18 morning, but next moved again (f9aa8e368 "slice tables and slice modules") and conflicts in header_check.py and headers.rs again. A plain merge no longer works: slice 11 must be re-ported into headers/slices/s11.tsv, spec/tools/header_checks/s11.py and xtask/src/headers/slices/s11.rs (its HEADERS entries, P1 union kind, ieee754 field proof, @@emit-c/@@if-macro/@@owns-macro markers, cxx_mask). Next step: that re-port as its own story, then rerun gate and plants.
