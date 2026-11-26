COMPLETE: snapshot-ready at dff0cc5c1 — step 4 GAPS-F's rows 0 DIFF in C and C++ on both flavours (yb7 1408 same / 0 diff; yb6 96/0, was 84 DIFF; hp4's 32 intN rows now SAME), whole `__USE_*` set 0 differing lines; step 5 util-linux and inetutils now build past 26.1's stops and stop on further header gaps (non-blocking by the brief): util-linux `CTRL` (`<sys/ttydefaults.h>` and the terminal headers; text-utils/more.c), inetutils `n_time` (`<netinet/in_systm.h>`; ping's icmp.h); perl, apt, packages, sweeps, start-up equal to 26.1

# RELEASE-SNAPSHOT-1126B: snapshot validation of dff0cc5c1 (serial 2026-11-26.2: HEADERS-STAGE2-GAPS-F)

**State.** Done (2 legs). Head under validation `dff0cc5c1c071b0a9980428ceb7405303c3e1008` (origin/next at the start,
15:42Z, "Merge task/release-snapshot-1126a into next"; commit time 2026-11-26T14:49:44Z, epoch 1795704584). First-parent
log since 26.1 (cd31515dc): headers-fragments (05995ca18), docs-catchup-1126 (9cfa6166c), headers-stage2-gaps-f
(e5901d8f8), train regen (088bc95e3), release-snapshot-1126a (dff0cc5c1). No crates/ or csys/ change since 26.1.
**Serial 2**: system `2.39+niji20261126.2-1`, coexist `0.0~snapshot20261126.2+dff0cc5c1c07`; both sort above 26.1's.
Branch = head + handoff/ only. Hosts (coordinator #8745): builds on the shiki role `~/w/sh2` (Haswell-EP: smoke,
gate, .debs, plants, heapnode); counts and checks on keke (Zen 1) and mei (Skylake-SP, Ubuntu glibc 2.39 = stock
headers). Products and .debs relayed shiki -> local -> keke, mei, sha256 checked at every hop. Job dirs keke and mei
`~/work/RELEASE-SNAPSHOT-1126B/` = 1126A's via copy.sh + subst.sed (data file names untouched). libc.so.6
`cf5043d13be5f2be` and ld.so `6c0bda72da3223c2` identical to 26.1's; of 1686 product files 144 differ (143 headers +
the manifest). Staged: keke `~/release-staging/snapshot-20261126.2/` (9 debs + SHA256SUMS + MANIFEST). Nothing to
GitHub, no tag.

## Step 4 (blocking): PASS
mei hp-1 104/0 DIFF, hp2-1 24/0, hp3-1 32/0 (as 26.1). hp4-1 same=658 diff=14 (26.1: 626/46): the 32 intN rows
(`__int8_t`..`__uint64_t`) are now SAME; the 14 left are `syslog-names-alone` (stock's `<syslog.h>` under SYSLOG_NAMES
alone fails, libniji passes; as 26.1). yb6-1 (util-linux / inetutils names, 26.1: 84 DIFF) 96 same / 0 diff. **yb7-1**
(every name of GAPS-F's probe list: `<sys/io.h>` port I/O incl. ins*/outs*, `<arpa/telnet.h>` name macros and tables,
`<sys/types.h>` reserved integer types; gcc and g++, default / _GNU_SOURCE / -O2 / strict ISO, coexist and system)
**same=1408 diff=0**; its 80 both-fail rows are TELOPT, TELOPT_FIRST, TELOPT_LAST, TELOPT_OK and `telopts` used
without TELOPTS defined (fail on stock too; with the request macros defined they pass on both). fp-3: `use` (whole
`__USE_*` set, 3217 rows) **0 differing lines** both flavours (as 26.1). Visibility on 1126A's 181 names + GAPS-F's
64 (vcmp.py, every row): **0 rows changed class from 26.1**; GAPS-F's names have 46 non-equal rows, none narrower: 23
`<complex.h>`-in-C++ include-chain rows (26.1's known class) and 23 libniji-only reserved typedefs via
`<netpacket/packet.h>`. Forms: 16276 -> 15882 differing lines, all moves toward stock (FD_SET now as stock;
include-guard rows kept->replaced 191 -> 62; ifa_broadaddr, ifa_dstaddr, _PATH_RESCONF now as stock); **one new row**:
`SLC_NAMES` in `<arpa/telnet.h>` is kept on stock, `replaced-form` on libniji (a program's own definition is
replaced; yb7's compile probes with SLC_NAMES predefined are SAME). keke hdrdiff-1 (26.1 libc6-dev vs this): 143
headers changed, **0 with removals**, added public names = GAPS-F's.

## Step 5 (blocking): PASS by the brief's exception (both stop later, on further header gaps)
mei ybuild-1: npth (4 debs), gnupg2 (29), libev (4), keyutils (5) build as stock; util-linux and inetutils build on
stock and stop on libniji, each past its 26.1 stop. Gaps confirmed black-box by mei yb8-1 (yb7's method, gcc and g++,
default / _GNU_SOURCE / -O2, both flavours; stock passes, libniji fails; 84 DIFF):
* **util-linux**: `CTRL` is not defined through `<sys/ttydefaults.h>`, `<termios.h>`, `<sys/termios.h>`, `<termio.h>`
  or `<sys/ioctl.h>` (stock: all five); unit `text-utils/more.c` (implicit `CTRL`, then "case label does not reduce
  to an integer constant").
* **inetutils**: type `n_time` is missing through `<netinet/in_systm.h>` (stock has it); unit ping's `icmp.h`
  (`icmp_echo.o`, `icmp_address.o`). (`n_short`/`n_long` were probed only together with `n_time`.)
26.1's stopping units (`sys-utils/hwclock-cmos.c` port I/O, `libtelnet/auth.c`/`encrypt.c`): their names compile as
stock (yb6-1, yb7-1); the build log keeps no per-unit lines and make -j8 stopped elsewhere, so the build passing those
units is shown at compile level only (NOT VERIFIED from the build log).

## Step 6 (blocking): unchanged from 26.1
ystage2-niji-1 rc 0; ystage2cmp-1 verdict lines byte-identical to 26.1's (perl 5.40.1 builds on the system flavour, 7
debs as stock; same failing-test counts; bad=1 = the LC_IDENTIFICATION langinfo item, ruling still open). yapt-1 rc
0; ypk-1 7 of 7 packages, 0 failing only on libniji. cmp11 with both of 1126A's lists (19 + 35 sets): every masked
verdict set equals 26.1's except roots' 4 work-dir path lines (as 26.1's own comparison) and sig32-1 / dpo600-1 (4
lines each: identical verdicts, only the product label `p228` is outside cmp11's `p22[0-7]` mask). gai6-1 log
identical to 26.1's (D-16.2 AI_ADDRCONFIG, unchanged).

## Step 7 (non-blocking): equal to 26.1
* Static busybox builds (ypk-1 `busybox niji make=0`), applet checks as 26.1 (cmp11 ypk 0 differing).
* #21 heapnode-1 6 passed / 1 FAILED as 26.1 (`find_over_200000_entries_bounds_its_address_space`, same panic line).
  The Haswell-EP role has one NUMA node, so `the_heap_bounds_address_space_after_a_node_change` printed its skip
  reason there (it ran on 26.1's Zen 1 host): the migration case was not exercised on this build.
* lint-1 masked lines equal 26.1's. Docs: no "no C headers" text in README.md, tools/deb/description/ or
  docs/install-packages.md.
* **Start-up** (libc.so.6 and ld.so identical to 26.1's): medians, user instructions / faults: Zen 1 empty main
  156,289 / 52, 3 libraries 241,564 / 67, clock_gettime 167,237 / 56 (glibc 101,359 / 136,560 / 103,498); Skylake-SP
  156,531 / 52, 241,821 / 67, 167,450 / 55 (glibc 107,390 / 142,732 / 109,500); p215 Zen 1 medians 741,443 / 763,564
  (26.1 +1 each, min equal: noise). kstart / rstart user-side rows (median and min instructions and faults, syscall
  lists) equal 26.1's on both CPU classes.

## Step 8: staged; plants
Builds: shiki smoke-shiki-1 `SMOKE: RESULT bad=0`; gate-1 `GATE bad=0`; debs-1 `DEBS bad=0`, check PASS,
check-symbols PASS (4490 lines, 14 libraries); plants-1 **6 CAUGHT, 0 SURVIVED, no WARN**
(handoff/RELEASE-SNAPSHOT-1126B-plants.txt). keke smoke-keke-1 bad=0; pkgdiff-1 file counts equal to 26.1 for every
package; stage-1; install-1 `inst: RESULT fails=0`; gaimass-1 `GAIMASS bad=0`. mei smoke-mei-1 bad=0; root checks
rc 1 by design as 26.1 (drp, dlcd, y14, y14r, yhc).

## Release notes for this snapshot should state
GAPS-F: `<sys/io.h>` port I/O, `<arpa/telnet.h>` AUTHTYPE/ENCTYPE name macros, `<sys/types.h>` reserved integer types.
util-linux (`CTRL` in the terminal headers) and inetutils (`n_time` in `<netinet/in_systm.h>`) still do not build
from source. perl 5.40.1 and static busybox build; perl's 4 IPv6 tests skip on hosts with only loopback/link-local
IPv6 (D-16.2).

## Findings for the coordinator
1. Next header rows (Yohane's util-linux and inetutils): `CTRL` (`<sys/ttydefaults.h>`, reached from `<termios.h>`,
   `<sys/termios.h>`, `<termio.h>`, `<sys/ioctl.h>`); `n_time` (and probably `n_short`, `n_long`) in
   `<netinet/in_systm.h>`; forms: `SLC_NAMES` in `<arpa/telnet.h>` should keep a program's definition.
2. Carried over unchanged from 1126A: D-16.2 compat option (RULING NEEDED there); LC_IDENTIFICATION en_US ruling.
3. Tooling: the copied fp.sh called header_probe internals (`package_names()`, `stage2e_names()`) that HEADERS-FRAGMENTS
   moved into `spec/tools/header_probe.d/`; fp-1 stopped there. fp.sh now reads 1126A's own names.txt plus GAPS-F's
   fragment (`h.family('stage2-gaps-f')`). fp-2 then ran the bash script under dash (comparison step: syntax error);
   fp-3 = the same under bash. Next copy: run fp.sh with bash and keep the names file as data.

## Jobs (all on dff0cc5c1's build)
JOBS (rc 0): see the hand-back / json `jobs` (shiki gate-1, plants-1, smoke-shiki-1, debs-1, precheck-1; keke 10;
mei 56). EVIDENCE (rc != 0 by finding, json `evidence_jobs_rc_nonzero_by_finding`): shiki heapnode-1 (#21 as 26.1);
mei hp4-1 (syslog-names-alone), fp-3 (forms/visibility differences above), yb8-1 (the two gaps), ybuild-1 (2 of 6
stop), ystage2cmp-1 (langinfo), drp-1, drp-ref261-1, dlcd-1, y14-1, y14r-1, yhc-1 (by design as 26.1).
SUPERSEDED: mei fp-1 (header_probe API moved), fp-2 (run under dash).

## Not to redo / next step
Nothing left in this story. Next: the coordinator cuts snapshot 2026-11-26.2 from the staging dir; brief the header
rows of finding 1.
