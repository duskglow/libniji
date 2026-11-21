# LIBM-LGAMMAF-EDGE handoff (run 4, LIBM-LGAMMAF-EDGE-4: every typical row at or below base)

State: branch `task/libm-lgammaf-edge-4` (continues -3 and -2), code head e91d39e2c, stacked on af1c82203 (the merge
base with origin/next). Role shiki:~/w/sh1. Decision: pending record records/pending/LIBM-LGAMMAF-EDGE-2-1.toml
(series D-10; the coordinator assigns the id). Spec: ~/state/briefs/LIBM-LGAMMAF-EDGE.md plus the -2, -3 and -4
briefs. Full report (percall per commit, the trace, jobs): handoff/LIBM-LGAMMAF-EDGE-report.md. Plants with results:
handoff/LIBM-LGAMMAF-EDGE-4-plants.txt (11/11 CAUGHT on e91d39e2c; the input file is handoff/LIBM-LGAMMAF-EDGE-2-plants.txt).

## Done
- Runs 2-3: `lgamma_reflect` (x < -42, 8.2 ulp bound, margin 16), `lgamma_rest` (second try, then the 18-entry
  table), `special::lgamma` making one cold call (`lgammaf_after_first`) on a first-try refusal.
- Run 4: lgammaf_zero's +4 was traced (wl_trace, one call, head vs base): two refusal call sites in
  `lgamma_core_signed` (`lgamma_wide` and `lgamma_edge`, which base had merged into one) and x passed twice to
  `lgammaf_after_first` kept x in xmm9, saved around the call, moved the result pointer to r15 and pinned the abs mask.
  Fix: both refusals call `lgamma_edge(xd)`; x < -42 (outside the kernel's domain) leaves `lgamma_edge` on its
  `!domain` branch through the cold `lgamma_wide_cold`; `lgamma_rest`'s recompute inlines `lgamma_wide`;
  `lgammaf_after_first` takes x alone.
- Percall e91d39e2c vs base af1c82203 (instructions:u per call): zero 1053 (1056), pos 323.8 (325.8), neg 371.5
  (372.6), r_neg 378.5 (379.6), big 361 (302860), tab 647.4 (272874), retry 1238 (484530).
- Negative trial (reverted in e91d39e2c): `lgamma_wide` inlined into `lgamma_edge` made zero 1056 (+3, prologue).

## Open / next
- Nothing left in this story's scope. The cold rows are about 18 instructions above the -3 head's (big 343 -> 361,
  tab 631 -> 647): reaching the reflection through `lgamma_edge` pays its domain test and one more call. Every other
  layout tried cost lgammaf_zero 3 to 4 instructions.
- Follow-up #5641 (NOT in scope): `lgamma_edge`'s own refusals (608 operands, |x| < 2^-20) still take the accurate
  path; a margin of 5 for the tiny piece would cut 594 to about 40.
Not to redo: the prerequisite check, the error analysis, the enumeration (current for margin 16), the trace above.
