COMPLETE: snapshot-ready at 88c0518f6 (serial 1; niji6 `0.0~snapshot20261124.1+88c0518f6ba4` and system `2.39+niji20261124.1-1` sort above 23.1's): #12 gencat catalogs byte-identical to stock gencat's (3 of 3 inputs) and read back as stock (17 of 17); #21 find formula passes 5 of 5 (23,584 KiB over the 3-file case; 23.1 32,320; stock 26,976); shadow stack: no host has SHSTK, every tunables row equals stock and 23.1; L2 fixed; every other check's verdicts equal 23.1's; start-up -2,061..-2,094 user instructions per start, faults unchanged

# RELEASE-SNAPSHOT-1124C: snapshot validation of 88c0518f6 (#12 gencat, #21 heap walk, shadow stack, #14 locale data)

**State.** Head under validation `88c0518f6ba416c75943f7f9d1c90aa69279ee84` (origin/next at the start, 2026-11-24
12:45Z, "Regenerate train-owned views"; commit time 2026-11-24T11:44:38Z, epoch 1795520678). Contains 88c0518f6.
**LD-START-MAPS-3 is not merged** (not in `git log --first-parent 80d1f809b..88c0518f6`, 41 commits: local scratch
`fplog.txt`); validated without it. No snapshot tag of 2026-11-24, so **serial 1**: system `2.39+niji20261124.1-1`,
coexist `0.0~snapshot20261124.1+88c0518f6ba4`; `dpkg --compare-versions` gt 23.1's for both. Branch
`task/release-snapshot-1124c` = head + handoff/ only. Role keke `~/w/n1` (Zen 1, products, .debs, gate, plants,
staging); root checks and Skylake-SP counts on mei; products and .debs relayed keke -> local -> mei, `sha256sum -c`
OK at each hop (9 .debs).

**Job dirs.** keke and mei `~/work/RELEASE-SNAPSHOT-1124C/` = 1124B's scripts through `copy.sh` + `subst.sed`
(head, epoch, serial 1, labels; the comparison side moves from 22.3 to 23.1: `RELEASE-SNAPSHOT-1124B` debs and
products, `-ref231`, p222; this head is p223). New this run (mei): `y12/` + `y12.sh` (#12: stock gencat vs libniji's
on three inputs: two message files, one call with both, an update of an existing catalog; byte compare, exit status
of malformed and missing input, catopen by path and NLSPATH, cross read), `ysk/` + `ysk.sh` (shadow stack: a marked
dynamic and static program and an unmarked library under the five D-21.6 tunables rows), both made by `mk2.sh` from
y24.sh's recipe; `cmp7.sh` (cmp6.sh against 1124B's logs; mask widened to 24 / both heads / -ref231 / p223);
run-mei.sh and run-ref.sh list y12 and ysk. Nothing of this run is left running on either host.

## Exact next step

None for this story: the coordinator decides on the tag and publication (nothing to GitHub, no tag from this run).
Staged: keke `~/release-staging/snapshot-20261124.1/` (7 system + 2 coexist .debs, GitHub-safe names (`~` -> `.`),
SHA256SUMS (`sha256sum -c`: 9 OK), MANIFEST).

## Results (labels `release-snapshot-1124c-<x>`)

| check | job(s) | result |
|---|---|---|
| build + smoke, Zen 1 / Skylake-SP | keke smoke-keke-1, mei smoke-mei-1 | `SMOKE: RESULT bad=0` both |
| gate | keke gate-1 | `GATE bad=0` (clippy all-features, xtask clippy, static, records check, direct tier) |
| #21 heap products test | keke heapnode-1 | 7 passed |
| both flavours, check, check-symbols | keke debs-1 | `DEBS bad=0`; check PASS (7; coexist 2); check-symbols PASS (4490 lines, 14 libraries; = 23.1) |
| plants (release checks) | keke plants-1 | 6 plants: 6 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY / WARN-NO-MSG (one WARN-DEV-SHM: plants dir under /dev/shm as briefed); results in handoff/RELEASE-SNAPSHOT-1124C-plants.txt |
| package diff vs 23.1 | keke pkgdiff-1 | libc-dev-bin +`/usr/bin/gencat` (7 -> 9 entries); libc6-dbg build-ids change; no package or other file dropped |
| stage, install steps | keke stage-1, install-1 | staged 9 + SHA256SUMS + MANIFEST; `inst: RESULT fails=0`, system debs identical 7 / different 0 |
| getaddrinfo_a (Zen 1) | keke gaimass-1 | `GAIMASS bad=0` |
| **#12** gencat | mei y12-2 (y12-1 superseded: probe defect, the cross read ran without its directory) | 17 PASS, 0 FAIL: `dpkg -L libc-dev-bin` names /usr/bin/gencat; catalogs byte-identical to stock gencat's (336, 619, 619 bytes); exit status as stock for each call, a malformed line and a missing file; catopen/catgets read-back (path and NLSPATH) equal stock's; libniji reads stock's catalog as stock does. 23.1 (y12-ref231-1): no gencat, 14 FAIL |
| **#21** | mei y21-1 / y21-ref231-1 | find formula 5 of 5 (both); table below |
| **shadow stack** | mei ysk-1 / ysk-ref231-1 | neither CPU class (Zen 1, Skylake-SP) has `user_shstk`; the kernel answers ARCH_SHSTK_STATUS (features 0). All 5 tunables rows, dynamic and static (75 lines) equal stock's, the unmarked dlopen loads in every row; same on 23.1. libniji's libc.so.6, ld.so, libm.so.6, crt1/crti/crtn.o carry the SHSTK property (programs linked against them are marked; trixie's stock crt files are not) |
| **#14** | mei y14l-1 / y14l-ref231-1 | `RESULT fail=0` both; bash's command compiles; read-back lines differing from stock: 0 of 37 (23.1: 8 of 37, L2) |
| B1, #25, #24, #11 | mei dlc, dlcd, y25, y25n, y25c, y25s, y2411 (-1; -ref231-1) | verdicts equal 23.1's (dlcd rc 1 by design as 1124B) |
| every other root check | mei roots, lint, yohane, many, dyn22, y10, y19, y26, y26t, y23, c19, y24, y28, y6, yres, y23p, y27, y13, y14, y6c, y14r, eetwet, y13s, drp, y6r, y13v, mint, sig32, y20, dpo600 (-1; y13s/drp/y6r/y13v also -ref231-1) | `cmp7.sh`: 39 of 40 jobs' masked verdict lines equal 23.1's; roots differs only by the work directory name (mask artifact). rc 1 by design as on 23.1: yohane, y14, y14r, drp |

**#21** (issue's probe, VmSize KiB, and find, trixie root, packaged find, normal exec): stock glibc 2.41 / 23.1 /
head: start 3,988 / 8,356 / 8,640; after 200,000 x malloc(96..159) 31,972 / 37,352 / 37,636; refill 33,556 /
47,592 / 47,876; realloc growth 32,040 / 49,160 / 49,444. Probe need 29,568 / 40,816 / 38,784. find over 3 files
4,640 / 21,584 / 22,144; 200,000 over 3 files 26,976 / 32,320 / **23,584** (allowance 35,000); the formula passes 5 of
5 on all three. Start and refill are +284 KiB over 23.1 (without LD-START-MAPS-3).

**Start-up** (user instructions / page faults per start, medians; glibc 2.39 / 23.1 (p222) / head (p223)):

| row | Zen 1: glibc | 23.1 | head | Skylake-SP: glibc | 23.1 | head |
|---|---|---|---|---|---|---|
| empty main | 101,533 / 48 | 156,382 / 52 | 154,296 / 52 | 107,752 / 49 | 156,623 / 52 | 154,535 / 52 |
| 3 libraries | 136,734 / 64 | 231,514 / 67 | 229,422 / 67 | 143,094 / 65 | 231,770 / 67 | 229,676 / 67 |
| clock_gettime | 103,672 / 50 | 167,387 / 56 | 165,300 / 56 | 109,862 / 50 | 167,599 / 55 | 165,510 / 55 |
| libresolv pair (res0) | 162,716 / 54 | 227,995 / 57 | 225,934 / 57 | 169,020 / 55 | 228,551 / 57 | 226,486 / 57 |

User instructions -2,061 to -2,094 per start (-0.9% to -1.3%) on both classes; faults equal 23.1's on every row. 23.1's
p222 numbers reproduce 1124B's exactly.

## 1124B's findings rechecked

* **L2 (d_t_fmt): fixed.** The en_US locale compiled from the shipped source formats `%c` as stock does; y14l's
  read-back differs from stock in 0 of 37 lines (23.1: 8).
* **L3 (lintian): unchanged.** errors 10, warnings 43 (= 23.1); `W: libc-bin: extended-description-line-too-long
  line 2` still (tools/deb/description/libc-bin line 2, `$programs` not wrapped).
* **+1 fault every row: unchanged.** Faults equal 23.1's on all 8 rows (still +1 over 22.3).
* **D1: unchanged.** tools/deb/description/locales line 1 still "locale support (placeholder)";
  docs/install-packages.md:160 still calls `locales` and `libc-l10n` placeholders.
* **D3: unchanged in kind, more stories uncovered.** Notes now come from docs/release-notes.d/ (3 fragments:
  UTIL-14-4 gencat, DOCS-CATCHUP-1121-3 #25 static, SYS-THREAD-DB-2). docs/RELEASE-NOTES.md has no section for the
  published snapshot-2026-11-23.1 (its header still names 22.3 as newest; the GitHub release has its notes). Landed
  since 23.1 with a user-visible effect and no fragment: HEAP-WALK-ADDRESS-SPACE / HEAP-FIND-DEPTH-NEED-3 (#21 size
  classes, find formula), SHSTK-OPT-IN-4 / SHSTK-ENABLE-2 (shadow stack opt-in through GLIBC_TUNABLES),
  LOCALE-C-UTF8-SOURCE, LOCALE-COLLATE-TAILORINGS-4, LOCALEDEF-COLLATE-GAPS-2 (#14 locale data and localedef),
  ICONV-LIST-ALIASES-2 (`iconv -l` lists 1028 names), the L2 fix, THREAD-CREATE-COST-2 and UNWIND-EXIT-COST-2 (thread
  create and exit cost). The gencat fragment's byte-identity claim is confirmed by y12.
* **D4: unchanged.** docs/compat-flags.md:33 (COMPAT-12) still names `crates/boundary/tests/s29_rpc_reply_source.rs`,
  which does not exist.

## Process note

The first ssh that started run-keke.sh stayed attached (the local call was stopped at its 10-minute limit; the
remote sequencer, started with setsid, ran on). Later starts used `timeout 20 ssh`, which returns at once.
