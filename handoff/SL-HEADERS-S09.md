# SL-HEADERS-S09: slice 9, Linux process and event interfaces (handoff after run 2)

**State.** Branch `task/sl-headers-s09-2` (stacks on `task/sl-headers-s09`); head in `handoff/reports/SL-HEADERS-S09-2.json`.
INCOMPLETE: header_check (P1, P5 and the new P8) is green, 0 failed of 2964 (job `sl-headers-s09-2-hc7`); P3 has 8 rows,
all the accepted C-linkage difference; P2 has 19 differences (below); P6 not run; the gate not run (budget). Clippy and
`cargo xtask static` are clean on the head (foreground, before the last commit's docs edit: static re-run after it, clean).
Say "slice 9" in commits and handoffs.

**Done in run 2.**
* Generator: a name an included template supplies counts as covered only in the configurations that template gives it in
  (`is_covered` in `xtask/src/headers.rs`); otherwise the header emits it under its own condition. This fixed the 88 failures.
* `sys/reg.h`, `sys/debugreg.h` (ruling #6399): 53 rows in `macros.tsv`/`constants.tsv`/`visibility.tsv` by
  `spec/tools/uapi_regs_table.py` (register index = member order of `struct user_regs_struct`; DR_ names = kernel UAPI
  `asm/debugreg.h`), each cross-checked by a black-box probe of the reference (exit status only). Six kernel names the
  reference does not give are not rows. `sys/debugreg.h` is generated from the table (no kernel include). PROVENANCE has it.
* Includes follow the visibility rows (stdint.h, sys/types.h, time.h only where the reference shows their names); `sys/reg.h`,
  `sys/debugreg.h`, `sys/user.h` include nothing (not even features.h).
* #6400: `docs/differences-from-glibc.md` row for the C linkage of `pidfd_*` and `thread_db.h` functions in C++ (no compat
  option: no library exports the mangled names); `handoff/SL-HEADERS-S09/p3-accept.txt` is the `--accept` file for
  `probe_compare.py callsites`.
* P8 in `header_check.py` (both orders, C and C++, with and without _GNU_SOURCE, program flags without -Wsystem-headers; a
  unit the reference system does not compile either is a NOTE).
* Plants `handoff/SL-HEADERS-S09/plants.txt` (7).

**Open (next step, in order).**
1. P2: 19 differences left. (a) `sys/syscall.h`, `syscall.h`: `include-error [*]` because `header_probe.py --include` adds no
   kernel UAPI `-isystem` (header_check has `uapi_flags()`); add the same to `header_probe.py` (line ~299) and
   `callsite_probe.py` (~375), `$NIJI_UAPI` or the host's dirs after libniji's tree (an untested edit was reverted for budget).
   (b) `_SIGSET_NWORDS`, `_STRUCT_TIMESPEC`: reference-internal macros that libniji does not define (7 rows): add them to an
   accept file or `headers/reserved.tsv` with a differences row. (c) `be16toh`/`le64toh` in `sys/epoll.h`, `sys/procfs.h`,
   `sys/random.h` are visible only in the table's gnu configurations but libniji's `sys/types.h` (slice 5) includes `endian.h`
   unconditionally: fix in the `sys/types.h` template (condition of the endian names), re-run slice 6's P2 too. (d)
   `sys/pidfd.h` `timespec` and `_STRUCT_TIMESPEC`: the struct is emitted for the POSIX configurations; the table lists it
   only for the non-POSIX ones.
2. P6 (`cargo xtask so`, `conformance/run.sh --niji-headers`), C++ units, the gate on the final head (direct tier, clippy and
   static green).
3. `struct siginfo_t` members of `sys/pidfd.h` (`si_status`, `si_value` shorthands are `#ifndef`-guarded in the template)
   belong to slice 8.

**Not typed into templates:** only `si_status`/`si_value` in `sys/pidfd.h.in`, and the descriptor-set macros in `sys/procfs.h.in`
(a copy of `sys/types.h.in`'s under its own `#ifndef FD_SET` guard until slice 7's `sys/select.h` can be included).

**Do not redo.** The generator additions of run 1 and 2; `cargo xtask headers-gen` on the role regenerates `libc-include/`
(rsync it back to the worktree; never edit it). Run the probes on the role: `header_probe.py --include libc-include --headers
<slice> --out D`, `probe_compare.py visibility --got D --headers <slice>`; `callsite_probe.py ...` and `probe_compare.py
callsites ... --accept handoff/SL-HEADERS-S09/p3-accept.txt`.

**Plan rows.** plan/headers.md section 2.4 and 3; pending record `records/pending/SL-HEADERS-S09-1.toml`.
