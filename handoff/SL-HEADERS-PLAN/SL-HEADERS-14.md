# SL-HEADERS-14: users, terminals, IPC, logging

Id: SL-HEADERS-14
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 14; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below.

Headers (plan/headers-inventory.tsv, slice 14): `pwd.h`, `grp.h`, `shadow.h`, `gshadow.h`, `utmp.h`, `utmpx.h`, `lastlog.h`, `sys/fsuid.h`, `termios.h`, `sys/termios.h`, `termio.h`, `sgtty.h`, `pty.h`, `ttyent.h`, `sys/ttychars.h`, `sys/ttydefaults.h`, `syslog.h`, `sys/syslog.h`, `mqueue.h`, `sys/ipc.h`, `sys/msg.h`, `sys/sem.h`, `sys/shm.h`, `aio.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P6. P7 for coreutils and bash.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
