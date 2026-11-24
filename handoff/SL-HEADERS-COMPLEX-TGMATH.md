# SL-HEADERS-COMPLEX-TGMATH: complex.h and tgmath.h (slice 11's deferred part), handoff

**State.** Branch `task/sl-headers-complex-tgmath`, base origin/next 44304e709. Build role keke:~/w/r5. Leg 1 (WIP)
committed: the probe tool `spec/tools/tgmath_probe.py` and the slice-11 generator markers
`xtask/src/headers/slices/s11.rs` (not compiled yet). No templates yet. Scratch (outside the tree):
`~/w/.scratch/SL-HEADERS-COMPLEX-TGMATH/` (vis.py, tgsum.py, grid.py summarisers; ref1.tsv = first probe run).

**Job running at hand-off:** keke `sl-headers-complex-tgmath-ref2` (reference probe, about 20 min, writes
`~/work/sl-headers-complex-tgmath/ref2.tsv` on keke). When it ends, copy it to `spec/abi/typegeneric.tsv`
(fetch: `"${VSSH[@]}" 'gzip -c ~/work/sl-headers-complex-tgmath/ref2.tsv | base64 -w0' | base64 -d | gunzip`),
add a PROVENANCE paragraph (tool, GCC 13.3, glibc 2.39, row count).

**Design (decided, leg 1).**
* visibility.tsv's C++ rows of complex.h/tgmath.h are the C++ library's: strict `-std=c++11..23` show `<complex>`
  and `<cmath>` (about 2,700 / 870 names), C++98 shows the C header (the libstdc++ wrapper does `#include_next`).
  The C header's content does not depend on the C++ standard. So `@@emit-c` (s11.rs) emits a row under its C
  configurations plus, when cxx98 shows it, every C++ configuration; rows only C++11+ show are not emitted.
  `@@if-c NAME` = `#if` with that condition for a macro row (template-defined function-like macros).
  This keeps the slice contract (only `slices/s11.rs`, no emitter change).
* complex.h facts (probe ref1): `I`, `_Complex_I` float _Complex constants in every C config, and in C++;
  `imaginary`, `_Imaginary_I` never defined; `complex` defined in C and C++98, undefined in gnu++17 (the wrapper
  #undefs it); CMPLX/CMPLXF/CMPLXL defined in gnu17, c2x, gnu (C11 level: `__NIJI_ISOC >= 2011`), not c89/c99;
  CMPLXF32/F64/F128/F32X/F64X only with _GNU_SOURCE; all `const` (static initializers); in C++ CMPLX* are defined
  but a use does not compile (write them with `__builtin_complex`, C-only, GCC manual). CMPLX* and `_Complex_I`
  have no visibility rows: their condition needs a marker reading typegeneric.tsv (planned `@@if-probe NAME`
  in s11.rs: mask of the probe configs where the name is not `undefined`, `condition()` over those configs).
* tgmath facts (ref1, `(void)` cast version; ref2 stores the result, so const functions show their calls):
  C2X rules with _FloatN (GCC's documented `__builtin_tgmath`); integer args -> double; complex args select c*;
  fabs(complex) -> cabs*; real-only macros reject complex (error); `d*` narrowing macros (dadd, dsqrt, ...) call
  the `*l` function for every real type, even accept complex (a plain call `daddl (x, y)`), _Float128 via
  __trunctfxf2; fadd/fsub/fmul/fdiv/fsqrt/ffma: fadd for float/double/_Float32/_Float64/_Float32x/ints, faddl for
  long double/_Float64x, error for _Float128 and complex; f32add: f32addf64 for float/double/_Float32/_Float64/ints,
  f32addf128 for long double (extended to _Float128) and _Float128, f32addf32x, f32addf64x; f32x*: f32xaddf64x for
  float/double/ld/ints, f32xaddf64 for _Float64, f32xaddf128; f64*: f64addf128 for most (extended), f64addf64x for
  _Float64x; f64x*: f64xaddf128 always. scalb and nexttoward reject _FloatN. c89 and c99 strict: _FloatN args
  error; c89: most macros `error` (defined, not compilable), carg-type complex-only ones work. C++98: every tgmath
  macro defined but no use compiles. gnu++17: names of <cmath>, not macros (wrapper; libniji's not reached).
* complex.h is reached in C++ only through the C++ library's wrapper (scripts/niji-cc puts the C++ library's
  directories above libniji's), so the probe's C++ configurations go through `scripts/niji-cc -x c++`.

**Next steps.**
1. `headers/complex.h.in`: features.h, `__NIJI_BEGIN_C`, `@@owns-macro complex I`, `@@emit-c`, end C, then
   `@@if-c complex` / `# define complex _Complex`, `_Complex_I` `(__extension__ 1.0iF)`, `I`, CMPLX family with
   `__builtin_complex` under the probe conditions.
2. `headers/tgmath.h.in`: `#include <math.h>`, `#include <complex.h>`, `@@owns-macro` every tgmath macro, `@@emit-c`,
   one `@@if-c M` + `# define M(...) __builtin_tgmath (...)` per macro (helper macros `__NIJI_TG*` pasting `f`,
   `l`, `c` prefixes; the _FloatN functions only where math.h/complex.h declare them); d* macros as plain calls.
3. `headers/slices/s11.tsv`: add rows `complex.h 0 -` and `tgmath.h 0 -`; `cargo xtask headers-gen`; fix errors.
4. Compare: on keke `cargo xtask so`, then `python3 spec/tools/tgmath_probe.py --niji-cc scripts/niji-cc --check
   spec/abi/typegeneric.tsv` (batched, about 1 min); iterate until 0 DIFF (or each remaining one a difference row).
5. header_checks/s11.py: `problems()` calls `tgmath_probe.check(...)` when target/niji has products (skip with a
   printed reason otherwise); remove the "not in this slice yet" note in `units()`.
6. C and C++ programs (brief item 4, 4b): a test fn in crates/boundary/tests/niji_cc_headers.rs (no new file)
   compiling and running with niji-cc: csqrt/cpow/CMPLX/tgmath results printed, compared with the same program
   built by system gcc; Yohane shapes (efivar tgmath, openssl rsa_complex, postgresql meson C99 probe, mpc
   mpc_get_dc/set_dc, guile csqrt run test, cython C and C++).
7. Plants (docs/plants.md style file handoff/SL-HEADERS-COMPLEX-TGMATH-plants.txt): tgmath macro resolving to the
   double function for float; a complex.h declaration missing in one configuration. Run `cargo xtask plants`.
8. Release-notes fragment; D-row (reserve with `ops reserve --story SL-HEADERS-COMPLEX-TGMATH D-10`) amending
   D-10.125 item (5); clippy, static, precheck, handback-check.

**Plan rows to read.** plan/headers.md §2.4, §4.1 (slice contract); D-10.125; handoff/HEADERS-TABLES-COMPLETE-2.md.
