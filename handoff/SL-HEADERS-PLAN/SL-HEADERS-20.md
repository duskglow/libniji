# SL-HEADERS-20: switch and package

Id: SL-HEADERS-20
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: all of SL-HEADERS-1..19
Model: opus
Effort: high
BUDGET: 100 tool calls / 200k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 20; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The full C++ check (plan/headers.md P4): every libstdc++ standard header compiles in every -std from c++98 to c++26; `noexcept.tsv` and `typedefs.tsv` are reproduced on Liella's headers. Then `liella-gcc` and `liella-cc` default to Liella's headers, and `liella6-dev` installs them (`/usr/lib/liella/include`; SL-FLAVOUR gets `/usr/include` and the multiarch directory). Add a differences row and a compat flag for any accepted difference.

## Proof

P4; P6 for all 154 conformance programs; P7 for the whole smoke corpus (the same test results and the same undefined-symbol sets as the reference-header build).

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
