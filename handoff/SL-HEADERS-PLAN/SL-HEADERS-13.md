# SL-HEADERS-13: network protocols and families

Id: SL-HEADERS-13
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-12
Model: sonnet
Effort: medium
BUDGET: 80 tool calls / 160k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 13; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. Their structs come from the new `layouts.tsv` rows (SL-HEADERS-3).

Headers (plan/headers-inventory.tsv, slice 13): `net/ethernet.h`, `net/if_arp.h`, `net/if_packet.h`, `net/if_ppp.h`, `net/if_shaper.h`, `net/if_slip.h`, `net/ppp-comp.h`, `net/ppp_defs.h`, `net/route.h`, `netinet/ether.h`, `netinet/icmp6.h`, `netinet/if_ether.h`, `netinet/if_fddi.h`, `netinet/if_tr.h`, `netinet/igmp.h`, `netinet/in_systm.h`, `netinet/ip.h`, `netinet/ip6.h`, `netinet/ip_icmp.h`, `netpacket/packet.h`, `netash/ash.h`, `netatalk/at.h`, `netax25/ax25.h`, `neteconet/ec.h`, `netipx/ipx.h`, `netiucv/iucv.h`, `netrom/netrom.h`, `netrose/rose.h`, `arpa/ftp.h`, `arpa/telnet.h`, `arpa/tftp.h`, `protocols/routed.h`, `protocols/rwhod.h`, `protocols/talkd.h`, `protocols/timed.h`

Legacy headers of this slice (`netash/ash.h`, `netatalk/at.h`, `netax25/ax25.h`, `neteconet/ec.h`, `netipx/ipx.h`, `netiucv/iucv.h`, `netrom/netrom.h`, `netrose/rose.h`): generate one only when plan/headers.md §5.1 lists it as shipped; leave the others out (their differences rows and the compat record belong to SL-HEADERS-15).

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P8 against the matching `linux/*.h` headers.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
