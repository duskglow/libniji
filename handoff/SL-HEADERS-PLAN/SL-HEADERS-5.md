# SL-HEADERS-5: generator and base layer

Id: SL-HEADERS-5
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-2, SL-HEADERS-3, rulings R-H1..R-H3
Model: opus
Effort: high
BUDGET: 100 tool calls / 200k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 5; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

`cargo xtask headers-gen [--check]` (plan/headers.md §2.4). Templates `headers/<path>.h.in` plus the `spec/abi` tables generate `include/`, which is committed and marked `@generated`. `--check` runs in `cargo xtask static`. The slice also adds the generated `_Static_assert` TUs, the staging in `cargo xtask so` (`target/liella/include` and the kernel UAPI staging), and `conformance/run.sh --liella-headers`, which builds the programs whose `#include`s are covered. The base headers are listed below. `features.h` holds the levels of §2.3. `stdc-predef.h` holds Liella's own values (`__STDC_ISO_10646__` from its Unicode tables, `__STDC_IEC_559__`, ...). `gnu/stubs.h` lists Liella's ENOSYS-only functions. Record the template and marker format and the internal `__liella/` layout in a decision row.

Headers (plan/headers-inventory.tsv, slice 5): `features.h`, `features-time64.h`, `stdc-predef.h`, `sys/cdefs.h`, `sys/types.h`, `sys/bitypes.h`, `stdint.h`, `inttypes.h`, `limits.h`, `endian.h`, `byteswap.h`, `errno.h`, `sys/errno.h`, `alloca.h`, `gnu-versions.h`, `gnu/stubs.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
