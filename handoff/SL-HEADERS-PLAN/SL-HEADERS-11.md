# SL-HEADERS-11: math

Id: SL-HEADERS-11
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6, ruling R-H5
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 11; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. The type-generic macros use the compiler builtins its manual documents. `tgmath.h` uses GCC's `__builtin_tgmath`. There are no `simd` declarations (R-H5).

Headers (plan/headers-inventory.tsv, slice 11): `math.h`, `complex.h`, `tgmath.h`, `fenv.h`, `fpu_control.h`, `ieee754.h`, `values.h`, `finclude/x86_64-linux-gnu/math-vector-fortran.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. The libm conformance programs. The type-generic macros on float, double, long double and _Float128. C++ `<cmath>` and `<complex>` in every -std.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
