# SL-HEADERS-S14: slice 14, users, terminals, IPC, logging (handoff)

**State.** Branch `task/sl-headers-s14` (from origin/next, slice 6 landed). INCOMPLETE: 22 of the slice's 24 headers
are generated; `aio.h` and `mqueue.h` are not. Say "slice 14" in commits and handoffs.

**Done.**
* Templates `headers/<name>.h.in` and committed output `libc-include/` for `pwd.h grp.h shadow.h gshadow.h utmp.h utmpx.h
  lastlog.h sys/fsuid.h termios.h sys/termios.h termio.h sgtty.h pty.h ttyent.h sys/ttychars.h sys/ttydefaults.h syslog.h
  sys/syslog.h sys/ipc.h sys/msg.h sys/sem.h sys/shm.h`. `termio.h` and `sgtty.h` ship (plan/headers.md 5.1: listed "ship").
* Generator (`xtask/src/headers.rs`): unnamed aggregate members and tag-without-rows members are written inline for
  `struct <name>` layouts only (utmp, utmpx; other output unchanged); `gtty`/`stty` are declared from their `typedefs.tsv`
  type; an include covers a name only in the configurations where the included header shows it.
* Tables: `spec/tools/type_probe.py --retry-missing --headers <list>` (valueless macros probed in the other headers that
  show them); 5 rows added to `spec/abi/macros.tsv` (`TTYDEF_*`, probed in `pty.h`).
* `spec/tools/header_check.py`: slice 14 in P5 and P1, kernel UAPI staged as links (`linux`, `asm`, `asm-generic`).
  Result on the final tree: 0 failed of 3101.
* Templates own: `LOG_MASK`/`LOG_UPTO` (syslog(3)), `SHMLBA` (page size through `getpagesize`), the `FD_*` block in
  utmp.h/utmpx.h, `TCSETS2` (taken from the kernel's `<asm/ioctls.h>`, which `termio.h`, `pty.h`, `sgtty.h` include first:
  the kernel spells `TIOCSER_TEMT` as `0x01`, ours `1`, so it must come before ours).
* Plants: `handoff/SL-HEADERS-S14/plants.txt` (7).

**Open.**
* `aio.h`, `mqueue.h`: need `struct sigevent` (nested union, a function-pointer member `fn(...) *` with no parameter list in
  `layouts.tsv`, `__sigval_t`), `struct aiocb64` (no `layouts.tsv` rows: add to `abi_extract.py` TYPES, `--layouts-only`)
  and `struct f_owner_ex` (enum member `enum __pid_type`). Slice 8 generates `sigevent` too: do it once, there or here.
* P2, P3 (probes on libniji's headers), P6 (`cargo xtask so`, `conformance/run.sh --niji-headers`), P7 (coreutils, bash),
  P8, P10 not run. `ut_name`/`ut_time`/`ut_addr` compatibility macros: no table fact.
* Pending decision record for the generator additions not written (`records new decision D-NEW --series D-X --story SL-HEADERS-S14`).
* Caveat: a program that includes `<termios.h>` and then the kernel's `<asm/ioctls.h>` gets a `TIOCSER_TEMT` redefinition
  warning (1 vs 0x01); the reverse order is clean.

**Not to redo.** Generator output for slices 5 and 6 is unchanged by this branch (`headers-gen --check` is clean).
**Next step.** Generate sigevent/aiocb64/f_owner_ex, then aio.h and mqueue.h; then P2, P3, P6, P7.
