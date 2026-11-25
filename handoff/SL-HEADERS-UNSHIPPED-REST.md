# SL-HEADERS-UNSHIPPED-REST: thread_db.h, proc_service.h, aio.h, mqueue.h, math-vector-fortran.h (handoff)

**State.** Run 2 (SL-HEADERS-UNSHIPPED-REST-2) on branch `task/sl-headers-unshipped-rest-2`, stacked on
`task/sl-headers-unshipped-rest` (base origin/next 44304e709). Build role keke:~/w/n3. Final head, jobs and verdict:
`handoff/reports/SL-HEADERS-UNSHIPPED-REST-2.json`. Scratch on keke: `~/work/sl-headers-unshipped-rest-2/` (ref.sh =
the reference-corpus reproduction, table.sh = the diagattrs rerun, gen.sh = headers-gen with a changed-file list,
plants.txt = the copy the plants job ran). Run 1's scratch: `~/work/sl-headers-unshipped-rest/`.

**Run 1 (summary).** Shipped thread_db.h, proc_service.h, aio.h, mqueue.h through the generator tables; typedef from
its tag's layout rows; mq_open fortify (`mode2`, __mq_open_2); a debugger-shaped libthread_db client test;
`conformance/c/s11_aio_listio.c`; sys/procfs.h prgregset_t, elf_fpregset_t, prstatus_t, prpsinfo_t;
`headers/arrayparams.tsv` (array-form register-set parameters). Probe-settled details are in spec/abi/PROVENANCE.

**Run 2: why run 1's gate was red, and the fix.** Only
`headers::slices::s19::tests::diagnostic_attributes_match_the_reference` failed: attr_probe.py's reference-side
corpus did not build. Not the host: the kept corpus passes `gcc -fsyntax-only` and fails at `-O2 -c` with exactly
one error, inside the reference's mqueue checking wrapper: a 3-argument mq_open call (the corpus's sentinel test,
there because mqueue.h is now shipped) is a compile error inside a header, which attr_probe cannot blank as a test
line. Fix (e73347476): mq_open joins open and kin in `NO_SENTINEL`. With the corpus building, `attr_probe.py table`
(two runs byte-identical) adds 23 diagattrs.tsv rows, nonnull only, for aio_*, lio_listio*, mq_* (none for td_*
or ps_*); every earlier row unchanged; aio.h and mqueue.h regenerated. `cargo test -p xtask --bin xtask headers::`
24 passed, static clean. The brief's first step (run the test at origin/next) was replaced by this direct
reproduction, which names the cause outright and needed no second build of next in the role.
Also: run 1's plants td-decl-dropped and gregs-decayed were WARN-NO-MSG (the oracle helper panics before the
test's own assert); `msg:` now names the compiler error each one causes.

**Plants.** `handoff/SL-HEADERS-UNSHIPPED-REST-2-plants.txt`: run 1's five plus mq-open-sentinel (attr_probe.py) and
mq-timedsend-nonnull (diagattrs.tsv), all on the s19 or thread_db/aio tests.

**math-vector-fortran.h.** Not shipped. Ruling #8493: owed #266, waits for an R-H7 report or the break (a Fortran
output form of the generator and a gfortran test, on a host with gfortran). Run 1's probe: 168 `_ZGV*N*` names,
every one a libniji libmvec export.

**Decisions in force.** D-X.168 (unshipped list), D-10.125, R-H7; rulings #8492 (generator change accepted in this
story) and #8493 (above). No new D-row.

**Do not redo.** The reference-side table probes (PROVENANCE lists them); the Fortran probe.

**Exact next step.** None in this story once the gate and plants are green; follow-up is #266 (Fortran header).
