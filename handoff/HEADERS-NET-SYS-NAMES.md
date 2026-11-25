# HEADERS-NET-SYS-NAMES: the header names a static busybox still misses (leg 3 handoff, final)

**State.** Branch `task/headers-net-sys-names` from origin/next 855efa170; final code head b17c834e8 (later commits:
handoff and report only). Role keke:~/w/n2. Scripts: `~/work/headers-net-sys-names/` on keke (bbsweep.sh, bbstatic.sh,
bbcc.sh, applets.txt, fm/ = the leg-3 macro probe), copies in the local scratch directory.
Decision in force: **D-26.3** (SYSLOG_NAMES tables end in `{ 0, -1 }`, no <stddef.h>). Reservation **D-2.15** was taken
by mistake in leg 1 (syslog is section 26) and is unused: to be released.

## Commits
* leg 1: b1d372c90 header_probe `net_sys_names()`; abi_extract HEADERS + scsi/scsi.h, scsi/sg.h.
* leg 2: 879685bbf spec/abi tables (visibility, macros, typedefs, 27 struct layouts), templates (ip_icmp.h, icmp6.h,
  ip6.h, if_ether.h, net/route.h, sys/msg.h, syslog.h SYSLOG_NAMES + LOG_PRI/FAC/MAKEPRI, scsi/sg.h), tests s12.py
  member_names, s14.py (new file: TESTS line), s15.py sg_timeout_units.
* leg 3: 6ceffee9b scsi/scsi_ioctl.h `@@emit` (the 8 SCSI_IOCTL_* rows had no emitter: the hgcheck-1 FAILs);
  ab3b2a07e regenerated libc-include + crates/abi; fb6e63d79 plants file; a2c86e2dd IN_CLASSA..IN_BADCLASS
  (netinet/in.h) and ICMP6_FILTER_* (netinet/icmp6.h); b17c834e8 P10 rows for both (spec/tools/p10_net.c).

## Results (keke, Zen 1 host)
* header_check: 0 failed of 18261 (headers-net-sys-names-hgcheck-4); P10 net macros 1410 lines, identical (final-1,
  final head). clippy -D warnings and `cargo xtask static`: clean (final-1, final head).
* `cargo xtask affected`: FULL (escalated: crates/abi, generated header inputs): FULL is the train's.
* bbsweep (make -k, defconfig + CONFIG_STATIC; headers-net-sys-names-bbsweep-1): stock 15 errors in 1 file
  (networking/tc.c: the host kernel UAPI lacks CBQ); libniji 30 in 2; **libniji-only: 15, all networking/nslookup.c**
  (`HEADER`, `ns_t_*`, `ns_s_an`: arpa/nameser.h, HEADERS-APT-GAPS' header, not landed: left alone per the brief).
* Static busybox (bbstatic.sh: defconfig + CONFIG_STATIC, tc and nslookup off on both sides; libniji side compiled
  -nostdinc against libc-include and linked -nostdlib with niji-crt1/crti/crtn, libc.a, libm.a, empty libresolv etc.;
  the link map shows nothing of the host C library): **links; 401 applets both; 0 of 45 applet commands differ**
  from stock (job headers-net-sys-names-bbstatic-4; applets.txt covers text tools, archives, hashes, date, awk floats,
  ash, logger facility/level names (SYSLOG_NAMES), ipcs -l, setarch/linux32, route, ifconfig, ip, eject, swapon, ping).
  Earlier runs: bbstatic-1 found IN_MULTICAST and ICMP6_FILTER_SETPASS/SETPASSALL/SETBLOCKALL undefined at link
  (implicit declarations: a warning, invisible to the error-only sweep) -> a2c86e2dd; bbstatic-2 a wrapper defect of
  mine (start files added to kbuild's `-r` partial links), fixed; bbstatic-3 one applet line of mine wrong (setarch
  x86_64 is not a busybox personality: both printed usage, differing only in the build timestamp).
* Plants: handoff/HEADERS-NET-SYS-NAMES-plants.txt, 15 plants, job headers-net-sys-names-plants-2 (final head).

## Probe-settled (black box, keke)
* Member-name expansions, SYSLOG_NAMES, LOG_*, SG_DEFAULT_TIMEOUT: as leg 2 (s12/s14/s15 docstrings).
* IN_CLASS*: every configuration; argument converted to an integer type (a double argument compiles), evaluated
  once, parenthesised; int 1/0; A 0xxx, B 10xx, C 110x, D = MULTICAST 1110, EXPERIMENTAL 111x, BADCLASS 1111.
* ICMP6_FILTER_*: every configuration; set bit = block, bit type%32 of word type/32; WILL* int 1/0; SETPASS/SETBLOCK
  expressions of type unsigned int (type evaluated twice, filter once); SETPASSALL/SETBLOCKALL a bare compound
  statement (not an expression; `if (c) M(p) else` compiles; `if (c) M(p); else` does not).
* regerror(…, NULL, 0) draws no -Wnonnull on the reference, (…, NULL, 1) does; libniji's nonnull(3) warns on both
  (INFORMATIONAL in the report: attr_probe models a size-bound access attribute as nonnull).

## Next step
Hand-back. Follow-ups: arpa/nameser.h (HEADERS-APT-GAPS) for nslookup; the attr_probe access-attribute gap.
