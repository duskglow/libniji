# libm.so.6's old (GLIBC_2.2.5) versions against their default versions (spec s0_6_c0_6_0; SYS-COMPAT-VERSIONS)

Black-box observations of the reference library (glibc 2.39, Ubuntu 24.04, x86_64, Zen 1 host), made on
2026-11-16 for GitHub issue #7: Debian trixie's `gawk` binds `fmod@GLIBC_2.2.5` and failed to start
on libniji, whose `libm.so.6` exported only `fmod@@GLIBC_2.38`. The reference exports 19 libm names at
`GLIBC_2.2.5` beside a newer default (the old error-handling wrappers); `spec/abi/compat-plan.tsv`
gave them `behaviour` rows, "alias if LG-0b shows identical results". This is that probe.

Probe: `crates/core/tests/oracle/libm_compat_versions.c`, built with `-DPROBE_ALL` (binds every old
version and the default version it is compared with, by `.symver`). Record:
[`libm-compat.glibc.txt`](libm-compat.glibc.txt) (a summary per name and every differing case).
Inputs: 22 values per argument (signed zeros, +-1, 0.5, 2.5, 7.5, -2, -3, values past the overflow and
underflow thresholds, +-1e-310 / +-1e-40 subnormals, +-1e308 / +-3e38, quiet and signalling NaN,
+-infinity); every pair for the two-argument functions (484 cases). Observed per call: the result's
bits, `errno` (cleared first), the invalid, divide-by-zero, overflow and underflow flags (cleared
first; whether a library function raises inexact is unspecified, C17 F.10, and the record leaves it
out; a first scratch run that included it showed the same old/default equality) and `signgam`
(preset to 7).
The same run with a `matherr` defined in the program (linked `-rdynamic`) gave the same text: the
reference never called it.

## Results

* `fmod`, `fmodf`, `exp`, `exp2`, `exp10`, `pow`, `hypot`, `lgamma` and the `float` forms (`expf`,
  `exp2f`, `exp10f`, `powf`, `hypotf`, `lgammaf`) and `lgammal`: the old version gives the default
  version's result bits, `errno`, flags and `signgam` in every case. libniji exports each as an alias
  of its default version (`alias` rows of compat-plan.tsv, generated stubs in
  `crates/libm-so/compat_aliases.S`); this story adds the ones Debian trixie's programs bind:
  `fmod`, `fmodf`, `exp`, `exp10`, `pow`, `hypot`, `hypotf` (handoff/SYS-COMPAT-VERSIONS.md has the
  counts). The others stay on spec/abi/compat-missing-libm.tsv until a program needs them.
* `log`, `log2`, `logf`, `log2f`: identical except for a negative argument (finite or -infinity):
  the old version returns a NaN with the sign bit clear (`7ff8000000000000`, `7fc00000`), the default
  version one with the sign bit set (`fff8000000000000`, `ffc00000`); `errno` (`EDOM`) and the flags
  (`FE_INVALID`) are the same. Not exported yet: an alias would give the default version's NaN.

Test: `old_libm_versions_bind_and_match_the_reference` (crates/core/tests/crt_products.rs) links the
probe (without `-DPROBE_ALL`) against libniji's `libm.so.6` and the reference's and requires the same
text, case by case.
