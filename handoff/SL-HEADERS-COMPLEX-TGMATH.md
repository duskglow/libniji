# SL-HEADERS-COMPLEX-TGMATH: complex.h and tgmath.h (slice 11's deferred part), handoff

**State.** Branch `task/sl-headers-complex-tgmath`, base origin/next 44304e709. Build role keke:~/w/r5. The slice is
implemented and committed (`Ship complex.h and tgmath.h`); decision D-10.135 (amends D-10.125 (5); record `records/decisions/D-10/D-10.135.toml`). Precheck GREEN on 00d90742c. Report:
`handoff/reports/SL-HEADERS-COMPLEX-TGMATH.json`. Scratch (outside the tree): `~/w/.scratch/SL-HEADERS-COMPLEX-TGMATH/`
(`gen_tg.py` writes `headers/tgmath.h.in`; `grid2.py` prints a per-config grid of typegeneric.tsv).

**What is there.**
* `headers/complex.h.in`, `headers/tgmath.h.in`, rows in `headers/slices/s11.tsv`; generated `libc-include/complex.h`,
  `libc-include/tgmath.h`.
* `xtask/src/headers/slices/s11.rs` markers: `@@emit-c` (C configurations + every C++ one when C++98 shows the row),
  `@@if-c NAME [KIND]`, `@@if-probe NAME [FUNCTION]` (conditions from `spec/abi/typegeneric.tsv`; the `_FloatN`
  CMPLX forms take their functions' visibility rows, checked against the probe).
* `spec/abi/typegeneric.tsv`: reference probe (`spec/tools/tgmath_probe.py`, 23474 rows, 8 configurations), with a
  PROVENANCE paragraph.
* `spec/tools/header_checks/s11.py`: P1 of the two headers (C++ rows as the wrappers read them) and P6: the tree vs
  typegeneric.tsv in c89, c99, gnu, cxx98 (compile-only, `tgmath_probe.tree_compiler`, no products needed).
* `crates/boundary/tests/niji_cc_headers.rs::complex_and_tgmath_programs_match_the_reference`: eleven programs
  (Yohane shapes: csqrt run test, C99 detection, header coexistence, tgmath user, get/set dc, binding types, Python
  extension C and C++, C++98 `<complex.h>`), output equal to the reference's; C ones linked with libniji and run.
* Plants: `validation/headers_complex_tgmath.plants` (tgmath sin(float) -> sin; complex.h declarations missing in C89).
* Release note `docs/release-notes.d/SL-HEADERS-COMPLEX-TGMATH.md`; slice 20's known-limitation note and
  `validation/headers_unshipped.tsv` no longer list the two headers.

**Results (keke).** `tgmath_probe.py --check` (all 8 configs, before the clog10 fix): 8 differences (log10 of a
complex argument), fixed; header_check.py: 0 failed of 17234 (P6 4264 cases, 0 differ); the program test passes.

**Probe-settled facts.** See D-10.135. Narrowing macros: the builtin takes the exact-type function, else the first
listed that holds the argument; orders f32: (f64, f32x, f128, f64x), f32x: (f64x, f64, f128), f64: (f128, f64x);
`d*` call `d*l`, `f64x*` call `f64x*f128` for every type (complex accepted, as the reference). scalb takes
float/double/long double only; nexttoward has no _FloatN forms.

**Jobs (keke, on the code commit; later commits are handoff files only).** `sl-headers-complex-tgmath-lint`: clippy
-D warnings, `cargo xtask static`, `tgmath_probe.py --check` all 8 configs: 0 differences in 8502 cases.
`sl-headers-complex-tgmath-plants`: 2 CAUGHT, 0 survived, no WARN (results `handoff/SL-HEADERS-COMPLEX-TGMATH-plants.txt`).
`sl-headers-complex-tgmath-gate` (direct tier, 1 test target + xtask bounded set): 8 suites, 707 passed, 0 failed,
peak 2.3G. Superseded: `-gen1`, `-chk1` (8 log10 differences, fixed), `-hc1` (P1 of the C++11+ rows, fixed), `-hc2`.

**Open.** No ruling needed. Next: land. Remaining unshipped headers (validation/headers_unshipped.tsv): aio.h,
mqueue.h, proc_service.h, thread_db.h, math-vector-fortran.h.
