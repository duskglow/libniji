---
section: changes
---
- **Header names gcc's Go dumper copies for libgo:** `<netinet/in.h>` declares the `IPPORT_*` well-known ports
  (an enumeration, as before only `IPPORT_RESERVED`) and `struct group_req`, `struct group_filter`, `struct ip_msfilter`
  and `struct ip_opts`; `<net/route.h>` defines `RTCF_*`, `RTMSG_*` (`RTMSG_OVERRUN` as `NLMSG_OVERRUN`) and
  `RT_CLASS_*`; `<netinet/ip.h>` `IPTOS_DSCP_VA` and `struct timestamp`; `<netinet/icmp6.h>` and `<netinet/ip_icmp.h>`
  the extended echo names (RFC 8335) and `ND_OPT_RTR_ADV_INTERVAL`; `<link.h>` the `LA_*` audit constants; and further
  names of `<elf.h>`, `<fstab.h>`, `<gshadow.h>`, `<wordexp.h>`, `<scsi/*.h>` and the legacy network headers.
  `<netinet/icmp6.h>` and `<netinet/ip6.h>` include `<inttypes.h>`, and `<sys/auxv.h>` includes `<elf.h>`, as the
  names they show require (the C2X `PRIb*` / `SCNb*` formats reach them through it).
