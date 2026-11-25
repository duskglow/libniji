# SL-HEADERS-UNSHIPPED-REST: thread_db.h, proc_service.h, aio.h, mqueue.h, math-vector-fortran.h (handoff)

**State.** Branch `task/sl-headers-unshipped-rest` (from origin/next 44304e709). Build role keke:~/w/n3. Final head
and verdict: `handoff/reports/SL-HEADERS-UNSHIPPED-REST.json`. Scratch on keke: `~/work/sl-headers-unshipped-rest/`
(t5.sh = the final test job, run as label -t6; arrprobe.sh = the leg-4 reference probe; csps/ and p3/ = the probes
of leg 3). The Fortran probe is in the same path on shiki.

**Commits (all this story's).** 1115454a3 ship thread_db.h, proc_service.h, aio.h, mqueue.h; 2d347eaf8 typedef from
its tag's layout rows; 4761f0be5 mq_open fortify (`mode2`, __mq_open_2); 286350a1a debugger-shaped libthread_db
client test + slice 14 P1 skips; ade508a1c `conformance/c/s11_aio_listio.c` (TESTS: line); 6daae886d release notes,
unshipped-row reason, plants file; 9d7c410f4 proc_service.h rows of callsites.tsv/noexcept.tsv (reference probe);
b5648a307 the C-linkage differences row narrowed to td_*; 118c52ad2 sys/procfs.h prgregset_t, elf_fpregset_t,
prstatus_t, prpsinfo_t (reference probes); d5ce6c483 typedefs.tsv rows sorted (static abi-types was red: the probe
splice had appended them as blocks; vs origin/next the file only gains rows); c13ab180f `headers/arrayparams.tsv`:
ps_lgetregs, ps_lsetregs, td_thr_getgregs, td_thr_setgregs declare their register set as `unsigned long long [27]`
(the array form of prgregset_t), two more plants; then this note and the report.

**Results (final head, job sl-headers-unshipped-rest-t6 on keke, plants sl-headers-unshipped-rest-plants-3).** See
the report json for the numbers. SUPERSEDED: t3, t4 (older heads), t5 (static abi-types red, client test red),
plants-1, plants-2 (baseline of td-decl-dropped failed on the client test), gen3/probe3/gen4 (tools).

**Probe-settled (reference, black box).** proc_service.h's 15 ps_* functions: plain symbol in every configuration,
C++ included, no exception specification (libniji identical). sys/procfs.h gives prgregset_t (216 bytes),
elf_fpregset_t (512), prstatus_t (336), prpsinfo_t (136) in every configuration but time64only, also through
thread_db.h and proc_service.h. Array form (leg 4, arrprobe.sh): a definition after the header with a prgregset_t
parameter passes `-Werror=array-parameter`, one with `elf_greg_t *` fails ("declared as a pointer"), for all four
functions; td_thr_setgregs's parameter is not const (a `const prgregset_t` definition: conflicting types). aio
program: built on the reference's headers and libc (gcc -Wall -Wextra), passes.

**P4 part 2 (classification).** Differs from spec/abi on next too, in headers this branch does not change (ctype.h
`*_l` call rows, libintl.h, argp.h `!error`, math.h isinf/isnan, stdbit.h noexcept, sys/pidfd.h, `__x` names: not
this story's; SL-HEADERS-P4-CALLSITES runs on it). thread_db.h's lines are the documented C-linkage difference
(differences row "pidfd_* ... td_*"). proc_service.h, aio.h, mqueue.h: 0 lines.

**math-vector-fortran.h (not shipped, RULING NEEDED).** Probe on the reference (gfortran 13.3 on shiki, one loop per
intrinsic, -O3 -ffast-math at default/-mavx/-mavx2/-mavx512f, `nm -u` only): the pre-included file vectorises sin cos
tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh exp log log10 erf erfc hypot pow (`**`) in real(4) and
real(8), ISA classes b/c/d/e: 168 `_ZGV*N*` names, every one among libniji's libmvec exports (all 216 of
versions.tsv), so nothing would be left out. gamma, log_gamma, bessel_* and sqrt stay scalar. NOT VERIFIED: what
gfortran does under niji-gcc's include order without the file. Shipping needs a Fortran output form of the
generator (a `!` banner, one directive per function from a table) and a gfortran test; the build role has no
gfortran. Recommend a follow-up story on a host with gfortran.

**Decisions in force.** D-X.168 (unshipped list), D-10.125, R-H7. No new D-row (mode2, the event-set macros, the
array-form table and the table rows are probe-settled local details).

**Exact next step.** None in this story if the hand-back is accepted; follow-up: the Fortran generator form above.
