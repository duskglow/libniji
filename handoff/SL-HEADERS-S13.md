# SL-HEADERS-S13: slice 13, network protocols and families (run 1)

**State.** Branch `task/sl-headers-s13` from origin/next 302dd7c3a. Say "slice 13" in commits and handoffs. Report:
`handoff/reports/SL-HEADERS-S13.json`. Plants: `handoff/SL-HEADERS-S13-plants.txt`. Data only (slice contract): 35 templates
`headers/<path>.h.in`, `headers/slices/s13.tsv`, `spec/tools/header_checks/s13.py`, the regenerated `libc-include/` headers. No generator,
`header_check.py` or table change; no layouts row changed, so no abi-gen / stubs regeneration was needed (abi-gen wrote no diff).

**Headers shipped (35).** All of the brief's list, the legacy eight included: plan/headers.md section 5.1 lists every one of them
as "ship" (`netash/ash.h`, `netatalk/at.h`, `netax25/ax25.h`, `neteconet/ec.h`, `netipx/ipx.h`, `netiucv/iucv.h`, `netrom/netrom.h`,
`netrose/rose.h`).

**Proof.** `header_check.py` on the final tree: 0 failed of 16200 (P5 each header alone and twice, P1 for 33 headers by the tables,
P1 of the other two by `header_checks/s13.py`, the slice-table check, P8 both include orders for 17 pairs). `cargo xtask static` green
(`headers-gen --check` included). Plants (5): see the report json. The direct tier selects no test for a generated-header change
(`affected --list`: 0 of 315), so the gate job is the empty direct tier and the train's FULL gate is the real one
(`FULL: the train's (generated headers only)`).

**What the headers hold: only what spec/abi holds.** Names visible in a header (visibility.tsv) are mostly macros of the
kernel UAPI and of included headers. The tables carry the structs and typedefs of these headers, but NOT their own macros: `IPTOS_*`,
`IPOPT_*`, `ICMP_*` / `ICMP6_*` / `ND_*`, `RTF_*`, `TELOPT_*`, `TELCMD_*`, the `arpa/ftp.h` and `arpa/tftp.h`
constants, `protocols/*` constants and `struct rtentry`, `struct arpreq` and `struct ether_arp` are in no spec/abi table (grep of
`spec/abi` for `IPTOS_LOWDELAY`, `RTF_UP`, `ICMP_ECHO`, `TELOPT_ECHO`, `ICMP6_ECHO_REQUEST`, `rtentry`, `arpreq`, `ether_arp`: no hit; `ARPHRD_*`, `icmphdr`, `icmp6_hdr` are there). `arpa/ftp.h`, `arpa/telnet.h`, `net/if_slip.h`,
`net/ppp-comp.h` have no row in any table: they are generated as the empty guard-only header (include-compatible, no content).
This is the "cannot be expressed in today's tables" case of the brief: RULING NEEDED below, owner HEADERS-TABLES-COMPLETE.

**Two headers wait for sys/ioctl.h (slice 7 tail).** `net/if_ppp.h` and `net/if_shaper.h` `@@needs sys/ioctl.h` (`__has_include`
guarded until it has a template); their table P1 is `p1` 0 with `header_checks/s13.py` running P1 without the names
visibility.tsv lists for `sys/ioctl.h` (`struct winsize`, `CBRK`, `TCSETS2`, ...). Next step when `sys/ioctl.h` lands: delete `s13.py`, set
`p1` 1 on both rows, merge next, `cargo xtask headers-gen`.

## Kernel-header-first measurement (coordinator note, ruling #6458)

Method (a one-off script, not committed): for each pair of the slices' `uapi` cells (all slices on this tree, 2 languages: C gnu17,
C++ gnu++17, flags `-Wall -Wextra -Werror`, `-fsyntax-only`), include the pair in each order against the system reference
headers and against libc-include with the staged UAPI. Pairs not listed compile in all four cells.

| libniji header | kernel header | libc first: ref / libniji | kernel first: ref / libniji |
|---|---|---|---|
| netinet/in.h | linux/in.h | ok / ok | FAIL / FAIL |
| net/if.h | linux/if.h | ok / ok | FAIL / FAIL |
| netinet/ip6.h | linux/ipv6.h | ok / ok | **ok / FAIL** |
| netinet/ip.h | linux/ip.h | FAIL / FAIL | FAIL / FAIL |
| netinet/ip_icmp.h | linux/icmp.h | FAIL / FAIL | FAIL / FAIL |
| netpacket/packet.h | linux/if_packet.h | FAIL / FAIL | FAIL / FAIL |
| net/if_arp.h | linux/if_arp.h | FAIL / FAIL | FAIL / FAIL |
| net/if_packet.h, net/if_ppp.h, net/route.h, netax25/ax25.h, netinet/icmp6.h, netrom/netrom.h, netrose/rose.h | their linux/ header | FAIL / ok | FAIL / ok |
| sys/timerfd.h, sys/eventfd.h (slice 8) | linux/ header | ok / ok | FAIL / ok |
| sys/ptrace.h | linux/ptrace.h, asm/ptrace.h | ok / ok | FAIL / FAIL |

Reading: of the slice 13 pairs the reference compiles `linux/ipv6.h` then `netinet/ip6.h` and libniji does not (`struct in6_addr`
defined by the kernel header, then again by libniji's netinet/in.h, which sets the kernel's `_NETINET_IN_H` compat marker only at its
end). Everything else the reference fails kernel-first, libniji passes or fails equally. A "FAIL / ok" row means libniji accepts a
pair the reference's own headers reject (the reference side may be rejecting a warning under `-Werror`; not diagnosed).

Recommendation: one opt-in extension is worth considering and only for the in6/in pair family: make `netinet/in.h` (and so ip6.h,
icmp6.h) define the kernel's compat marker before its struct definitions and skip a struct the kernel header already defined.
Cost: a few guard lines in one template; gain: `linux/in6.h`, `linux/ipv6.h`, `linux/in.h` first orders work, which the reference
supports only for ipv6.h. Default stays the reference's behaviour (libc first); not implemented here (brief). `netinet/ip6.h` is
left without a `uapi` cell in `s13.tsv` meanwhile, because P8 would be red on that one pair (P8 libc-first of it passes).

## RULING NEEDED

* Tables hold none of the own macros of these headers (above): name HEADERS-TABLES-COMPLETE (or an extraction story for the slice 13 header
  list in `spec/tools/abi_extract.py`, whose HEADERS list already names most of them); recommend a macros/constants/layout extraction
  for slice 13's headers (`struct rtentry`, `struct arpreq`, `struct ether_arp`, `IPTOS_*`, ...) followed by a re-run
  of this slice's `headers-gen`; the templates need no change.
* Kernel-first for `netinet/ip6.h` + `linux/ipv6.h` (above): opt-in extension, recommend yes, small.

## Not to redo
Include lists of the templates were found by running `headers-gen` (a name visible without a template covering it is a problem) and
fixing each; the 35 templates are the minimal set that satisfies visibility.tsv. Legacy decisions come from plan/headers.md 5.1.
