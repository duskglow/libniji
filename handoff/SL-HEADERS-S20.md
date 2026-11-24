# SL-HEADERS-S20: slice 20, switch and package (handoff, run 3 = SL-HEADERS-S20-3)

**State.** Branch `task/sl-headers-s20-3` (runs 1-2 on `task/sl-headers-s20[-2]` + run 3), base origin/next a0d93596e.
Run 3 code head 16327811b (this note and the report on top). Build role keke:~/w/n1. Say "slice 20" in commits.
`cargo xtask affected --list --base origin/next` escalates to FULL (headers/*.h.in are generator inputs): the gate is
the train's; this story proves itself with precheck + the `test` job.

**Decisions in force.** D-X.168 (drivers default to libniji's headers; coexist sysroot, ruling #8413; unshipped
headers a known limitation, #8419 B). D-X.169 (`__locale_t`, `__CORRECT_ISO_CPP_*`, ruling #8412). D-X.170 (run 3:
math.h's simd declarations). Ruling #8420 A: P4 part 2 is the follow-up SL-HEADERS-P4-CALLSITES.

**Run 3 (CI red on 65aecb5cb: deb_manifest coexist_lm_brings_in_libmvec_only_for_a_vector_call).** Cause: libniji's
math.h declared no simd variants, so `niji-gcc -O3 -ffast-math` emitted no `_ZGV*` call and libmvec was never
linked. Fix (16327811b): headers-gen reads the libmvec.so.1 rows of versions.tsv (54 scalars, each needing exactly the
unmasked b/c/d/e variants, else a generator error) and appends `__NIJI_SIMD` to their declarations (only when bound
to their own name); math.h.in defines it `__attribute__ ((__simd__ ("notinbranch")))` under `__FAST_MATH__` when
`__has_attribute (__simd__)`. No prototype or type changes.
Probe-settled on the reference (gcc 13, keke; probe script and tables in ~/work/sl-headers-s20-3/: probe.sh,
ref.tsv, niji.tsv, ref2.tsv, niji2.tsv): `_ZGV*` calls only with `__FAST_MATH__` (`-U__FAST_MATH__`, `-fopenmp`,
`-fopenmp-simd` alone: none); unmasked only; C and C++, every -std (strict c89/c++98 under `#pragma omp simd` too);
none for sinf32/sinf64/sinf32x. libniji matches in all of these.
Known remaining difference (not this story's cause): strict `-std=c89/c++98/c++03` without the pragma, 11 functions
diagattrs.tsv marks `__const__` (sinf cosf tanf atanf tanhf asinh asinhf cbrt cbrtf erf erff) vectorize on libniji
and not on the reference: the reference does not treat them as const in strict modes (CSE probe: 2 calls vs 1;
`errno = 0; sinf(x); return errno;` folds to 0 on libniji under -std=c89, reads errno on the reference). Follow-up:
the slice-19 `__const__` rows should apply only where the reference treats the function as const.

**Jobs (keke), head 16327811b.** `sl-headers-s20-3-t1` (test: so, niji_cc_headers incl. the new simd comparison,
deb_manifest's libmvec guard, P4 --no-noexcept), `sl-headers-s20-3-plants1` (3 plants,
handoff/SL-HEADERS-S20-3-plants.txt, `--timeout 900`: each plant regenerates headers and rebuilds products).

**Results.** t1 green (niji_cc_headers 5 passed incl. the simd comparison; deb_manifest libmvec guard passed;
P4 C++ part 0 FAIL, 2 known per c++98/03). plants1: 3 CAUGHT, 0 warnings. Precheck on 5cfb37efa: everything ok except
`slice-contract` RED (a header slice changing the shared generator: RULING NEEDED, recommend A = accept here;
B = split into a generator story first).

**Next step.** On ruling A: land (train). On B: move headers.rs/functions.rs changes to a generator story. Then: diagattrs `__const__` conditions (above), SL-HEADERS-P4-CALLSITES,
SL-HEADERS-COMPLEX-TGMATH, SL-HEADERS-UNSHIPPED-REST, P6/P7 with `--niji-headers`, a .deb with the new rows.

**Plan rows to read.** plan/headers.md section 3 (P4), slice 20 row; D-X.168, D-X.169, D-X.170, D-10.125.
