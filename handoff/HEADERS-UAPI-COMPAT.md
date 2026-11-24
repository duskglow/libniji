# HEADERS-UAPI-COMPAT: libniji's headers and the kernel's UAPI headers in either include order

**State:** branch task/headers-uapi-compat (cut from origin/next), head: see handoff/reports/HEADERS-UAPI-COMPAT.json.
Code commit 5f129a66d. Build role: shiki sh1 (Haswell-EP host); jobs named in the report json.

## What changed
* `headers/netinet/in.h.in`: before its definitions, the kernel's protocol (`linux/libc-compat.h`, kernel 6.8): when
  `linux/in6.h` was included first (`_LINUX_IN6_H`) and its `__UAPI_DEF_*` markers are 1, libniji's guards
  `__NIJI_D_struct_{in6_addr,sockaddr_in6,ipv6_mreq,in6_pktinfo,ip6_mtuinfo}` are preset, so the kernel's definitions
  stand, and libniji's `s6_addr`/`s6_addr16`/`s6_addr32` are not defined over the kernel's. libniji-first is unchanged
  (`_NETINET_IN_H` makes the kernel skip). Macros were already `#ifndef`-guarded, enum constants too.
* `headers/netinet/igmp.h.in`, `ip_icmp.h.in`: `@@needs-if` instead of unconditional includes of `netinet/in.h` (and
  `ip.h`): visibility.tsv shows those names there only in the MISC configurations; in strict C17 the reference accepts
  `linux/in.h` before them and libniji did not.
* P8 (`spec/tools/header_check.py`): new `uapi` cells (s12 netinet/in.h: + linux/in6.h, linux/ipv6.h; s13 netinet/ip6.h:
  linux/ipv6.h (S13 left it empty); igmp.h, ip_icmp.h: + linux/in.h; s07 sys/xattr.h: linux/xattr.h) and a gnu++17 mode.
  `spec/tools/header_checks/uapi.py`: 4 units x 4 configs (c17, gnu17, gnu, cxx17) that use what each order defines.
* Plants: `handoff/HEADERS-UAPI-COMPAT-plants.txt` (10).

## Overlap table (kernel 6.8 uapi; marker -> definitions -> libniji header)
| markers | kernel header: definitions | libniji | libc first | kernel first |
|---|---|---|---|---|
| IF_IFNAMSIZ, IF_NET_DEVICE_FLAGS(+_LOWER_UP_DORMANT_ECHO), IF_IFMAP, IF_IFREQ, IF_IFCONF | linux/if.h: IFNAMSIZ, enum net_device_flags + IFF_* macros, struct ifmap, ifreq (IFHWADDRLEN), ifconf | net/if.h (`_NET_IF_H`) | kernel skips | reference fails too |
| IN_IPPROTO, IN_ADDR, IP_MREQ, IN_PKTINFO, SOCKADDR_IN, IN_CLASS | linux/in.h: IPPROTO_* enum, struct in_addr, ip_mreq, ip_mreqn, ip_mreq_source, ip_msfilter, group_req, group_source_req, group_filter, in_pktinfo, sockaddr_in, IN_CLASS*/INADDR_* | netinet/in.h (`_NETINET_IN_H`) | kernel skips | reference fails too (except strict C17 igmp.h/ip_icmp.h: fixed) |
| IN6_ADDR, IN6_ADDR_ALT, SOCKADDR_IN6, IPV6_MREQ, IPPROTO_V6, IPV6_OPTIONS | linux/in6.h: struct in6_addr, s6_addr16/32, sockaddr_in6, ipv6_mreq, IPPROTO_HOPOPTS..MH, IPV6_ADD/DROP_MEMBERSHIP | netinet/in.h | kernel skips | **new: libniji skips** |
| IN6_PKTINFO, IP6_MTUINFO | linux/ipv6.h: struct in6_pktinfo, ip6_mtuinfo | netinet/in.h (GNU) | kernel skips | **new: libniji skips** (also when only linux/in6.h came first: the markers leave them to linux/ipv6.h) |
| XATTR | linux/xattr.h: XATTR_CREATE, XATTR_REPLACE | sys/xattr.h (`_SYS_XATTR_H`) | ok | ok |
| IPX_* | none in 6.8 (linux/ipx.h removed) | netipx/ipx.h | - | - |
| ETHHDR (if_ether.h's own) | linux/if_ether.h: struct ethhdr | not defined by libniji (nor the reference) | no overlap | no overlap |

## Both-orders probe (compile only; reference rc only, its diagnostics discarded unread)
Every libniji header (223, bits/ excluded) x 8 kernel headers (libc-compat.h, in.h, in6.h, ipv6.h, if.h, xattr.h,
if_ether.h, netfilter_bridge.h) x 2 orders x 5 modes (c17, c17+_GNU_SOURCE, gnu17, c++17, gnu++17) = 18,480 units.
* Before: 104 units reference ok / libniji FAIL: kernel-first linux/in6.h or linux/ipv6.h before arpa/inet.h,
  net/route.h, netdb.h, netinet/{icmp6,igmp,in,ip,ip6,ip_icmp}.h, resolv.h (5 modes each: `redefinition of struct
  in6_addr`), plus strict C17 linux/in.h (and linux/netfilter_bridge.h) before netinet/igmp.h, netinet/ip_icmp.h.
* After: **0**. Reference FAIL / libniji ok: 80, all `sys/vm86.h` (the reference rejects that header on its own; unrelated).
* Semantics after the pair (probe of 15 uses x 6 orders x 3 modes): identical to the reference (kernel's in6_u and
  ipv6mr_ifindex kernel-first, s6_addr16/32 in strict C kernel-first, no in6_pktinfo after linux/in6.h alone, libniji's
  after linux/if.h) except one: libniji's `IN6_IS_ADDR_*` work on the kernel's struct, the reference's do not (more
  permissive; no differences row, nothing a built program sees).

## Decisions / open
* No differences row: libniji compiles every pair the reference compiles. No new public name (only `__NIJI_*`).
* RULING NEEDED: none. INFORMATIONAL: kernel-first linux/in.h and linux/if.h stay failing as with the reference
  (S13's opt-in extension idea; not done: the brief asks for parity).
* Not to redo: the probes (scripts were scratch: probe.py, probe2.py; method above).

## Next step
Land. Nothing else owed by this story.
