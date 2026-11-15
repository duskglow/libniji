# SL-HEADERS-9: Linux process and event interfaces

Id: SL-HEADERS-9
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 9; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. `SYS_*` come from the kernel UAPI's `__NR_*` (linux-libc-dev), not from a list.

Headers (plan/headers-inventory.tsv, slice 9): `sys/signalfd.h`, `sys/timerfd.h`, `sys/eventfd.h`, `sys/epoll.h`, `sys/inotify.h`, `sys/fanotify.h`, `sys/prctl.h`, `sys/personality.h`, `sys/pidfd.h`, `sys/auxv.h`, `sys/random.h`, `sys/syscall.h`, `syscall.h`, `sys/ptrace.h`, `sys/user.h`, `sys/reg.h`, `sys/procfs.h`, `sys/debugreg.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P6. P8: each header in both include orders with its `linux/*.h` counterpart.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
