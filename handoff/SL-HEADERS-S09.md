# SL-HEADERS-S09: slice 9, Linux process and event interfaces (handoff after run 1)

**State.** Branch `task/sl-headers-s09` (cut from origin/next 20e47e117, slice 6 landed). Head: see
`handoff/reports/SL-HEADERS-S09.json`. INCOMPLETE: all 18 headers generate and clippy, static and `records check` are green
(job `sl-headers-s09-lint3`), but `header_check.py` (P1 and P5, 40 configurations) still reports 88 failures of 2789
units (322 before the run's last fixes). P2, P3, P6 and P8 not run. Say "slice 9" in commits and handoffs.

**Done.**
* Templates `headers/sys/{signalfd,timerfd,eventfd,epoll,inotify,fanotify,prctl,personality,pidfd,auxv,random,syscall,
  ptrace,user,reg,procfs,debugreg}.h.in` and `headers/syscall.h.in`; `libc-include/` regenerated (42 headers).
* Generator (`xtask/src/headers.rs`): derived layouts for a typedef member that only has dotted rows (`epoll_data_t`);
  anonymous-union members (`struct user`); packed structures (`struct epoll_event`); array typedefs (`elf_gregset_t`);
  enumeration members as `unsigned int`; `@@enum TAG PREFIX...` (`enum __ptrace_request`); `@@syscalls`; a C++-mangled
  call-site symbol that no library exports is ignored (pidfd_*): libniji declares `extern "C"`.
* `spec/abi/syscalls.tsv` (373 rows) by `spec/tools/syscall_table.py` from the kernel UAPI `asm/unistd_64.h` of the build
  host's linux-libc-dev (6.8; the plan names 6.12.116: regenerate there). `sys/debugreg.h` includes the kernel's
  `asm/debugreg.h`; `sys/syscall.h` includes `asm/unistd.h`.
* `header_check.py`: slice 9 in `HEADERS`/P1, SYS_x number check, `-isystem` for the kernel UAPI (`$NIJI_UAPI`, else the
  host's include dirs).
* Pending record `records/pending/SL-HEADERS-S09-1.toml` (D-NEW, series D-X). Plants `handoff/SL-HEADERS-S09/plants.txt` (5).

**Decisions in force.** The pending record. Not typed into templates: only `si_status` and `si_value` in `sys/pidfd.h.in`
(POSIX member shorthands of siginfo_t, which is slice 8's; they are `#ifndef`-guarded).

**Open.**
1. The 88 failures (sample, config isoc2x): `sigset_t` / `fd_set` / `struct itimerspec` used by the P1 unit for a header
   whose visibility row says visible there but the header does not define it (`sys/epoll.h`, `signalfd.h`, `procfs.h`,
   `timerfd.h`): the name comes through an include of the reference that libniji's template does not make, or a covered
   header's condition excludes the configuration. Group the failures by (header, config) and fix the includes or the
   conditions. The `c89`/`strict` rows with `include-error` are expected to be skipped.
2. P2/P3: `header_probe.py --include libc-include --headers <slice 9>` then `probe_compare.py`; the mangled `pidfd_*`
   call-site rows (and the 44 like them) need an `--accept` entry and a `docs/differences-from-glibc.md` row (neutral
   wording: libniji declares its functions with C linkage in C++). No compat option: no program can bind to a name no
   library exports.
3. Table gaps seen: `sys/reg.h` has only `SS` in `visibility.tsv` (the other register index names are not in
   `constants.tsv`), so the generated `sys/reg.h` is nearly empty; the names are `user_regs_struct` member indexes.
   `sys/debugreg.h` has no table rows at all. Both need a table slice (SL-HEADERS-3 style) or a ruling to derive them.
4. P8 (both include orders with `linux/*.h`: `linux/ptrace.h`, `linux/prctl.h`, `linux/inotify.h`, `linux/fanotify.h`, ...), P6,
   C++ units for the new headers, `struct siginfo_t` members (slice 8).

**Do not redo.** The plan rows of section 2.4 and 3; the generator additions above. Do not regenerate `libc-include/` by hand:
`cargo xtask headers-gen` on the role.

**Next step.** Run `header_check.py --include libc-include --work <ram dir>` on the role, group the 88 by header and config.
