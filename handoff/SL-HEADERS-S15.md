# SL-HEADERS-S15: slice 15, administration, devices, legacy (handoff; continued by SL-HEADERS-S15-3)

**State.** Branch `task/sl-headers-s15-3` (continues `task/sl-headers-s15-2`), merged with `origin/next` (generator split into
`xtask/src/headers/*.rs`; the slice's generator additions were reapplied there). Head: `handoff/reports/SL-HEADERS-S15-3.json`.
Say "slice 15" in commits and handoffs.

**Done in S15-3.**
* `D-X.126` record: already fixed in S15-2 (no `pattern` column); `cargo xtask static` passes on this head.
* Merge of `origin/next`: conflicts in `xtask/src/headers.rs` (next moved the code into modules): next's file taken, the placeholder enum
  tag (`headers/aggregates.rs`) and the enumerator-yields-to-macro guard (`headers.rs` `item`) reapplied; float `inf`/`nan` already on next.
  `docs/differences-from-glibc.md`, `headers/reserved.tsv`: both sides kept (the slice 9 `pidfd.h` timespec row is gone on next).
* `sys/timex.h`: plain `#include <sys/select.h>`, `@@owns` of `select`/`pselect`/`fd_set`/`FD_*` dropped.
* `sys/mount.h`: plain `#include <fcntl.h>` (`f_owner_ex`, and `struct timespec` in the xopen configurations: the one unexplained P2
  difference was `<fcntl.h>`'s, found by including it; no header text compared). `s15.py` `DEFERRED` keeps only the three mount names
  that need `sys/ioctl.h`.
* P2 (probe + compare, 27 headers, `NIJI_UAPI` staged): 69 -> 60 differences: 27 `MS_*` enumerators (COMPAT-34), 14 `sys/vm86.h`,
  1 `mntent.h _IO_FILE` (all three accepted by differences rows; `handoff/SL-HEADERS-S15/p2-accept.txt`, 42 lines) and 18 that wait
  for `sys/ioctl.h` (`TCSETS2`, `TTYDEF_*` of `sys/mount.h`, `sys/mtio.h`, `sys/raw.h`).

**Still held (not slice 15's to fix).** `sys/ioctl.h` has no template on `origin/next` (slice 7 tail: "blocked on slice 14"; the S07-TAIL
brief exists, not landed). Until it lands the three templates keep an optional `__has_include(<sys/ioctl.h>)` and `@@owns TCSETS2
TTYDEF_*`. P8 with a host `/usr/include` on the path still needs `NIJI_UAPI` limited to `linux`, `asm-generic`, `asm`.

**Next step.** After `sys/ioctl.h` lands: merge `origin/next`; in `sys/mount.h.in`, `sys/mtio.h.in`, `sys/raw.h.in` replace the
`__has_include` block by `#include <sys/ioctl.h>` and drop `@@owns TCSETS2 TTYDEF_*` (and `mount_attr`/`f_owner_ex` only if listed);
empty `DEFERRED` in `header_checks/s15.py` (delete the file if `MS_NOUSER` then passes P1; set `p1` 1 in `headers/slices/s15.tsv`);
`cargo xtask headers-gen`; rerun P2 (expect only the three accepted classes, 42 rows).

**Not to redo.** Registry move, MS_* ruling, P8 reverse order, the sys/types.h trimming, the select.h / fcntl.h switch. Plants:
`handoff/SL-HEADERS-S15/plants.txt` (paths follow the generator's modules).
