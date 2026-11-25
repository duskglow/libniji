# HEADERS-BUSYBOX-GAPS: net/if.h ifreq member names, netpacket/packet.h, sys/mtio.h (static busybox)

**State (leg 2, 2026-11-25).** Branch `task/headers-busybox-gaps` from origin/next 522900c6f; role keke:~/w/n4 (also the
reference system: Ubuntu GLIBC 2.39, GCC 13.3). Scratch: `~/w/.scratch/HEADERS-BUSYBOX-GAPS/` (local),
`~/work/headers-busybox-gaps/` on keke (probes, bbsweep.sh, *.errors). Decisions: none new; everything follows
black-box probes; no differences row.

## Commits
* ad0444409, f1feb557c, 1ac58ed72 (leg 1): header_probe busybox_names(), visibility +201, layouts +59, macros +122,
  typedefs +8.
* 93761ceac headers: net/if.h.in ifr_*/ifc_* member macros (`@@when ifreq` / `@@when ifconf`); net/if_ppp.h.in and
  net/if_shaper.h.in include <net/if.h> (they show every name of net/if.h but include guards); if_ppp.h's own
  `ifr_mtu` (`ifr_ifru.ifru_metric`) where net/if.h does not show it; sys/mtio.h.in GMT_* and MT_TAPE_INFO;
  `__daddr_t` visibility +139 rows and typedefs row (struct mtget's mt_fileno/mt_blkno).
* 83efb5a0e header_check: s12.py `member_names` (form, offset, not-shown units), s15.py `mtio_units`; plants file
  handoff/HEADERS-BUSYBOX-GAPS-plants.txt (6 template mutants).

## Probe-settled (black box, keke)
* ifr_*/ifc_* form: leg 1 (handoff history). ifr_mtu in net/if_ppp.h: net/if.h's `ifr_ifru.ifru_mtu` where struct
  ifreq is shown (gnu17, c++17), `ifr_ifru.ifru_metric` in strict C (c17): poison-after-header probe, ppp_form.sh.
* GMT_*: each `GMT_X(1UL<<k)` nonzero only for its bit and equal to it; type unsigned int for GMT_EOF, int for the
  others (int argument), unsigned long for an unsigned long argument. MT_TAPE_INFO: 17 rows, t_type = the MT_IS*
  values, names as in the template, last row {0, NULL}.
* __daddr_t: int, size 4, align 4, shown by 139 headers (!time64only).

## Results
* header_check.py --include libc-include: 0 failed of 17657 (job headers-busybox-gaps-hgcheck-1).
* clippy -D warnings and `cargo xtask static` on the role: clean (foreground).
* Gate: `cargo xtask affected` escalates to FULL (generated headers and spec/abi tables): FULL is the train's.
* busybox 1.37.0 defconfig + CONFIG_STATIC, `make -k` compile sweep on keke (bbsweep.sh, job
  headers-busybox-gaps-bbsweep-2): stock 15 errors in 1 file; libniji headers 94 in 17 files, 79 libniji-only in 16
  files; none in arp.c, arping.c, mt.c, libbb/xconnect.c. Remaining (outside this story's three headers), first
  error per file: ftpd.c, nbd-client.c TCP_NODELAY (netinet/tcp.h); nslookup.c HEADER (arpa/nameser.h); ping.c,
  traceroute.c icmp6_data8/icmp6_id (netinet/icmp6.h) and icmp_id/icmp_seq/icmp_data (netinet/ip_icmp.h); route.c
  rt_mss (net/route.h); udhcp/d6_dhcpc.c, d6_packet.c ip6_nxt/ip6_hlim (netinet/ip6.h); zcip.c arp_hln.. (netinet/
  if_ether.h); logger.c, syslogd.c, syslogd_and_logger.c CODE/facilitynames/INTERNAL_MARK (syslog.h SYSLOG_NAMES);
  eject.c ALLOW_MEDIUM_REMOVAL, sg_io_hdr_t (scsi/scsi.h, scsi/sg.h); ipcs.c msginfo/shminfo/shm_info, msg_cbytes
  (sys/msg.h, sys/shm.h); setarch.c PER_LINUX32 (sys/personality.h); swaponoff.c MNTOPT_NOAUTO (mntent.h). Most are
  the same member-name-macro pattern as ifr_* (one follow-up story).
* NOT DONE (refused): the static busybox build against the system flavour on mei and the debs/ypk steps (mei access
  refused, see the report). INFORMATIONAL: net/if_packet.h `sockaddr_pkt` not spliced (outside the three headers).

* Plants (headers-busybox-gaps-plants-1): 6 of 6 CAUGHT, no WARN.
* Coordinator add (#8510, ifr_qlen for libiproute/ipaddress.c): ifr_qlen is in the set; ipaddress.c compiles in the sweep
  (make -k runs every directory to the end).

## Next step
Hand-back done. Follow-up story for the sweep's remaining names.
