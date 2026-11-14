# SL-HEADERS-1: header and reserved-name corpus scan

Id: SL-HEADERS-1
Kind: code
Priority: 90
Size: S
Host: -
Stacks-on: -
Model: sonnet
Effort: medium
BUDGET: 60 tool calls / 120k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 1; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

Scan the program sources of the Essential and build-essential sets and of the smoke corpus (`validation/smoke`), fetched from the project's Devuan mirror with the chain verified as in plan/headers.md §1. Never open a `lib/`, `gnulib/` or `gnulib-tests/` directory: the tool skips them and logs each skip. Count `#include <...>` per header and the uses of `__GLIBC__`, `__GLIBC_MINOR__`, `__GLIBC_PREREQ`, `__GNU_LIBRARY__`, `__USE_*`, the `sys/cdefs.h` vocabulary (`__BEGIN_DECLS`, `__THROW`, `__P`, `__nonnull`, `__wur`, `__attribute_*`), reserved `__*_t` type names, FILE member names from `layouts.tsv`, and direct includes of `bits/` and `gnu/` names. Output: `results/headers-corpus-<date>.md` and a TSV; the tool goes in `validation/` with a README.

## Proof

Two runs give identical output; three counts checked by hand on named files; the skip log shows no excluded directory was read. The result feeds rulings R-H2, R-H3 and R-H4 and the slice order.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate
