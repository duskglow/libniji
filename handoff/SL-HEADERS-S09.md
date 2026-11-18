# SL-HEADERS-S09: slice 9, Linux process and event interfaces (handoff after run 4, a merge of next)

**State.** Branch `task/sl-headers-s09-4` (continues `task/sl-headers-s09-3`); head in `handoff/reports/SL-HEADERS-S09-4.json`.
Run 4 only merged `origin/next` (no other change). Conflicts: `spec/tools/header_check.py` (slice 14 and slice 9 both in the
probed set; next's isolated `kernel-uapi` link directory is used unless `NIJI_UAPI` names the staged tree) and
`xtask/src/headers.rs` (the `covered` computation: both sides had made an included header cover a name only where it shows it;
kept slice 9's `is_covered` with `dont_care`, next's rule for unparsable configurations). One merge error the compiler found: next's
unnamed-aggregate scan and slice 9's anonymous-union state both bound `anon` (renamed to `open_anon`; the `<anon>` union marker rows
are kept out of next's scan). `headers-gen` then rewrote `utmp.h` and `utmpx.h` (their `__sigset_t` now has its members).
Run 4 results (role on the Skylake-SP host): header_check 0 failed of 4563 (all headers, P1/P5/P8); P2 0 unaccepted of 1 difference
over 5024 rows; P3 0 unaccepted of 8 differences over 62 rows; the affected gate (escalated to full by the generator change, so it ran `cargo xtask test` too): 32 suites ok, 12125 passed, 0 failed, conformance 139 passed, 0 failed; plants 10 of 10 caught (jobs in the report json). Next step: land slice 9.
Run 4, final leg: no code change; the same head was re-proved on a new role (Zen 1 host, 32 cores): clippy and `cargo xtask static` clean,
header_check 0 failed of 4563, headers-gen --check ok (67 headers), P2 0 unaccepted of 1 over 5024 rows, P3 0 unaccepted of 8 over 62 rows,
gate (escalated to full) 32 suites ok, 12125 passed, 0 failed, conformance 139 passed, plants 10 of 10 caught (jobs `sl-headers-s09-4-hc`,
`-plants`, `-gate`). Exact next step: land slice 9.
The `D-X.119` record is the slice's decision (the pending record was assigned before this run); two further ids D-X.120 and D-X.121
were reserved by this run for it and are not used.
Say "slice 9" in commits and handoffs.

**Results on the final tree (keke, role n4).**
* header_check P1/P5/P8: 0 failed of 2964 (job `s09-3-hc`).
* P2 (visibility): 0 unaccepted of 1 difference over 5024 rows (job `s09-3-p23b`). The one accepted difference is `struct timespec` in
  `sys/pidfd.h` under `_XOPEN_SOURCE` below 700 (differences row, `handoff/SL-HEADERS-S09/p2-accept.txt`).
* P3 (callsites): 0 unaccepted of 8 differences, all the accepted C-linkage row (`p3-accept.txt`).
* P6 (`cargo xtask so`, `conformance/run.sh --niji-headers`, job `SL-HEADERS-S09-3-p6`): 25 passed, 2 failed (`s02_assert`,
  `s02_chk_fail`), neither includes a slice 9 header: see Open.
* clippy (workspace, all targets, -D warnings) and `cargo xtask static`: clean on the final head.
* Plants (job `SL-HEADERS-S09-3-plants`, 10 plants) and the gate (job `SL-HEADERS-S09-3-gate`): see the report json.

**Done in run 3.**
* `headers/endian.h.in`: the endian(3) macros now sit under their own condition (`@@if be16toh`); before, every header that
  includes `<endian.h>` through `<sys/types.h>` showed them in every configuration (6 of the 19 P2 differences, in
  `sys/epoll.h`, `sys/procfs.h`, `sys/random.h`).
* `headers/sys/syscall.h.in`: no `#include <features.h>` (the table shows no feature-test macros for it; that was 30 `extra` rows
  and the `time64only` include error of `sys/syscall.h`, `syscall.h`).
* `header_probe.py` and `callsite_probe.py`: `uapi_flags()` (the kernel UAPI directories after libniji's tree), as `header_check.py` has.
* `probe_compare.py`: the names `headers/reserved.tsv` marks `omit` (`_SIGSET_NWORDS`, `_STRUCT_TIMESPEC`) are not compared.
* `docs/differences-from-glibc.md`: row for `struct timespec` in `sys/pidfd.h` (XOPEN_SOURCE < 700), no compat option (compile-time
  header text, same layout); `p2-accept.txt`.
* 3 more plants (endian condition, syscall.h include, the probes' UAPI flags).
* Run the probes on the role after `cargo xtask headers-gen`, then copy `libc-include/` back (`ssh ... 'tar -C libc-include -cf - .' |
  tar -C libc-include -xf -`, in its own call: headers-gen prints to stdout) and commit it; a sync overwrites the role's copy with the
  worktree's.

**Open.**
1. P6 failures, outside slice 9: `s02_assert` (the program prints `s02_assert.lh:` and an absolute `__FILE__` path in
   `--niji-headers` mode, the test expects `s02_assert:` and `s02_assert.c:12`: a harness expectation of the `.lh` mode) and
   `s02_chk_fail` (libniji's `string.h` has no `_FORTIFY_SOURCE` redirection yet, so `strcpy` overflows into
   "stack smashing" instead of "buffer overflow detected"; a later slice's work). Slice 6 owns both.
2. The same `struct timespec` row applies to `fcntl.h` and every header that gives `struct stat`; slice 6 should list it too.
3. The all-headers P2 shows differences in `ctype.h` (18), `features-time64.h`, `gnu/stubs.h`, `limits.h`, `stdio.h`, `stdlib.h`,
   `alloca.h`: slices 5 and 6, not touched here.
4. `struct siginfo_t` members of `sys/pidfd.h` (`si_status`, `si_value` shorthands are `#ifndef`-guarded) belong to slice 8.

**Not typed into templates:** only `si_status`/`si_value` in `sys/pidfd.h.in`, and the descriptor-set macros in `sys/procfs.h.in`
(a copy of `sys/types.h.in`'s under its own `#ifndef FD_SET` guard until slice 7's `sys/select.h` can be included).

**Do not redo.** Generator additions of runs 1-2; P8; the 53 `sys/reg.h`/`sys/debugreg.h` rows; the C-linkage row.

**Plan rows.** plan/headers.md section 2.4 and 3; pending record `records/pending/SL-HEADERS-S09-1.toml`.
