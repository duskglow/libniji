# HEADERS-SNAPSHOT-GAPS report

Branch `task/headers-snapshot-gaps`, code head fc5f5b84a (commits listed in handoff/HEADERS-SNAPSHOT-GAPS.md).

## Probe-settled behaviour (reference: Ubuntu GLIBC 2.39-0ubuntu8.9, GCC 13.3, Zen 1 host)
* `<sys/types.h>`: u_char, u_short, u_int, u_long, ushort, uint, ulong, quad_t, u_quad_t, fsid_t, loff_t, daddr_t,
  caddr_t in gnu17, gnu2x, default, bsd, svid, gnu, lfs64, fob64, time64, dynstack, reentrant, fortify1-3 and C++;
  u_int8_t..u_int64_t, register_t, fsblkcnt_t, fsfilcnt_t wherever the header is accepted; blksize_t, suseconds_t,
  useconds_t, the *64_t names per visibility.tsv.
* `RE_TRANSLATE_TYPE`: `unsigned char *` (the `translate` member's type), regex.h and re_comp.h, GNU and C++ only.
* `_PATH_UTMP` "/var/run/utmp", `_PATH_WTMP` "/var/log/wtmp": paths.h everywhere, utmp.h/lastlog.h wherever
  accepted, utmpx.h GNU and C++; UTMP_FILE/WTMP_FILE in utmp.h and lastlog.h; UTMPX_FILE/WTMPX_FILE, _PATH_UTMPX,
  _PATH_WTMPX in utmpx.h GNU and C++ only.
* `sig_t`: `void (*)(int)` in signal.h, sys/signal.h, sys/wait.h, wait.h, sys/param.h, resolv.h, arpa/nameser.h in
  default, bsd, svid, gnu, gnu17/gnu2x, lfs64, fob64, time64, dynstack, reentrant, fortify1-3 and C++;
  `sighandler_t` GNU and C++ only (so apt's check finds sig_t in the default mode).
* xxhash: its build log shows S_ISDIR/S_ISREG expanding to S_IFMT/S_IFDIR in a configuration that hides those names;
  the reference keeps S_IS* usable there. Fixed by spelling S_IS* with the values.

## Results
* header_check (keke, headers-snapshot-gaps-hc2, form probe on): 0 failed of 17579; no FORM-DIFF on any name here.
* plants (keke, headers-snapshot-gaps-plants1, `--copies 4 --timeout 900`): see results in
  handoff/HEADERS-SNAPSHOT-GAPS-plants.txt. `--timeout 900` instead of 600: every header plant regenerates and runs
  header_check (about 6 min unloaded, keke load ~56/60), so 600 s risked a timeout counted as CAUGHT.
* debs (keke, headers-snapshot-gaps-debs1, serial 9, NIJI_DEB_COMMIT fc5f5b84a): system build + check-system.sh rc 0.
* ypk.sh on mei (headers-snapshot-gaps-ypk1, logs ~/work/headers-snapshot-gaps/logs-ypk): 7 of 7 packages PASS on
  libniji (tree and xxhash now PASS); busybox static: none of the earlier errors (u_long, RE_TRANSLATE_TYPE, wtmp,
  S_IFMT: 0 lines), make rc 2 on new gaps outside this story (below).

## INFORMATIONAL (not chased, per brief step 3)
* busybox (static, system flavour) next stops on: `<net/if.h>` struct ifreq member names ifr_name, ifr_hwaddr,
  ifr_flags, ifr_ifindex (networking/arp.c, arping.c, libbb/xconnect.c); `<netpacket/packet.h>` PACKET_HOST,
  PACKET_BROADCAST, PACKET_MULTICAST (arping.c); `<sys/mtio.h>` struct mtop/mtpos and MTWEOF, MTWSM, MTUNLOCK, ...
  (miscutils/mt.c).

## Next step
Land. Follow-up story for the busybox gaps above (net/if.h member macros, netpacket/packet.h, sys/mtio.h).
