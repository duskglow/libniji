# SL-HEADERS-19: diagnostic attributes

Id: SL-HEADERS-19
Kind: code
Priority: 90
Size: S
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: medium
BUDGET: 60 tool calls / 120k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 19; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

`format`, `nonnull`, `warn_unused_result`, `malloc`/`alloc_size`, `deprecated` and `noreturn` on the landed headers, from the standards (C `_Noreturn`/`[[noreturn]]`, printf/scanf format semantics) and a warning differential against the reference headers.

## Proof

The same warnings (code and line) as the reference headers on a probe corpus compiled with `-Wall -Wextra -Wformat=2 -Wformat-security`. Plants: one attribute removed from each kind.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
