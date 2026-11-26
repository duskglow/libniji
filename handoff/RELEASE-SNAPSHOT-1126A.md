WIP (leg 2 of RELEASE-SNAPSHOT-1126A): steps 4 and 5 judged (pass); ybuild-1, cmp11, start-up numbers, docs, report left

# RELEASE-SNAPSHOT-1126A: snapshot validation of cd31515dc (Yohane stage-2 header fixes: GAPS-D-6, GAPS-E-5, FEATURES-FORMS-5)

**State.** Head under validation `cd31515dc8be14b795d04d0a7924395e8832f42c` (origin/next at the start, 2026-11-26
10:28Z, "Regenerate train-owned views"; commit time 2026-11-26T08:48:25Z, epoch 1795682905). First-parent log since
25.2 (5a0acbb35): headers-enum-followups-2 (674b648b8), headers-net-sys-names-3 (88b76d217), release-snapshot-1125b
(db602e58c), headers-nonnull-match (818a8ec83), headers-gettext-clang (c74d35d7e), headers-stage2-gaps-c-3
(dc2a0f8e2), nl-locale-name-abi-4 (23b91a0a2), headers-stage2-gaps-b-4 (9f73af04d), review-20261126-1 (6d7c91f36),
headers-stage2-gaps-d-6 (d3bb2362a), affected-layout-rows-2 (6778b0b80), headers-stage2-gaps-e-5 (0323a4a9d),
plants-products-once (da67b9a02), headers-features-forms-5 (6df2e02ef), plus train view regenerations. HEADERS-FRAGMENTS
is NOT in. **Serial 1**: system `2.39+niji20261126.1-1`, coexist `0.0~snapshot20261126.1+cd31515dc8be`. Branch =
head + handoff/ only. Role keke `~/w/n1` (Zen 1); root checks, header probes, package builds on mei (Skylake-SP,
Ubuntu glibc 2.39 = stock headers). Job dirs keke and mei `~/work/RELEASE-SNAPSHOT-1126A/` (1125B's scripts via
copy.sh + subst.sed; local scratch `~/w/.scratch/RELEASE-SNAPSHOT-1126A/`). `logs-ystage2-stock` is COPIED from
1125B (stock side does not depend on libniji; job release-snapshot-1125b-ystage2-stock-3): say so in the report.
.debs relayed keke -> local -> mei, `sha256sum -c` OK at each hop (9 .debs).

## Done and judged (leg 2)
* keke: smoke-keke-1 `SMOKE: RESULT bad=0`; gate-1 `GATE bad=0`; heapnode-1 6 passed / 1 FAILED as 25.2 (#21
  find_over_200000..., every "needs" line identical to 25.2's); debs-1 `DEBS bad=0`, check PASS, check-symbols PASS
  (4490 lines, 14 libraries); plants-1 6 CAUGHT 0 SURVIVED, no WARN (results in handoff/RELEASE-SNAPSHOT-1126A-plants.txt);
  pkgdiff-1 file counts equal to 25.2 for every package, none missing; stage-1 `~/release-staging/snapshot-20261126.1/`;
  install-1 `inst: RESULT fails=0`, system debs identical 7; gaimass-1 `GAIMASS bad=0`.
* keke hdrdiff-1 (new, `hdrdiff.sh`): libc6-dev and niji6-dev Installed-Size -165 KiB vs 25.2 (223 headers smaller,
  elf.h and sys/auxv.h -97 KB each). Macro names + preprocessed identifiers, default and _GNU_SOURCE, 25.2 vs head:
  the only public name removed is `S_TYPEISTMO` (`<sys/stat.h>`), which stock 2.39 does not define (probed: stock has
  S_TYPEISMQ/SEM/SHM only), so it moves toward stock; other removals are internal (`__tu_sockaddr*`, `__nonnull__`
  spellings, `__NIJI_D_*` guards). rc 1 = "removals found" by design.
* **Step 4 (blocking): PASS.** mei hp-1 104/0 DIFF, hp2-1 24/0, hp3-1 32/0 (25.2: 22 DIFF). hp4-2 (every item of
  T1240Z/T1310Z/T1320Z + perl rows, both flavours, gcc/g++, 4 modes): every report-item probe SAME; TOTAL same=626
  diff=46, the 46 being (a) `syslog-names-alone` 14: stock's `<syslog.h>` under SYSLOG_NAMES alone fails (uses NULL
  undefined), libniji passes; with `<stddef.h>` first (iptables' real order) SAME; (b) `intN-family`/`int8-only` 32:
  libniji's `<sys/types.h>` has no `__int8_t`..`__uint64_t` (stock does): NOT a report item (hp4-1 had wrongly put
  `__int8_t` in u_char-family; hp4-1 superseded) -> next header row. 25.2 baseline hp4-ref252-3: same=174 diff=498.
  fp-1: `use` (whole `__USE_*` set, 3217 rows) **0 differing lines** both flavours (25.2: 1488). `forms` 16276 lines:
  1155 of 1189 replaced->kept names are kernel-spelled.tsv (guarded by design, FEATURES-FORMS open item); 34 others are
  FEATURES-FORMS' listed KNOWN follow-ups; vs 25.2 three changes: include-guard names (191 rows, `_AIO_H`...) now
  replaced (stock and 25.2 keep), FD_SET (62 rows) now kept (stock and 25.2 replace), ifa_broadaddr/ifa_dstaddr/
  _PATH_RESCONF now replaced-form (stock keeps); RE_* moved toward stock. Report item 2's named instances (SIZE_MAX,
  flex limits) SAME (hp4 flex-predefined-limits). Visibility (visx.py): 0 rows narrower on libniji; 31 stock-only =
  27 `<complex.h>`-in-C++ include-chain rows + GETSHORT/GETLONG/PUTSHORT/PUTLONG via `<arpa/nameser_compat.h>` alone
  (GAPS-E-4 moved them to nameser.h on purpose; via nameser.h/resolv.h equal to stock; on 25.2 visible nowhere);
  112 libniji-wider + 64 libniji-only (e.g. SOL_* via net/if_ppp.h in strict C) = namespace extras. Non-blocking.
* **Step 5 (blocking): PASS.** ystage2-niji-1 rc 0, ystage2cmp-1 (bad=1 = the 8 langinfo files, the known
  LC_IDENTIFICATION en_US RULING from 1125B): perl 5.40.1 builds on the system flavour, 7 debs = stock's 7; checked
  build rc 2 on both, the same 8 failing tests (Net-Ping ICMP), none only on libniji; test totals stock 2741/2742,
  libniji 2737/2736 (4-6 fewer tests run: NOT YET EXPLAINED, find which .t files differ: the build logs'
  test lines did not match my regex). Configure vs Debian's config: d_ip_mreq_source now as stock (25.2: undef);
  still different, same as 25.2: d_bsd define (stock undef), d_Gconvert gcvt (stock sprintf), gnulibc 2.39, sig_name
  (SIGRTMIN 33, D-20.1).
* mei root checks all ended (rc 1 by design as 25.2: dlcd, y14, y14r, drp, yhc; hp4/fp rc 1 = DIFF found).
* ybuild-ref252-1 (25.2 baseline, script check): stock 6/6 build; libniji 0/6: npth `npth_rwlock_t`, gnupg2
  certcache.o, util-linux TIMESPEC_TO_TIMEVAL, libev fd_mask, keyutils ns_t_srv/ns_s_an/ns_get16, inetutils tftphdr
  th_data.

## Exact next steps
1. mei `~/bin/jobwait release-snapshot-1126a-ybuild-1` (started 11:35Z); per package the first real `error:` (the
   log's grep sorts make lines first: read `logs-ybuild/out/build-niji-<p>.log`), header + name + unit (step 6).
2. Perl test-count gap: compare the .t result lines of `logs-ystage2-{stock,niji}/out/build-*-perl.log` (mei).
3. `sh cmp11.sh` with both of cmp10's lists (1125B's cmp10-a.out / cmp10-b.out headers) on mei; expect only yhc /
   roots path lines as in 25.2.
4. Start-up numbers vs 25.2 (keke startup/startup215/kstart/rstart-keke-1, mei *-mei-1 logs; 1125B table);
   `dpkg --compare-versions` both flavours gt 25.2's; lint-1 masked vs 25.2; docs check as 1125B.
5. Report (first line `COMPLETE: snapshot-ready at cd31515dc` unless ybuild/cmp11 show a blocking item), json with
   jobs, precheck, handback-check. JOBS so far: keke: gate-1, plants-1, smoke-keke-1, heapnode-1, debs-1, pkgdiff-1,
   stage-1, install-1, gaimass-1, hdrdiff-1, startup*-keke-1; mei: hp-1, hp2-1, hp3-1, hp4-2, fp-1, ystage2-niji-1,
   ystage2cmp-1, yapt-1, ypk-1, yh-1, yhc-1, ybuild-1, root checks. SUPERSEDED: hp4-1 (`__int8_t` mis-filed, syslog
   probe without stddef.h), hp4-ref252-1/-2.
