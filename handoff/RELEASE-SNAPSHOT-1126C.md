COMPLETE: snapshot-ready at 0f7aa0180 — step 4 GAPS-G's names and every 26.2 step-4 row 0 DIFF in C and C++ on both flavours (yb8 CTRL / n_time 156 same / 0 diff, was 84 DIFF; hp/hp2/hp3/yb6/yb7 as 26.2; `__USE_*` 0 differing lines); step 5 util-linux and inetutils build past `CTRL` / `n_time` and stop on further gaps (non-blocking): inetutils `<arpa/ftp.h>` FTP_NAMES tables (ftpd.c), util-linux `struct sched_attr` (chrt.c; stock 2.39 has no sched_setattr, libniji exports it at GLIBC_2.41); step 6 GAPS-G's 15 units rc 0 on the system headers; step 7 bindresvport / __libc_sa_len `@@GLIBC_2.2.5` as stock, getaddrinfo AI_ADDRCONFIG equal to stock on both flavours; perl, apt, packages, sweeps unchanged; start-up +76 instructions per start (non-blocking)

# RELEASE-SNAPSHOT-1126C: snapshot validation of 0f7aa0180 (serial 2026-11-26.3: GAI-ADDRCONFIG-LOOPBACK, EXPORT-LEGACY-DEFAULT, HEADERS-STAGE2-GAPS-G)

**State.** Done (1 leg). Head `0f7aa018052b19a5864ebc55107d1bde3688886b` (origin/next at the start, 21:06Z, "Regenerate
train-owned views"; commit time 2026-11-26T19:15:55Z, epoch 1795720555). First-parent log since 26.2 (dff0cc5c1):
gai-addrconfig-loopback (5a7c7a7b8), train regen (0b2cb1194), export-legacy-default (9eca50507), train regen
(6c7386544), release-snapshot-1126b (c450677b3), headers-stage2-gaps-g (7fa4931da), train regen (0f7aa0180); all three
stories in. **Serial 3**: system `2.39+niji20261126.3-1`, coexist `0.0~snapshot20261126.3+0f7aa018052b`; both
`dpkg --compare-versions` gt 26.2's. Branch = head + handoff/ only. Hosts as 1126B (coordinator #8745): builds in the
shiki role `~/w/sh1` (Haswell-EP: smoke, gate, .debs, plants, heapnode); counts and checks on keke (Zen 1) and mei
(Skylake-SP, stock 2.39 headers). Products and .debs relayed shiki -> local -> keke, mei, sha256 checked at every hop.
Job dirs shiki, keke, mei `~/work/RELEASE-SNAPSHOT-1126C/` = 1126B's via copy.sh + subst.sed (data file names
untouched: y23p/ref241.txt kept). libc.so.6 `1e9123e44e15e601` (26.2 `cf5043d13be5f2be`), ld.so `c1b0814f16ab99e3`
(26.2 `6c0bda72da3223c2`): both changed, nothing skipped. Staged: keke `~/release-staging/snapshot-20261126.3/` (9
debs + SHA256SUMS + MANIFEST). Nothing to GitHub, no tag.

## Step 4 (blocking): PASS
mei hp-1 104/0 DIFF, hp2-1 24/0, hp3-1 32/0, yb6-1 96/0, yb7-1 1408/0 (80 both-fail = 26.2's TELOPT rows), hp4-1
658/14 (the 14 DIFF rows byte-identical to 26.2's: `syslog-names-alone`). **yb8-1** (CTRL through sys/ttydefaults.h,
termios.h, sys/termios.h, termio.h, sys/ioctl.h; n_time/n_short/n_long through netinet/in_systm.h; gcc and g++, 3
modes, both flavours) **same=156 diff=0** (26.2: 84 DIFF); its 72 both-fail rows are CTRL via unistd.h/stdlib.h and
n_time via netinet/in.h, ip.h, ip_icmp.h, sys/types.h alone (fail on stock too). fp-1 (bash; names = 1126B's 245 + GAPS-G's
117 candidates = 362): `use` 0 differing lines both flavours; visibility (vcmp.py vs 1126B's logs-fp, every row):
**0 rows of 26.2's names changed class**; GAPS-G's 117 names have 31 non-equal rows, the same on both flavours: 12
stock-only = `<complex.h>` (10) and `<tgmath.h>` (2) in C++ only (the include-chain class known since 26.1: on stock g++
reaches libstdc++'s wrapper headers), 19 libniji-only (reserved `__BYTE_ORDER` family and `__bswap_*` also visible via
net/ppp_defs.h, netpacket/packet.h, utmpx.h). None narrower in C. Forms 15882 -> 15138 differing lines (stock 41773 rows:
GAPS-G's probe now compiles UAPI-including headers, so the row sets are not one to one with 26.2's). GAPS-G's 8 tag
names (fanotify_event_metadata, ethhdr, igmp, fddihdr, _ftsent64, fddi_8022_*/fddi_snap_hdr) are not in fp's names
list; the units of step 6 use fanotify_event_metadata, ethhdr and _ftsent64; igmp and the fddi tags are NOT VERIFIED.
keke hdrdiff-1 (26.2 libc6-dev vs this): 51 headers changed size, 1936 added names (GAPS-G's), "removals" 1: `<fts.h>`
under `_GNU_SOURCE` lost only `__niji_opaque`, `__aligned__`, `__attribute__` (GAPS-G's concrete FTS64 layout), no
public name.

## Step 5 (blocking): PASS by the brief's exception (both past CTRL / n_time, stop on further gaps)
mei ybuild-1: npth (4 debs), gnupg2 (29), libev (4), keyutils (5) as stock; util-linux and inetutils rc 2 on libniji
(stock 43 / 24 debs). Confirmed black-box by mei yb9-1 (yb8's method; 12 DIFF):
* **inetutils**: `<arpa/ftp.h>` with `FTP_NAMES` lacks `typenames`, `strunames`, `modenames`, `formnames` (stock has
  them, gcc and g++, 3 modes, both flavours; the FTP code macros TYPE_A, STRU_F, MODE_S, FORM_N, REC_ESC are SAME); unit
  `ftpd/ftpd.c`. `n_time` no longer appears in its errors.
* **util-linux**: `schedutils/chrt.c`: `struct sched_attr` incomplete, `sched_setattr`/`sched_getattr` implicit. Not a
  header difference from stock: yb9 shows `struct sched_attr` and both functions undeclared through `<sched.h>` on stock
  2.39 too (SAME, both fail). The libniji libc.so.6 exports `sched_setattr@@GLIBC_2.41` and `sched_getattr@@GLIBC_2.41`
  (stock 2.39: neither), so util-linux's configure most likely detects them and compiles the branch that needs the
  newer `<sched.h>` declarations (the build tree was not kept: the configure result itself is NOT VERIFIED).

## Step 6 (blocking): PASS
keke units-1: GAPS-G's units.sh / fetch.sh (copied to `units/`, only P, NI, L made parameters; sources re-fetched: its
RAM copy was gone) against this head's system libc6-dev (usr/include with x86_64-linux-gnu/ overlaid, 331 headers):
**15 units, each stock=0 niji=0** (systemd, elogind, eudev x2, openssh x2, liburing, iptables x3, fakechroot, libssh,
glib, libunwind x2).

## Step 7 (blocking): PASS
mei exp-1: `bindresvport` and `__libc_sa_len` are `FUNC GLOBAL ...@@GLIBC_2.2.5` in the system (libc6) and coexist
(niji6) libc.so.6, equal to stock's lines. getaddrinfo: 1126A's gai6.c probe (`::1`, `ip6-localhost`, the IPv6
passive wildcard, with and without AI_ADDRCONFIG; this host has only loopback/link-local IPv6) on the coexist loader
(gai6-1) and on the system libc6 .deb's loader and libc (gai6s-1, new): **0 differing lines** from stock on both
(26.2 gai6-1: 24 differing lines, the AI_ADDRCONFIG rows returned EAI_NONAME). GAI-ADDRCONFIG-LOOPBACK's own probe is in its scratch, not named
in its handoff, so gai6 (the probe that found the case) was used.

## Step 8 (blocking): unchanged from 26.2
cmp12 (cmp11 with the product mask widened to p2NN; both lists, 63 sets): every masked verdict set equals 26.2's except
roots' 4 work-dir path lines (as before): lint, yohane, many, dyn22, y10..y28, yres, y23p, dlc, dlcd, y13*, y14*, y6*,
y25*, y2411, y12, ysk, eetwet, drp, mint, sig32, y20, dpo600, hp*, yh, yhc, ypk, **yapt** (apt), **ystage2cmp** and
ystage2-niji (perl 5.40.1; bad=1 = the LC_IDENTIFICATION item, as 26.2), startup/kstart/rstart/startup215 verdicts.

## Step 9 (non-blocking)
* Static busybox: ypk-1 verdicts as 26.2 (cmp12 0). #21 heapnode-1 6 passed / 1 FAILED as 26.2
  (`find_over_200000_entries_bounds_its_address_space`, same panic line). The Haswell-EP role has one NUMA node (as 26.2).
* lint-1 masked lines equal 26.2's. Docs: no "no C headers" text in README.md, tools/deb/description/,
  docs/install-packages.md.
* **Start-up** (medians, user instructions / faults): every libniji row **+76 instructions**, faults equal, min and
  median alike: Zen 1 empty main 156,289 -> 156,365 / 52, 3 libraries 241,564 -> 241,640 / 67, clock_gettime 167,237
  -> 167,313 / 56; Skylake-SP 156,531 -> 156,607, 241,821 -> 241,897, 167,450 -> 167,526; kstart / rstart user side
  +76 too (156,193 -> 156,269). glibc rows and the p215 rows unchanged. Not chased (which of the three stories adds it
  is not measured).

## Builds, staging, plants
shiki smoke-shiki-1 `SMOKE: RESULT bad=0`; gate-1 `GATE bad=0` (clippy, xtask clippy, static, records, direct tier);
debs-1 `DEBS bad=0`, check PASS, check-symbols PASS (4492 lines, 14 libraries; 26.2: 4490, +2 = the two exports);
plants-1 **6 CAUGHT, 0 SURVIVED, no WARN** (handoff/RELEASE-SNAPSHOT-1126C-plants.txt). keke smoke-keke-1 bad=0,
pkgdiff-1 file counts equal to 26.2 for every package, stage-1, install-1 `inst: RESULT fails=0`, gaimass-1 bad=0.
mei smoke-mei-1 bad=0; root checks rc 1 by design as 26.2 (drp, dlcd, y14, y14r, yhc).

## Release notes for this snapshot should state
getaddrinfo with AI_ADDRCONFIG now counts loopback/link-local IPv6 as the reference does (perl's 4 IPv6 tests no
longer skip for that reason: NOT VERIFIED here, ystage2's test counts not re-read); `bindresvport` and `__libc_sa_len`
at their default version; GAPS-G's header rows (CTRL, n_time, byte-order names, obstack, ELF notes, IGMP, FDDI, ...).
util-linux (`sched_setattr` detection) and inetutils (`<arpa/ftp.h>` FTP_NAMES tables) still do not build from source.

## Findings for the coordinator
1. Next rows for Yohane's two packages: inetutils `<arpa/ftp.h>` FTP_NAMES tables (`typenames`, `strunames`,
   `modenames`, `formnames`; header story); util-linux `sched_attr`: libniji exports `sched_setattr`/`sched_getattr` at
   GLIBC_2.41 beyond stock 2.39 while `<sched.h>` declares neither: RULING NEEDED (recommend: declare `struct sched_attr`
   and both functions in `<sched.h>` under `_GNU_SOURCE`, matching the export, so a link test and the header agree; or
   stop exporting them while the ABI target is 2.39).
2. Start-up +76 instructions per start on every libniji row, both CPU classes (cost; source not identified).
3. Tooling: vcmp.py's labels are hard-coded ("26.1", "26.2", "GAPS-F names", "181 common names"); read its rows, not
   its labels. fp.sh must run under bash (done here).
4. Carried over: LC_IDENTIFICATION en_US ruling (ystage2cmp bad=1).

## Jobs (all on 0f7aa0180's build)
JOBS (rc 0): the json's `jobs` (shiki 5 incl. precheck-1, keke 10, mei 59). EVIDENCE (rc != 0 by finding, json
`evidence_jobs_rc_nonzero_by_finding`): shiki heapnode-1 (#21); keke hdrdiff-1 (the fts.h internal-name removal); mei
hp4-1 (syslog-names-alone), fp-1 (forms/visibility differences above), ybuild-1 (2 of 6 stop), yb9-1 (the two gaps),
ystage2cmp-1 (langinfo), drp-1, drp-ref262-1, dlcd-1, y14-1, y14r-1, yhc-1 (by design as 26.2).

## Not to redo / next step
Nothing left in this story. Next: the coordinator cuts snapshot 2026-11-26.3 from the staging dir; brief finding 1.
