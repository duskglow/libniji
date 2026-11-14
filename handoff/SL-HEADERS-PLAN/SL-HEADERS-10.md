# SL-HEADERS-10: locale, text, matching

Id: SL-HEADERS-10
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 10; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below, with the C++ overloads of [cwchar.syn].

Headers (plan/headers-inventory.tsv, slice 10): `locale.h`, `langinfo.h`, `nl_types.h`, `libintl.h`, `monetary.h`, `wchar.h`, `wctype.h`, `uchar.h`, `iconv.h`, `gconv.h`, `stdbit.h`, `regex.h`, `re_comp.h`, `regexp.h`, `fnmatch.h`, `glob.h`, `wordexp.h`, `search.h`, `fts.h`, `ftw.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P7 for grep, sed, diffutils, findutils and gawk.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
