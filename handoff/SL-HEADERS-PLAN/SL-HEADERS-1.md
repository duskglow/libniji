# SL-HEADERS-1: header and reserved-name corpus scan

Id: SL-HEADERS-1
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: -
Model: sonnet
Effort: medium
BUDGET: 80 tool calls / 160k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 1; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

Scan the program sources of the Essential and build-essential sets and of the smoke corpus (`validation/smoke`), fetched from the project's Devuan mirror with the chain verified as in plan/headers.md §1. Never open a `lib/`, `gnulib/` or `gnulib-tests/` directory: the tool skips them and logs each skip. Count `#include <...>` per header and the uses of `__GLIBC__`, `__GLIBC_MINOR__`, `__GLIBC_PREREQ`, `__GNU_LIBRARY__`, `__USE_*`, the `sys/cdefs.h` vocabulary (`__BEGIN_DECLS`, `__THROW`, `__P`, `__nonnull`, `__wur`, `__attribute_*`), reserved `__*_t` type names, FILE member names from `layouts.tsv`, and direct includes of `bits/` and `gnu/` names. Output: `results/headers-corpus-<date>.md` and a TSV; the tool goes in `validation/` with a README.

(b) The R-H7 scan (owner ruling 2026-11-14, plan/headers.md §5.1): for each legacy header that §5.1 still lists as undecided, which source packages of the whole mirror (`main/source/Sources.xz` of excalibur, 37,587 packages, 98.1 GB of source files, chain verified) contain an `#include` of it. Stream one package at a time in RAM (`/dev/shm/claude/<story>/`), keep only `package<TAB>header` names, delete each tree after its grep. Exclude the C library source packages by name (glibc, musl, dietlibc, klibc, newlib, picolibc, uclibc and the like) and every `lib/`, `gnulib/` and `gnulib-tests/` directory, logging each exclusion. Run it as one long detached job. Then update §5.1: a header with a hit is shipped; one with none is dropped (SL-HEADERS-15 lands the drops).

## Proof

Two runs give identical output; three counts checked by hand on named files; the skip log shows no excluded directory was read. (b): every package of `Sources` is accounted for (scanned, excluded with its reason, or a download that failed, retried until none is left). The result decides which glibc-internal names R-H2 and R-H3 allow (plan/headers.md §5) and the slice order.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate
