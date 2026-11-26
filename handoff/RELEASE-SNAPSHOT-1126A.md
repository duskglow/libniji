COMPLETE: snapshot-ready at cd31515dc — step 4 (Yohane's stage-2 rows) 0 DIFF on every report item, the whole `__USE_*` set equal to stock; step 5 perl 5.40.1 builds on the system flavour (7 debs as stock), same 8 failing tests (Net-Ping ICMP) as stock, 4 IPv6 test files skipped (D-16.2 AI_ADDRCONFIG); step 6 4 of 6 Yohane packages build (25.2: 0 of 6); static busybox now builds and its applets match stock; everything else equals 25.2

# RELEASE-SNAPSHOT-1126A: snapshot validation of cd31515dc (Yohane stage-2 header fixes: GAPS-D-6, GAPS-E-5, FEATURES-FORMS-5)

**State.** Done (3 legs). Head under validation `cd31515dc8be14b795d04d0a7924395e8832f42c` (origin/next at the start,
2026-11-26 10:28Z, "Regenerate train-owned views"; commit time 2026-11-26T08:48:25Z, epoch 1795682905). First-parent
log since 25.2 (5a0acbb35): headers-enum-followups-2 (674b648b8), headers-net-sys-names-3 (88b76d217),
release-snapshot-1125b (db602e58c), headers-nonnull-match (818a8ec83), headers-gettext-clang (c74d35d7e),
headers-stage2-gaps-c-3 (dc2a0f8e2), nl-locale-name-abi-4 (23b91a0a2), headers-stage2-gaps-b-4 (9f73af04d),
review-20261126-1 (6d7c91f36), headers-stage2-gaps-d-6 (d3bb2362a), affected-layout-rows-2 (6778b0b80),
headers-stage2-gaps-e-5 (0323a4a9d), plants-products-once (da67b9a02), headers-features-forms-5 (6df2e02ef), plus train
view regenerations. HEADERS-FRAGMENTS is NOT in (landed after the head), so its byte-identical claim was not checked
here. **Serial 1**: system `2.39+niji20261126.1-1`, coexist `0.0~snapshot20261126.1+cd31515dc8be`; both sort above
25.2's (`dpkg --compare-versions` gt `2.39+niji20261125.2-1` and `0.0~snapshot20261125.2+5a0acbb35533`). Branch = head
+ handoff/ only. Role keke `~/w/n1` (Zen 1); root checks, header probes and package builds on mei (Skylake-SP, Ubuntu
glibc 2.39 = stock headers). Job dirs keke and mei `~/work/RELEASE-SNAPSHOT-1126A/` (1125B's scripts via copy.sh +
subst.sed). `logs-ystage2-stock` is COPIED from 1125B (the stock side does not depend on libniji; job
release-snapshot-1125b-ystage2-stock-3). .debs relayed keke -> local -> mei, `sha256sum -c` OK at each hop (9 .debs).
Staged: keke `~/release-staging/snapshot-20261126.1/`. Nothing to GitHub, no tag.

## Step 4 (blocking): PASS
mei hp-1 104/0 DIFF, hp2-1 24/0, hp3-1 32/0 (25.2: 22 DIFF). hp4-2 (every item of T1240Z/T1310Z/T1320Z + perl rows,
both flavours, gcc/g++, 4 modes): every report-item probe SAME; TOTAL same=626 diff=46, the 46 being (a)
`syslog-names-alone` 14: stock's `<syslog.h>` under SYSLOG_NAMES alone fails (NULL undefined), libniji passes; with
`<stddef.h>` first (iptables' real order) SAME; (b) `intN-family`/`int8-only` 32: libniji's `<sys/types.h>` has no
`__int8_t`..`__uint64_t` (stock does): not a report item -> next header row. 25.2 baseline hp4-ref252-3: same=174
diff=498. fp-1: `use` (whole `__USE_*` set, 3217 rows) **0 differing lines** both flavours (25.2: 1488). `forms` 16276
lines: 1155 of 1189 replaced->kept names are kernel-spelled.tsv (guarded by design, FEATURES-FORMS open item); 34 are
FEATURES-FORMS' listed KNOWN follow-ups; vs 25.2: include-guard names (191 rows, `_AIO_H`...) now replaced (stock and
25.2 keep), FD_SET (62 rows) now kept (stock and 25.2 replace), ifa_broadaddr/ifa_dstaddr/_PATH_RESCONF now
replaced-form (stock keeps); RE_* moved toward stock. Visibility (visx.py): 0 rows narrower on libniji; 31 stock-only =
27 `<complex.h>`-in-C++ include-chain rows + GETSHORT/GETLONG/PUTSHORT/PUTLONG via `<arpa/nameser_compat.h>` alone
(GAPS-E-4 moved them to nameser.h on purpose; via nameser.h/resolv.h equal to stock); 112 libniji-wider + 64
libniji-only = namespace extras. keke hdrdiff-1: libc6-dev / niji6-dev Installed-Size -165 KiB vs 25.2; the only public
macro name removed is `S_TYPEISTMO`, which stock 2.39 does not define (moves toward stock).

## Step 5 (blocking): PASS
ystage2-niji-1 rc 0, ystage2cmp-1 (bad=1 = the 8 langinfo files: the LC_IDENTIFICATION en_US ruling from 1125B,
unchanged). perl 5.40.1 builds on the system flavour, 7 debs = stock's 7. Checked build rc 2 on both: the same 8
failing tests (Net-Ping `001_new`, `110_icmp_inst` ..., ICMP: no raw sockets under unshare), none only on libniji.
Configure vs Debian's config: d_ip_mreq_source now as stock (25.2: undef); still as 25.2: d_bsd define (stock undef),
d_Gconvert gcvt (stock sprintf), gnulibc 2.39, sig_name (SIGRTMIN 33, D-20.1).
**Test counts** ("Failed 4 tests out of N", N = files run, skips excluded): stock 2741 + 2742, libniji 2737 + 2736, 5
fewer per run. **4 explained**: `cpan/IO-Socket-IP/t/06local-cross-v6`, `cpan/IO-Socket-IP/t/16v6only`,
`cpan/libnet/t/pop3_ipv6`, `cpan/libnet/t/smtp_ipv6` are `ok` on stock and `skipped` on libniji in both runs. Cause,
probed black-box (mei gai6-1, gai6.c: stock 2.39 vs the coexist loader, same host): with `AI_ADDRCONFIG`, getaddrinfo
of `::1`, `ip6-localhost` and the IPv6 passive wildcard returns the address on stock and EAI_NONAME on libniji; every
call without AI_ADDRCONFIG is equal. The host has only `::1` and link-local IPv6 (plus global IPv4). That is the
documented difference D-16.2 (docs/differences-from-glibc.md: "Loopback and link-local IPv6 do not count as
configured"); IO::Socket::IP sets AI_ADDRCONFIG by default, so the tests skip. **The 5th per run is not identified**
(NOT VERIFIED): the two harness runs write interleaved into one log, so per-file verdicts are recoverable only from
lines that parse cleanly; the only other file consistently ok-on-stock / skipped-on-libniji is
`cpan/CPAN/t/32pushyhttps`, but its skip conditions (AUTHOR_TEST unset, curl/wget, IO::Socket::SSL) are environment-only
and both build roots installed the same 39 packages, so that reading is probably an interleaving artefact.

## Step 6 (non-blocking): 4 of 6 build (25.2: 0 of 6)
mei ybuild-1: npth (4 debs), gnupg2 (29), libev (4), keyutils (5) build on libniji as on stock. Still stopping (both
confirmed by mei yb6-1, hp4's method, C and C++, default/_GNU_SOURCE/-O2, both flavours, 84 DIFF / 12 SAME):
* **util-linux**: `<sys/io.h>` lacks the port I/O family `outb`, `inb` (and `outw`, `outl`, `inw`, `inl`, `outb_p`,
  `inb_p`; `ioperm`/`iopl` are present, SAME); unit `sys-utils/hwclock-cmos.c` (cmos_read). 25.2 stopped earlier
  (TIMESPEC_TO_TIMEVAL), now fixed.
* **inetutils**: `<arpa/telnet.h>` lacks `AUTHTYPE_NAME`, `AUTHTYPE_NAME_OK`, `ENCTYPE_NAME`, `ENCTYPE_NAME_OK`
  (macros, missing with or without AUTH_NAMES/ENCRYPT_NAMES); units `libtelnet/auth.c`, `libtelnet/encrypt.c`. 25.2
  stopped earlier (tftphdr th_data), now fixed.

## Steps 7 and 8 (non-blocking): equal to 25.2 or better
* apt and 1125B's packages and sweeps: yapt-1, ypk-1 (7 of 7 packages, 0 failing only on libniji), yh-1, yhc-1;
  cmp11 (both of cmp10's lists, 53 sets): every masked verdict set equals 25.2's except roots' 4 work-dir path lines
  (as in 25.2's own comparison); yhc 242 -> 242, 0 differing (25.2 had 8).
* **Static busybox now builds** (ypk-1, `busybox niji make=0`; 25.2: make rc 2) and its 10 applet checks (id, chown,
  stat, ping, nslookup) print the same as stock's.
* #21 heapnode-1 6 passed / 1 FAILED as 25.2 (`find_over_200000_entries_bounds_its_address_space`, every "needs" line
  identical). lint-1 masked lines equal 25.2's (cmp11).
* **Start-up** (user instructions / faults per start, medians): libc.so.6 changed (cf5043d13be5f2be vs
  c78ee370c65bdadf), every libniji row unchanged: Zen 1 empty main 156,289 / 52, 3 libraries 241,564 / 67,
  clock_gettime 167,237 / 56 (glibc 101,359 / 136,560 / 103,498); Skylake-SP 156,531 / 241,821 / 167,450 (glibc
  107,390 / 142,732 / 109,500). p215 rows: medians +1 instruction with min/max unchanged (741,443 -> 741,444,
  763,564 -> 763,565); kstart / rstart user-side rows equal 25.2's (kernel side varies run to run on both sides).
* keke: smoke-keke-1 `SMOKE: RESULT bad=0`; gate-1 `GATE bad=0`; debs-1 `DEBS bad=0`, check PASS, check-symbols PASS
  (4490 lines, 14 libraries); plants-1 6 CAUGHT 0 SURVIVED, no WARN (handoff/RELEASE-SNAPSHOT-1126A-plants.txt);
  pkgdiff-1 file counts equal to 25.2 for every package; stage-1; install-1 `inst: RESULT fails=0`, system debs
  identical 7; gaimass-1 `GAIMASS bad=0`. mei root checks all ended (rc 1 by design as 25.2: dlcd, y14, y14r, drp, yhc).
* Docs: no "no C headers" text in README.md, tools/deb/description/ or docs/install-packages.md.

## Release notes for this snapshot should state
perl 5.40.1 builds from source; its 4 IPv6 tests skip on hosts with only loopback/link-local IPv6 (D-16.2). Static
busybox builds. util-linux (`<sys/io.h>` port I/O) and inetutils (`<arpa/telnet.h>` AUTHTYPE/ENCTYPE name macros) do not
build from source yet.

## Findings for the coordinator
1. RULING NEEDED: D-16.2's AI_ADDRCONFIG difference has no compat option (CLAUDE.md rule 2 needs one or a recorded
   reason) and now shows in a distribution build (perl's IPv6 tests skip on build hosts without a global IPv6
   address; programs binding `::1` through IO::Socket::IP or AI_ADDRCONFIG fail there). Recommend: a COMPAT option
   restoring the reference's result, or reconsider counting the loopback literal when asked for by name.
2. Next header rows: `<sys/io.h>` port I/O functions; `<arpa/telnet.h>` AUTHTYPE_NAME(_OK)/ENCTYPE_NAME(_OK);
   `<sys/types.h>` `__int8_t`..`__uint64_t` (hp4 intN rows); FEATURES-FORMS' open items (FD_SET kept, include guards
   replaced, ifa_* forms).
3. LC_IDENTIFICATION en_US ruling (from 1125B) still open: ystage2cmp-1's only bad item.

## Jobs (all on cd31515dc's build)
JOBS (rc 0): keke: gate-1, plants-1, smoke-keke-1, debs-1, pkgdiff-1, stage-1, install-1, gaimass-1, startup-keke-1,
startup215-keke-1, kstart-keke-1, rstart-keke-1, precheck-1 (GREEN, 34 checks, on 31366d40d). mei: hp-1, hp2-1, hp3-1,
ystage2-niji-1, yapt-1, ypk-1, yh-1, gai6-1, smoke-mei-1, roots-1, lint-1, startup/startup215/kstart/rstart-mei-1, and
the other root checks of cmp11's lists. EVIDENCE (rc != 0 by finding, not in JOBS; the json's
`evidence_jobs_rc_nonzero_by_finding` gives each reason): keke hdrdiff-1, heapnode-1 (#21 as 25.2); mei hp4-2,
hp4-ref252-3, fp-1, fp-ref252-1, yb6-1, ybuild-1, ybuild-ref252-1, yhc-1, ystage2cmp-1. SUPERSEDED: hp4-1 (`__int8_t` mis-filed, syslog probe without stddef.h), hp4-ref252-1/-2 (baseline
reruns; -3 is the one used).

## Not to redo / next step
Nothing left in this story. Next: the coordinator cuts snapshot 2026-11-26.1 from the staging dir; brief the two
header rows above for util-linux and inetutils.
