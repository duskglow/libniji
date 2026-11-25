# HEADERS-NET-SYS-NAMES: the header names a static busybox still misses (leg 3 handoff, for leg 4)

**State.** Branch `task/headers-net-sys-names` from origin/next 855efa170; **final code head 9ab89ce93** (any later
commit: handoff/report only). Role keke:~/w/n2, synced clean at 9ab89ce93. Scripts: `~/work/headers-net-sys-names/` on
keke (bbsweep.sh, bbstatic.sh, bbcc.sh, applets.txt, fm/ = the leg-3 macro probe), copies in the local scratch dir.
Decision in force: **D-26.3** (SYSLOG_NAMES tables end in `{ 0, -1 }`, no <stddef.h>). Reservation **D-2.15** was taken
by mistake in leg 1 (syslog is section 26) and is unused: to be released (report INFORMATIONAL).

## Commits
* leg 1: b1d372c90 header_probe `net_sys_names()`; abi_extract HEADERS + scsi/scsi.h, scsi/sg.h.
* leg 2: 879685bbf spec/abi tables (visibility, macros, typedefs, 27 struct layouts), templates (ip_icmp.h, icmp6.h,
  ip6.h, if_ether.h, net/route.h, sys/msg.h, syslog.h SYSLOG_NAMES + LOG_PRI/FAC/MAKEPRI, scsi/sg.h), tests s12.py
  member_names, s14.py (new file: TESTS line), s15.py sg_timeout_units.
* leg 3: 6ceffee9b scsi/scsi_ioctl.h `@@emit` (8 SCSI_IOCTL_* rows had no emitter: the hgcheck-1 FAILs); ab3b2a07e
  regenerated libc-include + crates/abi; a2c86e2dd IN_CLASSA..IN_BADCLASS (netinet/in.h), ICMP6_FILTER_* (icmp6.h);
  b17c834e8 P10 rows for both (spec/tools/p10_net.c); 9ab89ce93 header_check.py `--only REGEX` (a plant on one
  header ran all 18261 units, ~540 s, past the 600 s plant limit: plants-2 BASELINE FAILED) + plants file uses it.

## Results (keke, Zen 1 host)
* header_check 0 failed of 18261, P10 1409 lines identical (hgcheck-4, templates = final); P10 1410 identical, clippy
  -D warnings + `cargo xtask static` clean (final-1, at b17c834e8; no Rust changed after it).
* `cargo xtask affected`: FULL (escalated: crates/abi, generated header inputs): FULL is the train's.
* bbsweep (make -k, defconfig + CONFIG_STATIC; headers-net-sys-names-bbsweep-1): stock 15 errors in 1 file
  (networking/tc.c: host kernel UAPI lacks CBQ); **libniji-only: 15, all networking/nslookup.c** (`HEADER`, `ns_t_*`,
  `ns_s_an`: arpa/nameser.h, HEADERS-APT-GAPS' header, not landed: left alone per the brief).
* Static busybox (bbstatic.sh: tc and nslookup off on both sides; libniji side -nostdinc against libc-include, linked
  -nostdlib with niji-crt1/crti/crtn, libc.a, libm.a, empty libresolv etc.; link map: nothing of the host C library):
  **links; 401 applets both; 0 of 45 applet commands differ** (headers-net-sys-names-bbstatic-4; covers text tools,
  archives, hashes, date, awk floats, ash, logger facility/level names, ipcs -l, setarch/linux32, route, ifconfig, ip,
  eject, swapon, ping). bbstatic-1 found IN_MULTICAST, ICMP6_FILTER_SETPASS/SETPASSALL/SETBLOCKALL undefined at link
  (implicit declarations: warnings the error-only sweep cannot see) -> a2c86e2dd.
* SUPERSEDED: hgcheck-1 (the scsi_ioctl FAILs, fixed), hgcheck-2/3 (older heads), hgcheck-5 (killed: header_check.py
  changed), plants-1 (killed: stale head, blocked sync), plants-2 (BASELINE FAILED: 600 s timeout), bbstatic-1..3
  (missing macros; my wrapper's `-r` defect; one wrong applet line of mine), final-1 (b17c834e8, before --only).

## Probe-settled (black box, keke)
* Member-name expansions, SYSLOG_NAMES, LOG_*, SG_DEFAULT_TIMEOUT: as leg 2 (s12/s14/s15 docstrings).
* IN_CLASS*: every configuration; argument converted to an integer type (double compiles), evaluated once,
  parenthesised; int 1/0; A 0xxx, B 10xx, C 110x, D = MULTICAST 1110, EXPERIMENTAL 111x, BADCLASS 1111.
* ICMP6_FILTER_*: every configuration; set bit = block, bit type%32 of word type/32; WILL* int 1/0; SETPASS/SETBLOCK
  unsigned int expressions (type evaluated twice, filter once); SETPASSALL/SETBLOCKALL a bare compound statement.
* regerror(…, NULL, 0): no -Wnonnull on the reference, (…, NULL, 1) warns; libniji's nonnull(3) warns on both
  (INFORMATIONAL in the report json: attr_probe reads a size-bound access attribute as nonnull).

## Exact next steps (leg 4)
1. On keke: `~/bin/jobwait headers-net-sys-names-final-2 headers-net-sys-names-plants-3` (started ~08:30Z on
   9ab89ce93). final-2 = static, headers-gen --check, header_check --only scsi/sg, P10, full header_check: read
   `grep -E "^FAIL|header_check:|P10" ~/logs/headers-net-sys-names-final-2.log`. plants-3: 15 plants, read
   `grep -E "CAUGHT|SURVIVED|WARN|EQUIV|BASELINE|plants:" ~/logs/headers-net-sys-names-plants-3.log | tail -25`.
   A SURVIVED or WARN-NO-MSG: fix the plant's msg or the test, rerun only that (plants-4).
2. Append the plants result lines to handoff/HEADERS-NET-SYS-NAMES-plants.txt (model: HEADERS-BUSYBOX-GAPS-plants.txt).
3. handoff/reports/HEADERS-NET-SYS-NAMES.json: head -> 9ab89ce93 (or later code head), jobs -> final-2, bbsweep-1,
   bbstatic-4, plants-3 (+ final-1 for clippy); add the --only tooling impact; commit, push, sync (rc must be 0).
4. On the role: `cargo xtask precheck --jobs <labels> --head <sha>`, then `python3 ~/src/liella-ops/bin/ops
   handback-check HEADERS-NET-SYS-NAMES --head <sha> --jobs "<labels>"` (exit 0), hand back: FULL: the train's.
