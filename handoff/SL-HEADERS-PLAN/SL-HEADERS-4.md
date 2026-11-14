# SL-HEADERS-4: call-site and noexcept tables

Id: SL-HEADERS-4
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-2
Model: opus
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 4; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

`spec/abi/callsites.tsv`: for each function and configuration, the symbol that `&f` references at -O0, and the symbol that a call with a known-size buffer references at -O2 with `_FORTIFY_SOURCE` 1, 2 and 3 (`nm -u` on the probe object). `spec/abi/noexcept.tsv`: the C++17 overload probe on the function type (`R(*)(A...) noexcept` or not, variadic forms included), and `throw()` at -std=c++98.

## Proof

Each of the 44 `__isoc*` and 83 `*_chk` exports is reached by some (function, configuration) or listed with a reason; every `prototypes.tsv` function has a noexcept row; two runs are identical; plants on the checker.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
