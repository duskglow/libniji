# SL-HEADERS-S15: slice 15, administration, devices, legacy (handoff; continued by SL-HEADERS-S15-2)

**State.** Branch `task/sl-headers-s15-2` (continues `task/sl-headers-s15`), merged with `origin/next` (slice registry). Head: see
`handoff/reports/SL-HEADERS-S15-2.json`. Say "slice 15" in commits and handoffs.

**Held (coordinator ruling, 11-17, #6434).** Slice 7 (`fcntl.h`, `sys/select.h`, `sys/ioctl.h`) has not landed on `origin/next`
(no template for any of the three). The names slice 15 borrows (`fd_set`, `select`, `pselect`, `FD_*` in `sys/timex.h`; `f_owner_ex`,
`TCSETS2`, `TTYDEF_*` in `sys/mount.h`, `sys/mtio.h`, `sys/raw.h`) are therefore still not defined, and the optional
`__has_include` includes are still in the templates. 26 P2 rows are exactly these.

**Done in S15-2.**
* Registry move: `headers/slices/s15.tsv` (27 rows; `sys/mount.h` pairs with `linux/mount.h` for P8; `sys/mount.h` and `sys/timex.h`
  are `p1` 0), `spec/tools/header_checks/s15.py` (their P1 without the deferred names). Next's `header_check.py`, probes and `headers.rs`
  were taken; the slice's own staging of the kernel UAPI is gone (next's `uapi_flags`/`NIJI_UAPI` does it); `@@partial-cover` is gone
  (next's configuration-aware `is_covered` does it).
* Real generator change (own commit, `xtask/src/headers.rs`): `float`/`double`/`long double` `inf`/`nan` literals; an enumeration
  the tables record by name and size gets a placeholder enumerator; an enumerator yields to a macro of the same name (`#ifndef` guard,
  which adds guard lines to eleven committed headers).
* #6435: `MS_*` of `sys/mount.h` are macros: D-X.126, COMPAT-34 (registered), differences row.
* `sys/soundcard.h`, `sys/timex.h`, `sys/mount.h` no longer include `sys/types.h` (`sys/timex.h` not `time.h`; `sys/soundcard.h`
  includes `endian.h`): P2 differences 352 -> 69 (`header_probe` on the 27 headers, `NIJI_UAPI` staged).
* P8 reverse order of `sys/mount.h` with `linux/mount.h`: run, 4 modes x 2 orders, passes. `header_check.py`: 0 failed of 6170.
  NOTE: run it with `NIJI_UAPI` set to a directory holding only `linux`, `asm-generic`, `asm` links; without it P8 puts the host's
  own `/usr/include` on the path, `__has_include(<sys/ioctl.h>)` finds the host's header and `struct winsize` is defined twice
  (goes away when the include is plain and slice 7 is in).

**P2 classes of the 69 (`handoff/SL-HEADERS-S15/p2-accept.txt` holds the accepted 42).**
* 27 `MS_*` enumerator rows missing (the macro rows match): accepted, COMPAT-34.
* 14 `sys/vm86.h` (the reference header cannot be included on x86-64; one `differs include-error` and 13 feature-test macros): accepted
  differences row, no compat option (a compile error cannot be depended on).
* 1 `extra mntent.h _IO_FILE`: the structure is complete after `<mntent.h>`: accepted differences row.
* 26 waiting for slice 7 (`select`, `pselect`, `fd_set`, `FD_*` of `sys/timex.h` 7; `f_owner_ex` + `TCSETS2` + `TTYDEF_*` of
  `sys/mount.h` 7, `sys/mtio.h` 6, `sys/raw.h` 6).
* 1 UNEXPLAINED: `differs sys/mount.h timespec struct`: the reference has `struct timespec` in the four `xopen*` configurations
  (`_XOPEN_SOURCE` 400..600, `_XOPEN_SOURCE_EXTENDED`) and libniji's `sys/mount.h` does not (`time_t` matches). The generator's condition
  for the row misses them; not chased.

**Next step.** After slice 7 lands: merge `origin/next`; replace the three `__has_include` blocks by plain `#include <fcntl.h>`,
`<sys/select.h>`, `<sys/ioctl.h>` and drop `@@owns` of the borrowed names; delete `DEFERRED` of `header_checks/s15.py`, set `p1` 1 for
`sys/mount.h`, `sys/timex.h` (and delete `s15.py` if `MS_NOUSER` then passes P1); regenerate; rerun P2 (expect the 26 gone) and look at
the `timespec` row; regenerate `p2-accept.txt` only for the three accepted classes.

**Not to redo.** Registry move, MS_* ruling, P8 reverse order, the sys/types.h trimming. Plants: `handoff/SL-HEADERS-S15/plants.txt`.
