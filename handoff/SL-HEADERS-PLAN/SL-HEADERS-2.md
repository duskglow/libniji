# SL-HEADERS-2: visibility tables

Id: SL-HEADERS-2
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: ruling R-H0
Model: opus
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 2; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

Add `spec/tools/header_probe.py` (run on the reference system, like `abi_extract.py`). It writes `spec/abi/ftm-configs.tsv` (the configurations of plan/headers.md §2.3) and `spec/abi/visibility.tsv`: for each public header of `plan/headers-inventory.tsv` and each configuration, the visible names and their kinds (function, object, typedef, complete tag, `#ifdef` macro, enumerator). Read only diagnostics that name the probe's own identifiers and the symbols and data of the probe's own objects (§2.2). Never read `-E` or `-dM` output. Add Appendix C section C.12 and the PROVENANCE line.

## Proof

Every `prototypes.tsv` and `constants.tsv` name is visible in some (header, configuration); two runs are byte-identical; an xtask check validates the table shape, with plants on it.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
