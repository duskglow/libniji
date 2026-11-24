# MATH-M6-F32-TRIG-2 handoff (2026-11-23): superseded; float trig per-call counts recorded

**State:** branch `task/math-m6-f32-trig-2`, cut from origin/next 0495c4684; no library change (this note and
`handoff/reports/MATH-M6-F32-TRIG-2.json` only). Role shiki:~/w/sh1 (26 vCPU host).

**Premise moved (challenge #8016, coordinator agreed: both stories close as superseded):** the work MATH-M6-F32-TRIG
handed back (sinf/cosf/tanf/sincosf fast paths, Cody-Waite pi/64) is already on next. It is in
`crates/math/src/fast32/trig.rs` (small-argument Cody-Waite reduction, a large-argument reduction `TRIG_LARGE`,
math-gen coefficients `coeffs::fast32_sin`/`fast32_cos`, per-function exhaustive tests in `fast32/tests.rs`). These
stories carried it: INTEGRATE-MATH-4/-5 (the original commit), LIBM-F32-SINF-SHRINK-4 (landed 11-05: kernel, margins,
8 plants caught), LIBM-SINF-IVB-RATCHET-4 (11-10) and LIBM-TINY-FASTPATHS-2 (11-12: large-argument bands). The old
branch (now `historic/task/math-m6-f32-trig`, 92aef4ef5) shares no merge base with the current next, so the CI
conflict in `fast32.rs` does not matter any more. Nothing was restacked or merged.

**Measured (the original brief's one open item, per-call instructions vs glibc), job `math-m6-f32-trig-2-percall`,
rc 0, every case's self-check `ok`:** `validation/wl_percall.sh --base none --spec validation/percall/math_f32.c`, next
0495c4684, instructions:u per call, glibc dynamic:

| case | glibc | libniji | ratio | earlier (MATH-M6-F32-BENCH-2-R, Skylake-SP host, older next) |
|---|---|---|---|---|
| sincosf_typ | 59.1 | 136.1 | 2.30 | 240.0 |
| sincosf_red | 92.0 | 152.9 | 1.66 | 1846.8 |
| sinf_typ | 43.2 | 97.9 | 2.27 | 152.0 |
| cosf_typ | 43.6 | 97.9 | 2.25 | 152.0 |
| sinf_red | 74.0 | 115.9 | 1.57 | 945.3 |
| cosf_red | 74.0 | 115.8 | 1.56 | 946.3 |
| tanf_typ | 106.5 | 105.9 | 1.00 | 160.0 |
| tanf_red | 136.9 | 123.8 | 0.90 | 1020.4 |

The earlier column is copied from that handoff (another host, older tree; not re-measured here). Step-0 lint
(`math-m6-f32-trig-2-lint`: clippy -D warnings all targets and features, then `cargo xtask static`) passed, rc 0. It
ran before the coordinator's note said to skip it.

**Open:** sinf/cosf/sincosf typical rows are still about 2.3x glibc in instructions; tanf is at or below glibc. If
those rows matter, they belong to a new performance story (profile first; the saturation register already records a
negative Estrin result for the cos polynomial on Ivy Bridge, docs/saturation/01-kernel-shared-work.md).

**Next step:** none for this story (the coordinator closes MATH-M6-F32-TRIG and -2 as superseded).
