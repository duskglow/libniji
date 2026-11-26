# HEADERS-STAGE2-GAPS-H: handoff (run 1, end of leg 1)

Branch task/headers-stage2-gaps-h from origin/next 0f7aa0180; role keke:~/w/n1. No decision rows (black-box probes
only). Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-H/` (probe.py, probe2.py, probe3.py, test1.sh, pkg.sh,
ybuild/); keke `~/work/headers-stage2-gaps-h/` (same scripts; stock*.tsv = reference results, niji*.tsv = libniji).

## Done (pushed)
* 730f215ea: `<arpa/ftp.h>` FTP_NAMES tables (fragment headers/arpa/ftp.h.d/010-name-tables.frag, `@@fragments` in
  ftp.h.in); header_check fragment s13.d/30-stage2-gaps-h.py (units "FTP_NAMES absent / empty / 0 arpa/ftp.h");
  conformance/c/s13_arpa_ftp_names.c (the strings; TESTS: line in the commit).
* Probe-settled (keke, stock 2.39, 11 configurations C and C++): each table `char *[]`, external linkage (nm `D`, two
  TUs with FTP_NAMES = multiple definition), defined under `#ifdef FTP_NAMES` (empty and 0 both), undeclared without
  it; typenames 5 ("0" ASCII EBCDIC Image Local), strunames 4 ("0" File Record Page), modenames 4 ("0" Stream Block
  Compressed), formnames 4 ("0" Nonprint Telnet Carriage-control); no null terminator.
* Proof so far (job headers-stage2-gaps-h-test1 on keke): headers-gen --check ok; xtask headers tests 29 passed;
  header_check 0 failed of 26078; probe.py libniji vs stock 0 DIFF (ftp + telnet lines); telnet tables (probe2) 0 DIFF.
  Conformance result: read `~/work/headers-stage2-gaps-h/conf1.log` and the test1 log tail.

## Sweep (item 2) results
* `<arpa/ftp.h>`: the only switch is FTP_NAMES (nm sweep of 22 candidate switches); nothing else defined by it.
* `<arpa/telnet.h>`: TELCMDS, TELOPTS, AUTH_NAMES, ENCRYPT_NAMES, SLC_NAMES as GAPS-F: 0 DIFF (no regression).
* `<arpa/tftp.h>`, `<arpa/inet.h>`, `<arpa/nameser*.h>`: no switch defines anything; tftp names probed SAME.
* `<protocols/routed.h>` RIPCMDS defines `ripcmds` (5: "#0" REQUEST RESPONSE TRACEON TRACEOFF);
  `<protocols/timed.h>` TSPTYPES defines `tsptype` (25: ANY .. LOOP). Neither declared without its switch.
* GAP (report, not chased): libniji's `<protocols/routed.h>`, `<protocols/timed.h>`, `<protocols/rwhod.h>`,
  `<protocols/talkd.h>` lack their protocol names entirely (probe3: RIPVERSION, RIPCMD_*, HOPCNT_INFINITY,
  struct rip/netinfo; TSPVERSION, TSP_*, TSPTYPENUMBER, ANYADDR, struct tsp; WHODVERSION, WHODTYPE_STATUS,
  struct whod/outmp; TALK_VERSION, LOOK_UP, ANNOUNCE, ..., NAME_SIZE, TTY_SIZE, struct osockaddr: stock 1, libniji 0).
  inetutils' talk/talkd use `<protocols/talkd.h>`: likely the package build's next stop. Recommend one follow-up
  story for the four protocols headers (with ripcmds/tsptype).

## Jobs started at the end of leg 1 (keke)
* headers-stage2-gaps-h-plants1: `cargo xtask plants handoff/HEADERS-STAGE2-GAPS-H-plants.txt --dir
  /dev/shm/claude/headers-stage2-gaps-h/plants-1` (6 plants).
* headers-stage2-gaps-h-pkg1: `sh ~/work/headers-stage2-gaps-h/pkg.sh` (system-flavour debs from ~/w/n1, then
  inetutils in a trixie chroot, stock then libniji; RELEASE-SNAPSHOT-1126C's method). Must run alone on the role.

## Next leg: item 3 (coordinator note 2026-11-26T23:08Z, owner ruling on #8766 option A)
Declare `struct sched_attr`, `sched_setattr`, `sched_getattr` in `<sched.h>` with the form, layout (offsetof/sizeof
per member), attributes and feature-macro visibility of stock **2.41**: probe Devuan excalibur libc6-dev 2.41 headers
through the compiler only (never read them); 2.39 stays the reference for everything else. Also probe a program's own
`struct sched_attr` and `<linux/sched/types.h>` together with `<sched.h>` on 2.41 and match. Record the addition beside
R3 (prototypes-additions.tsv, the spec row naming R3). Proof: probes 0 DIFF against 2.41 (C and C++, both flavours);
util-linux builds from source against the system flavour (or report the next stop). Size M; extend-budget once if needed.
Steps: (1) jobwait plants1 and pkg1, record results here; (2) find where 2.41 headers can be compiled against (a Devuan
excalibur chroot by mmdebstrap --mode=unshare, keyring ~/keyrings/devuan-archive-keyring.gpg on keke); (3) probe, add
rows/fragment, header_check unit, plants; (4) util-linux via pkg.sh with P=util-linux; (5) clippy, static, precheck,
report json, handback-check.

## Not to redo
The four FTP tables' probes; the arpa/protocols switch sweep; telnet regression.
