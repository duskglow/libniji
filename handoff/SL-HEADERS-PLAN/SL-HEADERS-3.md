# SL-HEADERS-3: type and value tables

Id: SL-HEADERS-3
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-2
Model: opus
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 3; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

Extend the probe with three outputs. (1) `spec/abi/typedefs.tsv`: the resolved type and C++ linkage spelling of each typedef and tag (C++ functions that take the type, read back with `nm -C`), the data-object types (`decltype`), and the typedef-resolved prototypes. (2) `spec/abi/macros.tsv`: the C type of each integer constant (`_Generic`), and the value and type of each string and floating macro (bytes of the probe object). (3) The missing `layouts.tsv` rows of plan/headers.md §2.1 (protocol, ELF and debug types, `struct stat64`, `struct flock`, ...), through the existing DWARF path.

## Proof

Sizes agree with `layouts.tsv` and with the OBJECT sizes in `versions.tsv` for every row both have; two runs are identical; plants on the checker.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
