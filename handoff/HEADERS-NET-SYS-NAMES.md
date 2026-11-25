# HEADERS-NET-SYS-NAMES: the header names a static busybox still misses (leg 4: final, handed back)

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
* **At 9ab89ce93:** headers-net-sys-names-final-2 (rc 0): static, headers-gen --check, header_check --only scsi/sg
  0 failed of 117, P10 1410 lines identical, full header_check 0 failed of 18261. headers-net-sys-names-clippy-1
  (rc 0): `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo xtask static`.
  headers-net-sys-names-plants-3: 15 plants (results in HEADERS-NET-SYS-NAMES-plants.txt).
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
  (missing macros; my wrapper's `-r` defect; one wrong applet line of mine), final-1 (b17c834e8, before --only),
  hgcheck-4 (b17c834e8; final-2 repeats it at 9ab89ce93).

## Probe-settled (black box, keke)
* Member-name expansions, SYSLOG_NAMES, LOG_*, SG_DEFAULT_TIMEOUT: as leg 2 (s12/s14/s15 docstrings).
* IN_CLASS*: every configuration; argument converted to an integer type (double compiles), evaluated once,
  parenthesised; int 1/0; A 0xxx, B 10xx, C 110x, D = MULTICAST 1110, EXPERIMENTAL 111x, BADCLASS 1111.
* ICMP6_FILTER_*: every configuration; set bit = block, bit type%32 of word type/32; WILL* int 1/0; SETPASS/SETBLOCK
  unsigned int expressions (type evaluated twice, filter once); SETPASSALL/SETBLOCKALL a bare compound statement.
* regerror(…, NULL, 0): no -Wnonnull on the reference, (…, NULL, 1) warns; libniji's nonnull(3) warns on both
  (INFORMATIONAL in the report json: attr_probe reads a size-bound access attribute as nonnull).

## Exact next step
Handed back in leg 4 (final code head 9ab89ce93; later commits are handoff/report only). Next: land through the train
(FULL is the train's: `cargo xtask affected` escalates on crates/abi and the generated header inputs). If plants-3 was
still running at hand-back (PENDING JOBS), read `grep -E "CAUGHT|SURVIVED|WARN-|EQUIV|BASELINE" ~/logs/
headers-net-sys-names-plants-3.log` on keke; a SURVIVED or WARN-NO-MSG: fix that plant's msg or its test, rerun only it.
Follow-ups (not this story): arpa/nameser.h names (HEADERS-APT-GAPS); attr_probe size-0 access-attribute probe; release
reservation D-2.15.
