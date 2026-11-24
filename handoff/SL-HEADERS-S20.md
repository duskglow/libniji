# SL-HEADERS-S20: slice 20, switch and package (handoff, run 4 = SL-HEADERS-S20-4)

**State.** Branch `task/sl-headers-s20-4` (runs 1-3 on `task/sl-headers-s20[-2,-3]`, then run 4), origin/next
bf40b1b1c merged in (f05fd0ae1). Run 4 fix f811cf2d1, plant file 6e5d31009, this note and the report on top. Build
role keke:~/w/n1. Say "slice 20" in commits. `cargo xtask affected --list --base origin/next` escalates to FULL
(headers/*.h.in are generator inputs): the gate is the train's; this story proves itself with precheck, the rows
test by name and the `test` job.

**Decisions in force.** D-X.168 (drivers default to libniji's headers; coexist sysroot, ruling #8413; unshipped
headers a known limitation, #8419 B). D-X.169 (`__locale_t`, `__CORRECT_ISO_CPP_*`, ruling #8412). D-X.170 (run 3:
math.h's simd declarations). Ruling #8420 A: P4 part 2 is the follow-up SL-HEADERS-P4-CALLSITES. Ruling #8453 A:
the generator change (simd set, ` __NIJI_SIMD` in extern_lines) stays in this story.

**Run 4 (CI red on 85021878b: xtask `headers::tests::every_table_row_is_emitted_and_matches_the_tables`).** Cause:
the shared declaration emitter (`extern_lines`, functions.rs) appends ` __NIJI_SIMD` to every libmvec scalar's
declaration, but only math.h.in defined the macro. The rows test builds its units from the tables with
`<features.h>` only (rows.rs PRELUDE), so the identifier was undefined after the declarator and GCC read the rest
of each unit as an old-style parameter list ("storage class specified for parameter 'niji_status'"). Reproduced on
keke with a five-line unit against the generated headers. Fix (f811cf2d1): the `#ifndef __NIJI_SIMD` block moves
from math.h.in to features.h.in, beside `__NIJI_NOTHROW` (which the same emitter appends), so every unit that can
see such a declaration has the macro; conditions unchanged. libc-include regenerated: only features.h and math.h
change. D-X.170's text stays true (it says what math.h declares, not where the macro lives); not edited.

**Probe-settled (run 3, unchanged).** `_ZGV*` calls on the reference only with `__FAST_MATH__`; unmasked only; C
and C++, every -std; none for sinf32/sinf64/sinf32x. libniji matches. Known remaining difference (not this story):
strict `-std=c89/c++98/c++03` without the pragma, 11 functions diagattrs.tsv marks `__const__` vectorize on libniji
and not on the reference (follow-up: the slice-19 `__const__` rows' conditions).

**Jobs (keke), head 6e5d31009 (code) / final head in the report.** `sl-headers-s20-4-rows` (the rows test by name:
1 passed, 9.6 s), `sl-headers-s20-4-t1` (so, niji_cc_headers 5 passed incl. the simd comparison, deb_manifest
`coexist_lm_brings_in_libmvec_only_for_a_vector_call` passed), `sl-headers-s20-4-plants1`
(handoff/SL-HEADERS-S20-4-plants.txt: the definition reachable only through math.h; result in the report).
Run 3's plants (handoff/SL-HEADERS-S20-3-plants.txt) still cover the simd set, the emitter and the fast-math condition.

**Not to redo.** The deb_manifest libmvec guard, the per-function `_ZGV*` equality, the form rule (run 3).

**Next step.** Land (train). Then: diagattrs `__const__` conditions (above), SL-HEADERS-P4-CALLSITES,
SL-HEADERS-COMPLEX-TGMATH, SL-HEADERS-UNSHIPPED-REST, P6/P7 with `--niji-headers`, a .deb with the new rows.

**Plan rows to read.** plan/headers.md section 3 (P4), slice 20 row, 2.4 (rows test); D-X.168, D-X.169, D-X.170,
D-10.125.
