# ERROR-H-NORETURN handoff (leg 1 -> leg 2)

Branch task/error-h-noreturn (from origin/next e3b0caf60). Role keke:~/w/r3. Scratch: ~/w/.scratch/ERROR-H-NORETURN/ (local),
~/work/error-h-noreturn/ (keke: probe/ caller matrix, diag.tsv from the probe tool).

## Done (leg 1, WIP commit)
* Caller matrix (scratch probe/matrix.sh; callers.c: status 0, constant 1, variable, for error and error_at_line; marker
  kept = control returns; -Wreturn-type lines; gcc 13.3 only: NO clang on keke, mei, shiki or kanon, installing one
  needs root, not attempted):
  - reference headers (black box): -O0 all kept; -O2 constant-1 calls dropped, status 0 and variable kept; -Wreturn-type on all 6.
  - A (no noreturn): all kept at -O0 and -O2; -Wreturn-type on all 6.
  - B (origin/next, noreturn always): NOTHING kept even for status 0 (the bug), no -Wreturn-type.
  - C (A + GCC-optimizing always-inline wrapper calling a noreturn asm alias only for a nonzero constant status):
    identical to the reference at -O0 and -O2. -Wmaybe-uninitialized caller separated none (gcc folds it).
  Chosen: C. Macro wrapper rejected (a `#define error(...)` breaks `obj->error (...)` callbacks).
* diagattrs.tsv: error, error_at_line noreturn `1` -> `nonzero:1`. attr_probe.py: new nz.k / nv.k tests write
  `nonzero:K` (so a table regeneration cannot bring the bug back). headers.rs: loads `nonzero:K`, rejects other
  values, `__noreturn__` only for `1`. functions.rs: `nonzero_wrapper` emits C's wrapper (GCC, __OPTIMIZE__, not C++).
* Job error-h-noreturn-hgen (keke, kind test) started: headers-gen, probe-table diff vs the edited table, matrix.
  Read ~/logs/error-h-noreturn-hgen.log (expect probe-diff-rc=0; if headers-gen rewrote libc-include/error.h, commit it).

## Audit (step 2): the other noreturn=1 rows
_Exit, _exit, exit, quick_exit, abort, pthread_exit, thrd_exit, __pthread_unwind_next, longjmp, _longjmp, siglongjmp,
err, errx, verr, verrx: none returns for any argument by its specification (exit family and abort terminate;
pthread_exit/thrd_exit end the thread; the longjmp family transfers control; err/verr(3) always exit). They stay `1`.
Only error and error_at_line return (status 0). The probe tool's nz/nv tests confirm this mechanically when the diff is 0.

## Next steps (leg 2)
1. Read the hgen log; commit regenerated libc-include/error.h (never hand-edit it).
2. Test: conformance/run.sh gets two directives: `/* NIJI-HEADERS: yes */` (build that program against target/niji/include
   + gcc include + target/niji/uapi, as --niji-headers does, in every mode) and `/* OPT: -O2 */` (flags="$flags -O2").
   New conformance/c/s02_error_returns.c (spec clause: s02 error/error_at_line return when status is 0; check
   spec/clauses.tsv for the id): `volatile int after = 0; error (0, 0, "x"); after = 1; error_at_line (0, 0, "f", 1, "y");
   after |= 2;` then exit(after == 3 ? 0 : 1), plus a variable-status call; EXPECT-STDERR lines.
3. docs/plants.md: plant on libc-include? No: plant headers/diag source = spec/abi/diagattrs.tsv `error\t...nonzero:1` ->
   `1` (the header regenerates in `cargo xtask so`?) - check how plants reach generated headers; msg:s02_error_returns.
4. D-row: reserve with `ops reserve --story ERROR-H-NORETURN D-X` (code comment says D-NEW; replace).
5. Sync, clippy + static, gate (affected --run), plants, precheck, report json, handback-check.
