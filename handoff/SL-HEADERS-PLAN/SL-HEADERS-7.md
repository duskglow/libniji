# SL-HEADERS-7: POSIX files and processes

Id: SL-HEADERS-7
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 7; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below.

Headers (plan/headers-inventory.tsv, slice 7): `fcntl.h`, `sys/fcntl.h`, `sys/stat.h`, `sys/statvfs.h`, `sys/statfs.h`, `sys/vfs.h`, `dirent.h`, `sys/dir.h`, `sys/wait.h`, `wait.h`, `sys/time.h`, `sys/times.h`, `utime.h`, `sys/resource.h`, `sys/utsname.h`, `sys/select.h`, `poll.h`, `sys/poll.h`, `sys/mman.h`, `sys/uio.h`, `sys/file.h`, `sys/param.h`, `sys/sysmacros.h`, `paths.h`, `libgen.h`, `sys/ioctl.h`, `sys/xattr.h`, `sys/sendfile.h`, `spawn.h`

Legacy headers of this slice (`sys/dir.h`): generate one only when plan/headers.md §5.1 lists it as shipped; leave the others out (their differences rows and the compat record belong to SL-HEADERS-15).

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P6. P7 for zlib, xz-utils and sqlite3 (the same test results and the same undefined-symbol sets as the reference-header build). P8 for `sys/xattr.h` (both orders with `linux/xattr.h`). P10 for the `W*` macros (every 16-bit status) and `FD_*` (0..1023).

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
