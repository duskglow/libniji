# SL-HEADERS-12: network core, name services

Id: SL-HEADERS-12
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 12; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. Kernel UAPI coexistence follows `linux/libc-compat.h` per rulings R-H1 and R-H2 (plan/headers.md §2.4).

Headers (plan/headers-inventory.tsv, slice 12): `sys/socket.h`, `sys/un.h`, `netinet/in.h`, `arpa/inet.h`, `netdb.h`, `net/if.h`, `ifaddrs.h`, `netinet/tcp.h`, `netinet/udp.h`, `resolv.h`, `arpa/nameser.h`, `arpa/nameser_compat.h`, `nss.h`, `aliases.h`, `rpc/netdb.h`, `sys/socketvar.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P8: `netinet/in.h` and `net/if.h` in both orders with `linux/in.h`, `linux/in6.h` and `linux/if.h`, with and without `_GNU_SOURCE`. P10 for `CMSG_*` and `IN6_IS_ADDR_*`.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
