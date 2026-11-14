# SL-HEADERS-16: ELF, dynamic linking, debugging

Id: SL-HEADERS-16
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 16; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. `elf.h` takes its types from the System V gABI and the x86-64 psABI (new `layouts.tsv` rows) and its values from `constants.tsv`. `gnu/lib-names.h` takes its soname strings from `versions.tsv`.

Headers (plan/headers-inventory.tsv, slice 16): `dlfcn.h`, `link.h`, `elf.h`, `sys/elf.h`, `a.out.h`, `ar.h`, `stab.h`, `sys/platform/x86.h`, `thread_db.h`, `proc_service.h`, `sys/gmon.h`, `sys/gmon_out.h`, `sys/profil.h`, `gnu/lib-names.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P6 (the dlfcn, link and elf programs).

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
