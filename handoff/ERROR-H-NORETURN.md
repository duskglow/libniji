# ERROR-H-NORETURN handoff

Branch task/error-h-noreturn (from origin/next e3b0caf60). Role keke:~/w/r3. Decision D-2.13 (records/decisions/D-2/D-2.13.toml).

## State: done (leg 2)
* `libc-include/error.h` regenerated (headers-gen): `error`, `error_at_line` declared without `__noreturn__`; under
  GCC with `__OPTIMIZE__` (not clang, not C++) an always-inline gnu_inline wrapper calls a noreturn `__asm__` alias only
  when `__builtin_constant_p (status) && status != 0`, else a returning alias.
* Table: spec/abi/diagattrs.tsv noreturn `nonzero:1` for both; attr_probe.py measures `nonzero:K` (status-0 and
  constant-1 callers); headers.rs loads it, functions.rs emits the wrapper.
* Test: conformance/c/s02_error_returns.c (NIJI-HEADERS: yes, OPT: -O2): status-0 and run-time-status calls of both
  functions, then checks the code after each ran. Rust test `error_status_zero_returns` in
  crates/core/tests/s02_diagnostics_clauses.rs (clauses s2_3_c_2, s2_3_c_3).
* conformance/run.sh: new per-program directives `NIJI-HEADERS: yes` (libniji's staged headers in the default mode)
  and `OPT: <flags>` (extra flags, part of the relink key; also in dynamic-link builds).
* Plants: handoff/ERROR-H-NORETURN-plants.txt (noreturn back on `error`; error_at_line wrapper always noreturn;
  table `nonzero:1` -> `1` caught by headers-gen --check).

## Caller matrix (gcc 13.3; no clang on any build host, installing needs root: not attempted)
callers: status 0, constant 1, run-time variable, for each function. kept = code after the call survives.
* reference headers: -O0 all kept; -O2 constant-1 dropped, 0 and variable kept; -Wreturn-type on all 6.
* A (no noreturn): all kept at -O0/-O2; -Wreturn-type on all 6.
* B (origin/next): nothing kept, even status 0 (the bug); no -Wreturn-type.
* C (chosen, the wrapper): identical to the reference at -O0 and -O2.
Macro wrapper rejected: `#define error(...)` breaks `obj->error (...)`.

## Audit of other noreturn=1 rows
_Exit, _exit, exit, quick_exit, abort, pthread_exit, thrd_exit, __pthread_unwind_next, longjmp, _longjmp, siglongjmp,
err, errx, verr, verrx: none returns for any argument; they stay `1`. Only error/error_at_line return (status 0).
Not re-run this leg: the attr_probe.py table diff (leg 1's hgen job chained it after a `git diff` that fails in a
non-git role dir, so it never ran).

## Jobs
* error-h-noreturn-gate2 (direct gate, head 102859c95): 709 passed, 1 failed (headers::slices::s19
  diagnostic_attributes_match_the_reference: the probe's new nz/nv callers passed 0 to qsort's typedef'd comparator,
  -Wnonnull in the reference only); conformance 148 passed. Fixed in b2286ec01 (nz/nv judged by their marker only).
* error-h-noreturn-s19 (head b2286ec01): the failing suite (headers::slices::s19) ok, `cargo xtask static` and
  workspace clippy -D warnings clean.
* error-h-noreturn-plants2: 3 plants, 3 CAUGHT, 0 WARN (header noreturn back; error_at_line wrapper always noreturn;
  table nonzero:1 -> 1 caught by headers-gen --check).
* FINAL (content of d57798fdf): error-h-noreturn-gate3 direct gate 716 passed, 0 failed, conformance 148 passed,
  peak 1.8G; error-h-noreturn-plants3 3/3 CAUGHT, 0 WARN; precheck GREEN (34 checks).
* Superseded: error-h-noreturn-gate2, error-h-noreturn-s19, error-h-noreturn-plants2 (older content), error-h-noreturn-gate (trace red: clause-prefixed test name), error-h-noreturn-plants (killed, older
  head), error-h-noreturn-hgen (header regeneration only).

## Next step
Done (report handoff/reports/ERROR-H-NORETURN.json). Optional follow-up: rerun
`python3 spec/tools/attr_probe.py table` and diff against spec/abi/diagattrs.tsv on a build host; clang matrix
when a host has clang.
