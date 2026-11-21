IN PROGRESS (leg 1 of RELEASE-SNAPSHOT-1121D; a REFRESH leg collects the rest). Not a hand-back.

# RELEASE-SNAPSHOT-1121D: snapshot validation of cd1bbabdb (#14 zic/zdump, #20 stdio, WIDE-READ-FAST)

**State.** Head validated: `cd1bbabdba59474b6c9f8605ed08c057452ccbd1` (origin/next at the start, 16:01 UTC; "Regenerate
train-owned views"; commit time 2026-11-21T15:08:59Z, epoch 1795273739); contains a42e446a (TZ-TOOLS-2, #14) and
11381084 (FCLOSE-COOKIE-SEEK, #20). Branch `task/release-snapshot-1121d` = head + handoff/ only. Role **mei `~/w/s1`
(Skylake-SP)**; keke = Zen 1. 21.4 = e4d7011e6 (RELEASE-SNAPSHOT-1121C). Not in the head (CI queue at the start):
COMPAT19-LDSO-COST, DL-PROCESS-OBJECTS-2, VALGRIND-DBG-RODATA (#13), ICONV-CJK (#6): one line each in the report.

**Landed since 21.4** (`git log --oneline --first-parent e4d7011e6..cd1bbabdb`): release-snapshot-1121c (5e1903cce),
wide-read-fast (9f4eccfa9), view regen (7ca864312), sl-headers-s11-7 (4dbdcc68b), view regen (43058b7df),
heap-gap-test-keke (8e34f9c72), tz-tools-2 (a42e446af, #14), fclose-cookie-seek (113810845, #20), dl-rdebug-version
(8ca99d206), view regen (cd1bbabdb).

**Serial 5** (head dated 2026-11-21, fifth snapshot): NIJI_DEB_SERIAL=5, 2.39+niji20261121.5-1, stage keke
`~/release-staging/snapshot-20261121.5/`, tag snapshot-2026-11-21.5 (not made).

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121D/`; local edits in `~/w/.scratch/RELEASE-SNAPSHOT-1121D/mirror/`):
1121C's set by copy.sh + subst.sed (every input dir incl. y13/ y27/ sig32/); tree.tgz = git archive cd1bbabdb (8807
files = ls-tree; sha checked on both hosts). Plants file handoff/RELEASE-SNAPSHOT-1121D-plants.txt (each pattern
present once). New: **y14.sh** (#14; made by mknew4.sh from y21.sh's swapped-root recipe; y14/zt.sh runs zic default /
-b fat / -b slim / -b fat -L, zdump -v/-c/-i on 14 zones, 11 invalid names as zone and link, zdump on 4 non-zones, on
the stock root then after the swap; y14/tzdec.py decodes both zic trees per RFC 9636), **y20.sh** (#20; y20/y20.c,
37 rows, glibc 2.39 vs p214 vs p215; y20/recorded.txt = the ruled rows). Sequencing: run-mei.sh (1121C's),
run-new.sh (rstart, y21, yres), run-new4.sh (sig32, y20, then y23p, dlc, y27, y13, y14 on the debs), keke
run-keke.sh (chain-keke-pre + rstart-keke-1); chain-keke-debs.sh not yet started (needs the debs relayed).

## Results so far (labels release-snapshot-1121d-*)

| check | job | result |
|---|---|---|
| smoke | mei smoke-mei-1 | `SMOKE: RESULT bad=0` |
| gate | mei gate-1 rc 0 | clippy, xtask clippy, static, records check rc 0; affected: 0 of 309 targets (handoff-only); `GATE bad=0` |
| start-up counts | mei startup-mei-1, kstart-mei-1, rstart-mei-1 rc 0 | numbers not yet tabled (logs) |
| signal 32 | mei sig32-1 | `sig32: RESULT fail=0`; p214 and p215 identical (default before the first thread, handler after) |
| **#20** | mei y20-1 rc 0 | `RESULT fail=0`, 37 PASS: every row as glibc 2.39 except the 3 ruled ones (file r/r+ offset after fclose of a partial read 10 vs glibc 100; fflush hook EINVAL ferror 1 vs 0; no seek hook fflush 0 / ftell, fseek -1 ESPIPE vs -1 errno unchanged). 21.4 differs from head on 15 rows: fclose of a cookie stream with a failing hook -1 EINVAL after 1 seek (8 rows), seeks=1 at fclose with a working hook (4), fflush ESPIPE -1 EINVAL ferror 1, ftell/fseek ESPIPE -> EINVAL |
| **#14, 21.4 side** | mei y14-ref214-1 rc 1 (by design) | stock trixie (libc-bin 2.41-12+deb13u4, tzdata 2026c): zic writes 598 files in each mode, zdump 29,928 lines; after the 21.4 swap: no /usr/sbin/zic, /usr/bin/zdump, localedef |
| wide reads | mei percall-1 rc 0 (Skylake-SP; glibc / 21.4 / head, instructions per call) | fgetwc_unlocked 1/2/3/4-byte 35.6/58.1/84.9/98.1 / 363/640/908/1173 / 89.1/120.2/138.4/152.6; fgetwc 67.6/90.2/116.9/130.1 / 373/650/918/1183 / 95.1/126.2/144.4/158.6 (sd 0) |
| docs | doccheck.py (1120F's) | 23 problems, the same 23 lines as 21.4 (two line numbers moved); RELEASE-NOTES.md mentions none of zic/zdump, #20, wide reads |

Findings so far: no regression seen; FCLOSE-COOKIE-SEEK's three rulings have no row in docs/differences-from-glibc.md
yet (its handoff lists them as RULING NEEDED): INFORMATIONAL. The run-mei.out "Terminated" lines are the queue
ticker of jobrun being stopped when a slot frees (each followed a "slots: waiting" line; every job rc 0).

## Exact next step (leg 2)

1. mei: `~/bin/jobwait release-snapshot-1121d-debs-1`, then the jobs run-mei.sh / run-new.sh / run-new4.sh start
   (plants-1, y24-1, y28-1, y6-1, y25s-1, roots-1, lint-1, yohane-1, many-1, dyn22-1, y10-1, y19-1, y26-1, y26t-1,
   y23-1, c19-1, y25-1, y21-1, yres-1, y23p-1, dlc-1, y27-1, y13-1, y14-1). Wait with jobwait.
2. Relay mei debs to keke (rsync mei `work/RELEASE-SNAPSHOT-1121D/debs/{system,coexist}/*.deb` + debs.sha via local
   scratch `relay/`), then on keke `setsid nohup sh ~/work/RELEASE-SNAPSHOT-1121D/chain-keke-debs.sh > .../chain-keke-debs.out
   2>&1 < /dev/null &` (pkgdiff-1, stage-1, install-1). keke run-keke.sh (smoke-keke-1, startup-keke-1,
   kstart-keke-1, rstart-keke-1) is running (queued on slots at 16:28).
3. Compare: mei `sh ~/work/RELEASE-SNAPSHOT-1121D/cmp4.sh` (1121C's -1 logs), then by hand roots-1/lint-1 vs 1121c's
   -2, y27-1 vs 1121c-y27-2, y13-1 vs 1121c-y13-1, y23p, dlc. Table start-up from startup/kstart/rstart logs (both
   classes; glibc, p214, p215).
4. y14-1 verdict: decoded zic trees (tzdec lines in logs-y14/tzdec-*.txt); known differences listed by TZ-TOOLS may
   show; anything else is a finding. localedef absent (#14 partial).
5. Write the report here (1121C's shape), handoff/reports/RELEASE-SNAPSHOT-1121D.json with "jobs", precheck on the
   final head (`cargo xtask precheck --jobs <labels> --head <sha>` in ~/w/s1 after a sync), handback-check, hand back.
