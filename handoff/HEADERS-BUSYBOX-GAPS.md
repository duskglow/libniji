# HEADERS-BUSYBOX-GAPS: net/if.h ifreq member names, netpacket/packet.h, sys/mtio.h (static busybox)

**State (leg 1 done, 2026-11-25 ~03:00Z).** Branch `task/headers-busybox-gaps` from origin/next 522900c6f; role
keke:~/w/n4 (the reference system too: Ubuntu GLIBC 2.39, GCC 13.3). Scratch: `~/w/.scratch/HEADERS-BUSYBOX-GAPS/`
(local), `~/work/headers-busybox-gaps/` (keke: probe1.sh, probe2.sh, member_form.py, names.txt, busybox source
tarballs in dl/ = busybox_1.37.0.orig.tar.bz2 + busybox_1.37.0-6.debian.tar.xz from the trixie mirror).
Decisions: none new (everything follows probes; no differences row).

## Done (commits)
* ad0444409 `header_probe.py busybox_names()` (candidates from netdevice(7), packet(7), st(4), UAPI linux/if.h,
  if_packet.h, mtio.h) wired into candidates(); `abi_extract.py` TYPES + sys/mtio.h in HEADERS.
* f1feb557c visibility.tsv +201 rows (header_probe --names), layouts.tsv +59 rows (8 tags: ifconf, ifaddr,
  packet_mreq, mtop, mtget, mtpos, mtconfiginfo, mt_tape_info). net/if_packet.h `sockaddr_pkt` row NOT spliced
  (outside the three headers; INFORMATIONAL).
* 1ac58ed72 macros.tsv +122 (PACKET_*, MT*, MT_ST_*, MTIOC*, DEFTAPE), typedefs.tsv +8.

## Probe-settled
* net/if.h: ifr_*/ifc_* are MACROS wherever struct ifreq is shown (gnu17,gnu2x,default,bsd,svid,gnu,lfs64,fob64,
  time64,dynstack,reentrant,fortify1-3,C++; not strict C/POSIX). Form (member_form.py: `#define <token>
  niji_no_member` after the header, which `sizeof(r->M)` lines fail): ifr_name = ifr_ifrn.ifrn_name;
  ifr_hwaddr/addr/dstaddr/broadaddr/netmask/flags/mtu/map/slave/data/newname = ifr_ifru.ifru_<same>;
  ifr_metric, ifr_ifindex, ifr_bandwidth, ifr_qlen = ifr_ifru.ifru_ivalue; ifc_buf = ifc_ifcu.ifcu_buf,
  ifc_req = ifc_ifcu.ifcu_req. ifr_settings: not in the reference. libniji today: none of them (busybox's failure).
* Not in the reference anywhere: PACKET_USER, PACKET_KERNEL, MTWEOFI, MT_ISONSTREAM_SC, GMT_CLN, MT_ST_TIMEOUTS,
  MT_ST_SET_TIMEOUT, MT_ST_SET_LONG_TIMEOUT, MT_ST_SET_CLN, MT_ST_SYSV, MT_ST_NOWAIT, MT_ST_SILI, MT_ST_NOWAIT_EOF,
  tags tpacket_req/stats/auxdata/hdr.
* sys/mtio.h: include-error in time64only only; netpacket/packet.h names in every configuration (`*`).

## Exact next steps (leg 2)
1. Value probe (black box, run a program against the reference on keke): GMT_EOF..GMT_IM_REP_EN (12 function-like:
   mask by evaluating each on 1UL<<k, k 0..63, and the result type) and MT_TAPE_INFO (`struct mt_tape_info t[] =
   MT_TAPE_INFO;` count, t_type, t_name). Spell them in headers/sys/mtio.h.in (`@@when`/`@@if` like signal.h.in).
2. headers/net/if.h.in: `@@when ifreq` block with the ifr_* macros, `@@when ifconf` with ifc_buf/ifc_req (exact
   spelling above; see headers/signal.h.in si_* for the pattern; `@@owns-macro` if needed).
3. `cargo xtask headers-gen`; check packet.h/mtio.h emit the structs and macros; `cargo xtask abi-gen` if layouts
   feed crates/abi (HEADERS-TABLES-NET did: crates/abi/src/layout.rs, appendix_c_layouts.rs).
4. header_check rows: s12.py (net/if.h): per config, macro form via C++11 stringification (`niji_streq(XS(ifr_name),
   "ifr_ifrn.ifrn_name")`), offsetof/type equal to the nested member; ctx.exact for ifc/ifr names. s13/s15 P1 cover
   packet.h/mtio.h values and sizes automatically (check). GMT_* value test in s15.py.
5. busybox sweep on keke (`make -k` stock vs `-nostdinc -isystem libc-include`), then debs (keke, like
   ~/w/.scratch/HEADERS-SNAPSHOT-GAPS/debs.sh, ~38 min) + ypk.sh on mei (~5 min). Plants file
   handoff/HEADERS-BUSYBOX-GAPS-plants.txt (template mutants: drop an ifr_ macro, ifr_ifindex via ifru_mtu, a GMT
   mask, MTWEOF value via template). header_check, clippy, static, precheck; report json.
