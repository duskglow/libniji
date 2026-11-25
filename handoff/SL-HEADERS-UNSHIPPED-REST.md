# SL-HEADERS-UNSHIPPED-REST: thread_db.h, proc_service.h, aio.h, mqueue.h, math-vector-fortran.h (handoff)

**State.** Branch `task/sl-headers-unshipped-rest` (from origin/next 44304e709). Build role keke:~/w/n3. Final head
and verdict: `handoff/reports/SL-HEADERS-UNSHIPPED-REST.json`. Scratch on keke: `~/work/sl-headers-unshipped-rest/`
(t5.sh = the final test job; csps/ and p3/ = the reference probes of leg 3). The Fortran probe is in the same path on shiki.

**Commits (all this story's).** 1115454a3 ship thread_db.h, proc_service.h, aio.h, mqueue.h; 2d347eaf8 typedef from
its tag's layout rows; 4761f0be5 mq_open fortify (`mode2`, __mq_open_2); 286350a1a debugger-shaped libthread_db
client test + slice 14 P1 skips; ade508a1c `conformance/c/s11_aio_listio.c` (TESTS: line); 6daae886d release notes,
unshipped-row reason, plants file; 9d7c410f4 proc_service.h rows of callsites.tsv/noexcept.tsv (reference probe);
b5648a307 the C-linkage differences row narrowed to td_*; 118c52ad2 sys/procfs.h prgregset_t, elf_fpregset_t,
prstatus_t, prpsinfo_t (reference probes; the client did not build without prgregset_t); then this note and report.

**Results at 118c52ad2 (job sl-headers-unshipped-rest-t5 on keke; its static step was still running at the end of
leg 3: read `~/logs/sl-headers-unshipped-rest-t5.log`).** headers-gen leaves libc-include unchanged; rows test and
mq_open fortify test green; header_check 0 failed of 17462; conformance `s11_aio_listio s25_mqueue` 2 passed,
`--niji-headers s25_mqueue s11_private_descriptors s18_fork_reset x_self_bind` 4 passed; P4 part 1 0 FAIL every -std;
clippy (workspace, all targets, all features, -D warnings) clean. **RED: thread_db_products
`a_debugger_shaped_client...` fails** (so plants-1 and plants-2 aborted at its baseline: BASELINE FAILED for
td-decl-dropped; the other copy's plants lio-write-wrong and mode2-arity: read
`~/logs/sl-headers-unshipped-rest-plants-2.log`). SUPERSEDED: t3, t4 (stopped, older heads), gen3/probe3 (tools).

**The open defect (exact next step).** The client defines `ps_lgetregs (..., prgregset_t)` and `ps_lsetregs (...,
const prgregset_t)`; libniji's proc_service.h declares them with the decayed `elf_greg_t *` (prototypes.tsv spells the
decayed type), so gcc's `-Werror=array-parameter` rejects the definition (`argument 3 of type 'long long unsigned
int[27]' with mismatched bound ... previously declared as 'long long unsigned int *'`). On the reference the client
builds with -Wall -Wextra -Werror, so the reference keeps the array form (form, not only value, rule of 11-24). The
same applies to td_thr_getgregs/td_thr_setgregs in thread_db.h (prgregset_t) if they are declared decayed: check.
Fix: (1) black-box probe on the reference: per function, a definition with the array-typedef parameter compiled with
`-Werror=array-parameter` after including the header (passes = the reference declares the array form); (2) a
generator input that spells a parameter by its typedef (no mechanism exists: grep found none; e.g. a column or a
headers/*.tsv table `function, parameter index, typedef`, used by functions.rs `declaration`), rows for the probed
functions; (3) headers-gen + rows test; (4) rerun t5.sh (`~/work/sl-headers-unshipped-rest/t5.sh`, label -t6) and
plants (new dir plants-3); (5) precheck `--jobs <t6>,<plants-3> --head <sha>`, report json, handback-check.
Budget note: leg 3 used ~130 calls; the fix plus reruns is ~20 calls.
**Also RED: `cargo xtask static` fails its `abi-types` check** at 118c52ad2 (t5 ended; `~/work/sl-headers-unshipped-rest/
t5-static.log` on keke names what): most likely the 53 new layout rows (prgregset_t, elf_fpregset_t, prstatus_t,
prpsinfo_t) in crates/abi need a registration abi-types expects. Read that log first; fix with the defect above.

**Probe-settled (reference, black box).** proc_service.h's 15 ps_* functions: plain symbol in every configuration,
C++ included, no exception specification (libniji identical). sys/procfs.h gives prgregset_t (216 bytes),
elf_fpregset_t (512), prstatus_t (336), prpsinfo_t (136) in every configuration but time64only, also through
thread_db.h and proc_service.h. aio program: built on the reference's headers and libc (gcc -Wall -Wextra), passes.

**P4 part 2 (classification).** Differs from spec/abi on next too, in headers this branch does not change (ctype.h
`*_l` call rows, libintl.h, argp.h `!error`, math.h isinf/isnan, stdbit.h noexcept, sys/pidfd.h, `__x` names: not
this story's; SL-HEADERS-P4-CALLSITES runs on it). thread_db.h's 120 lines are the documented C-linkage difference
(differences row "pidfd_* ... td_*"). proc_service.h, aio.h, mqueue.h: 0 lines.

**math-vector-fortran.h (not shipped, RULING NEEDED).** Probe on the reference (gfortran 13.3 on shiki, one loop per
intrinsic, -O3 -ffast-math at default/-mavx/-mavx2/-mavx512f, `nm -u` only): the pre-included file vectorises sin cos
tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh exp log log10 erf erfc hypot pow (`**`) in real(4) and
real(8), ISA classes b/c/d/e: 168 `_ZGV*N*` names, every one among libniji's libmvec exports (all 216 of
versions.tsv), so nothing would be left out. gamma, log_gamma, bessel_* and sqrt stay scalar. NOT VERIFIED: what
gfortran does under niji-gcc's include order without the file. Shipping needs a Fortran
output form of the generator (a `!` banner, one directive per function from a table) and a gfortran test; the build
role has no gfortran. Recommend a follow-up story on a host with gfortran.

**Decisions in force.** D-X.168 (unshipped list), D-10.125, R-H7. No new D-row (mode2, the event-set macros and the
table rows are probe-settled local details).

**Exact next step.** None in this story if the hand-back is accepted; follow-up: the Fortran generator form above.
