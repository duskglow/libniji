# SL-HEADERS-6: minimal C program

Id: SL-HEADERS-6
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-4, SL-HEADERS-5
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 6; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below, with the Appendix C.9 protocols they carry: `errno`, `assert`, the ctype masks, `MB_CUR_MAX`, `stdin`/`stdout`/`stderr`, and the `__isoc99_*`/`__isoc23_*` call sites of `callsites.tsv`. In C++, the overloads of [cstring.syn].

Headers (plan/headers-inventory.tsv, slice 6): `stdio.h`, `stdlib.h`, `string.h`, `strings.h`, `memory.h`, `unistd.h`, `sys/unistd.h`, `assert.h`, `ctype.h`, `getopt.h`, `time.h`

Legacy headers of this slice (`memory.h`): generate one only when plan/headers.md §5.1 lists it as shipped; leave the others out (their differences rows and the compat record belong to SL-HEADERS-15).

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P6: hello and every conformance program whose includes are covered, with the same verdicts. P10: ctype over 384 inputs in C and C.UTF-8, `assert`, `errno`. C++: `<cstdio>`, `<cstdlib>`, `<cstring>`, `<ctime>`, `<cctype>` and `<cassert>` compile in every -std.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
