# SL-HEADERS-S07: slice 7, POSIX files and processes (handoff after run 1)

**State.** Branch `task/sl-headers-s07` (stacked on slice 6). Head: see `handoff/reports/SL-HEADERS-S07.json`. INCOMPLETE: 24 of
the 29 headers are generated and pass P1 and P5 (0 failed of 3245 units, 40 configurations). Say "slice 7" in commits.

**Done.**
* Templates (`headers/`, from `handoff/SL-HEADERS-S07/mk.py`-style plain `@@emit` plus owned macros): `fcntl.h poll.h dirent.h
  sys/statfs.h sys/statvfs.h sys/times.h utime.h sys/utsname.h sys/mman.h sys/uio.h sys/file.h sys/sendfile.h sys/xattr.h
  sys/resource.h paths.h sys/select.h sys/stat.h sys/time.h sys/sysmacros.h libgen.h` and the aliases `sys/fcntl.h sys/poll.h
  sys/vfs.h sys/dir.h` (R-H7: `sys/dir.h` ships). Macros written from the standards and man pages: `S_IS*`, `S_TYPEIS*`, `st_atime`
  family, `timerisset/clear/cmp/add/sub`, `major/minor/makedev` (call `gnu_dev_*`), `basename` (is `__xpg_basename`), `MAP_FAILED`, `FD_*`.
* Generator (`xtask/src/headers.rs`): anonymous-union members with a reserved alias (`ru_maxrss`), enumeration members, kernel
  `__u8..__s64` members, typedef-of-anonymous-structure members (`f_fsid`), opaque `DIR` (incomplete tag), enumeration
  parameters as `unsigned int` (`prlimit`).
* Tables: `utime` in `prototypes.tsv`, `callsites.tsv`, `noexcept.tsv` (abi_extract.py `--prototypes-only`, `utime.h` in HEADERS;
  callsite_probe.py `--names utime`); `struct statx_timestamp` in `layouts.tsv` (4 rows). Other rows unchanged.
* `getumask` is omitted (`headers/reserved.tsv`): the reference declares it, no library version defines it.
* `header_check.py` covers the 24 headers (P1 values, sizes; P5). Plants: `handoff/SL-HEADERS-S07/plants.txt` (8).

**Not done / blocked.**
* `sys/wait.h`, `wait.h`, `sys/param.h`: the reference's visibility for them includes the whole signal vocabulary (`struct
  sigaction`, `ucontext_t`, `SIG_DFL`, `SIGRTMIN`, `si_status`, `bsd_signal` ...). They need slice 8's `signal.h` (generate them as
  `#include <signal.h>` plus their own macros: `W*` from `wait(2)`, `MAX MIN howmany roundup powerof2 setbit...` from the BSD set).
  `bsd_signal` has a versions row but no prototypes row (a second table gap, like `utime`).
* `spawn.h` needs `sched.h` (`CPU_ALLOC`, slice 8). `sys/ioctl.h` needs `sys/ttydefaults.h` (`TTYDEF_*`, slice 14) and `TCSETS2`.
* P2, P3 (probes on libniji's headers), P6 (`cargo xtask so`, `conformance/run.sh --niji-headers`), P7 (zlib, xz-utils, sqlite3),
  P8 (`sys/xattr.h` with `linux/xattr.h`), P10 (`W*` over all 16-bit statuses, `FD_*` over 0..1023) were NOT run.
* `_IO _IOR _IOW _IOWR` (kernel `linux/ioctl.h`) are not defined by libniji's `sys/ioctl.h` yet; decide with the kernel-coexistence work.
* Pending decision record (generator additions) and the differences row for the omitted `getumask` not written.

**Things not to redo.** The two full table reruns are done; `abi_extract.py` without flags would drop the two `sched_*attr`
prototype rows the committed table has (kept by hand).

**Next step.** Slice 8 first (signal.h), then wait.h/param.h/spawn.h/sys/ioctl.h and the P2/P3/P6/P7/P8/P10 runs here.
Plan rows: plan/headers.md section 3 (P-list), section 4 slice 7.
