COMPLETE: snapshot-ready at 2a76723f2 (snapshot-2026-11-22.3, serial 3 of the commit date 2026-11-22); three localedef findings (L1-L3), not blocking (ruling asked)

# RELEASE-SNAPSHOT-1122C: snapshot validation of 2a76723f2 (UTIL-12-3 localedef #14, HEAP-SPILL-COLD-3 #21, LD-PHDR-ONCE-2)

**State.** Head under validation `2a76723f2ed41f4aa4cac7b5d73b70fa295c2c61` (origin/next at the start, 14:09 UTC;
"Regenerate train-owned views"; commit time 2026-11-22T13:32:44Z, epoch 1795354364). Commit date 2026-11-22; 22.1 and
22.2 were serials 1 and 2, so this is **serial 3**: `NIJI_DEB_SERIAL=3`, `2.39+niji20261122.3-1` (coexist
`0.0~snapshot20261122.3+2a76723f2ed4`). Branch `task/release-snapshot-1122c` = that head + handoff/ only. Role **mei
`~/w/s2` (Skylake-SP)**; keke = Zen 1 (products and .debs relayed from mei, sha256 checked on arrival). 22.2 =
302dd7c3a (RELEASE-SNAPSHOT-1122B). Nothing to GitHub, no tag.

**Landed since 22.2** (`git log --first-parent 302dd7c3a..2a76723f2`): **heap-spill-cold-3 (6e2417c16)**, view
regeneration, stats-test-isolation (e88018eb6), dl-find-object-fast-3 (4252fa98d), view regeneration, **util-12-3
(15337f45c)**, release-snapshot-1122b (a0df4ded5), view regeneration, **ld-phdr-once-2 (9795fb79c)**,
root-harness-followup (8f888d550), sl-headers-s13 (40c73bde4), view regeneration (2a76723f2). All three the brief
names are in the head.

**Procedure.** 1122B's scripts (mei and keke `~/work/RELEASE-SNAPSHOT-1122C/`, made by `copy.sh` + `subst.sed`: head,
epoch, serial, role and labels only; every input directory copied recursively, tree/ from `git archive 2a76723f2`).
Plants file `handoff/RELEASE-SNAPSHOT-1122C-plants.txt` (1122B's six; each pattern present once in the head). Changed
in run-mei.sh: y21 moved to the `-ref222` list (22.2's .debs beside the head); new role job `heapnode` after the gate
(`cargo test -p niji --test heap_meta_products -- --nocapture`); new check `y14l` (#14, below). Comparison: 1122B's
`cmp4.sh` (masked verdict lines, this run's -1 logs against 1122B's) plus the last verdict line of every other job.

## Results (all on 2a76723f2's products and .debs; labels prefixed `release-snapshot-1122c-`)

| check | jobs | result | vs 22.2 |
|---|---|---|---|
| smoke | mei smoke-mei-1, keke smoke-keke-1 | `SMOKE: RESULT bad=0` both | same |
| gate (clippy all-features, xtask clippy, static, records, direct tier) | mei gate-1 | `GATE bad=0`, peak 2.4G | same |
| heap node change (#21) | mei heapnode-1 | 6 passed; starts on CPUs 12-17, moves to CPU 0; all 12 VmSize points equal their bounds (largest +45,640 KiB over start) | new |
| .debs | mei debs-1 | `DEBS bad=0`; check PASS 7 packages; check-symbols PASS 4440 lines, 13 libraries | same |
| plants | mei plants-1 | 6 of 6 CAUGHT by their messages; no WARN | same |
| package diff | keke pkgdiff-1 | libc-bin +`usr/bin/localedef` (24 -> 25 files; 335,612 -> 476,532 bytes, Installed-Size 1763 -> 2189); libc6-dbg + its debug file (51 -> 52); other file lists equal | expected (UTIL-12) |
| stage, install | keke stage-1, install-1 | staged below; mmdebstrap rc 0, system debs different: 0, `inst: RESULT fails=0` | same |
| trixie root, swapped | mei roots-1 | rc 1 on the known lines; masked lines differ only in version and path | same |
| lint | mei lint-1 | errors 10, warnings 48 -> 50: `libc-bin: no-manual-page [usr/bin/localedef]`, `libc-bin: shared-library-lacks-prerequisites [usr/bin/localedef]` | finding P2 |
| yohane, many, dyn22, y10, y19, y26, y26t, c19, y23, y25, y24, y28, y6, y25s, yres | mei (-1 each) | yohane rc 1 on the known lines | 0 differ each |
| y23p, dlc, y27, y13, y14, y6c, y14r, eetwet, sig32, y20, dpo600 | mei (-1 each) | y14 / y14r rc 1 on the known lines | same last verdict each |
| valgrind (#13) | mei y13s-1, y13v-1 (+ -ref222-1 each) | 13S fail=0; VGACC 8 of 8, both on head and 22.2 | same |
| DL-RUNPATH, CJK iconv | mei drp-1, y6r-1 (+ -ref222-1 each) | drp fail=2 both (setgid `$ORIGIN` rows, known); y6r 13/0 both | same |
| getaddrinfo_a | keke gaimass-1 | `GAIMASS bad=0` | same |
| malloc replacement | mei mint-1 | `MINT fail=0 internal_rows_differing_from_stock=40 changed_from_previous=0` | same |
| #21 probe and find | mei y21-1, y21-ref222-1 | pass 7 fail 0 both | equal within one 16 KiB bisect step |
| #14 localedef | mei y14l-2 | pass 12 fail 0 | new (22.2's libc-bin has no localedef) |

## Start-up (execcount, ASLR off, median of 101; startup/kstart/rstart/startup215 on both classes, all rc 0)

**Headline: LD-PHDR-ONCE-2.** User instructions per dynamic start are -18.1% to -23.2% against 22.2 on both
classes; page faults are unchanged on every row. 22.2's and glibc's rows were measured again in this run and equal
1122B's record exactly on both classes.

| program | class | glibc 2.39 | 21.5 | 22.2 | 2a76723f2 | change from 22.2 | faults vs 21.5 |
|---|---|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 741,532 / 51 | 320,731 / 51 | **260,857 / 51** | **-59,874 (-18.67%)** / 0 | 0 |
| empty main | Zen 1 | 101,533 / 48 | 741,300 / 51 | 320,492 / 51 | **260,618 / 51** | **-59,874 (-18.68%)** / 0 | 0 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 984,518 / 65 | 483,296 / 66 | **371,146 / 66** | **-112,150 (-23.20%)** / 0 | +1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 984,271 / 65 | 483,042 / 66 | **370,892 / 66** | **-112,150 (-23.22%)** / 0 | +1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 763,624 / 54 | 335,064 / 54 | **274,330 / 54** | **-60,734 (-18.13%)** / 0 | 0 |
| clock_gettime | Zen 1 | 103,672 / 50 | 763,420 / 55 | 334,854 / 55 | **274,120 / 55** | **-60,734 (-18.14%)** / 0 | 0 |

21.5's instructions are RELEASE-SNAPSHOT-1121D's record; its faults were measured again (startup215, same as 1122B).

Kernel instructions / syscalls (kstart; kernel counts vary by up to ~50k between runs, syscalls are exact):

| program | class | glibc 2.39 | 22.2 | 2a76723f2 | change from 22.2 |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 688,994 / 46 | 717,709 / 29 | 737,452 / 29 | +19,743 / 0 |
| empty main | Zen 1 | 703,662 / 42 | 735,936 / 28 | 755,112 / 28 | +19,176 / 0 |
| 3 libraries | Skylake-SP | 982,410 / 72 | 1,117,047 / 59 | 1,103,432 / 59 | -13,615 / 0 |
| 3 libraries | Zen 1 | 1,002,946 / 68 | 1,148,032 / 58 | 1,136,427 / 58 | -11,605 / 0 |
| clock_gettime | Skylake-SP | 692,265 / 46 | 746,983 / 29 | 764,353 / 29 | +17,370 / 0 |
| clock_gettime | Zen 1 | 713,475 / 42 | 771,569 / 28 | 789,661 / 28 | +18,092 / 0 |

libresolv pair (rstart; res0 needs libresolv.so.2, pie0 empty):

| class | row | glibc 2.39 | 22.2 | 2a76723f2 | change from 22.2 |
|---|---|---|---|---|---|
| Skylake-SP | res0 user / faults | 169,020 / 55 | 448,781 / 56 | 374,316 / 56 | -74,465 / 0 |
| Skylake-SP | res0 kernel / syscalls | 834,457 / 58 | 917,752 / 42 | 906,378 / 42 | -11,374 / 0 |
| Skylake-SP | res0 - pie0 user | 61,375 | 128,082 | 113,491 | -14,591 |
| Zen 1 | res0 user / faults | 162,716 / 54 | 448,229 / 56 | 373,764 / 56 | -74,465 / 0 |
| Zen 1 | res0 kernel / syscalls | 844,965 / 54 | 925,331 / 40 | 920,556 / 40 | -4,775 / 0 |
| Zen 1 | res0 - pie0 user | 61,235 | 127,819 | 113,228 | -14,591 |

## #21 (y21, swapped trixie root; head beside 22.2 and stock)

Issue's probe, least headroom over start: stock 29,568 KiB; 22.2 40,816; head 40,816. Under start + 35000 KiB:
stock rc 0, 22.2 and head rc 11 (the probe's own failure status, unchanged); under 41296 all rc 0. find: need for 3
files stock 4,640 / 22.2 21,632 / head 21,648 KiB; for 200,000 files 31,616 / 49,296 / 49,296; the issue's formula
(`ulimit -v` = need3 + 35000): 5 of 5 randomised runs succeed for stock, 22.2 and head. The head equals 22.2 within
one 16 KiB bisect step: HEAP-SPILL-COLD-3 bounds a mid-run NUMA node change (heapnode-1 above), which this probe does
not make.

## #14 (y14l, new: swapped trixie root; stock = trixie's own localedef, glibc 2.41, with trixie's locales package)

libniji's libc6, libc-bin, libc6-dev, libc-dev-bin, locales and libc-l10n were swapped in and held, so the root holds
only libniji's locale files: `/usr/share/i18n/locales` has no en_US and `/usr/share/i18n/charmaps` has no UTF-8 (only
libc6's single-byte charmaps for iconv). Scripts: mei `~/work/RELEASE-SNAPSHOT-1122C/y14l.sh` and `y14l/`.

* **(a) bash's exact command** `localedef -i en_US -c -f UTF-8 $LOCPATH/en_US.UTF-8`: libniji exit 4,
  `[error] cannot open locale definition file `en_US': No such file or directory`; no locale file written; an empty
  `en_US.UTF-8` directory is left, as stock leaves one for a missing source. It needs **both** a source and a charmap:
  with only the charmap supplied the same source error; with only the source supplied `[error] character map file
  `UTF-8' not found: No such file or directory`, `[error] default character map file `ANSI_X3.4-1968' not found`,
  `[error] no output file produced because errors were issued`, exit 4. Stock: exit 0, twelve categories (its `%c`
  reads "Sun 22 Nov 2026 02:05:09 PM UTC").
* **(b) the test's own files** (made by the test, never a distribution file): a UTF-8 charmap of POSIX's portable
  character set (XBD 6.1) and an en_US definition of the test's own values (two-level LC_COLLATE, US dates, all twelve
  categories, characters named `<Uxxxx>`). libniji's localedef exit 0 (stock 0); `setlocale`, `nl_langinfo`,
  `localeconv`, `strftime` (14 formats), `strcoll` (15 pairs), ctype and `mbrtowc` read back on libniji **equal to
  stock's on all 37 lines**. libniji also reads the reference's own en_US.UTF-8 (stock localedef, stock sources) and
  stock's build of the test's files equal to stock on every line.
* **(c)** `--help` exit 0 (45 lines; stock 47; the same default paths), `--version` exit 0 (`localedef (libniji)
  2.39`); a missing source, a missing charmap, no operand and an unknown option exit 4 each, as stock, with the same
  messages for the missing source and the unknown option.

## Staged (keke `~/release-staging/snapshot-20261122.3/`; MANIFEST + SHA256SUMS)

libc6, libc6-dev, libc6-dbg, libc-bin, libc-dev-bin (amd64), libc-l10n, locales (all) `2.39+niji20261122.3-1`;
niji6, niji6-dev `0.0.snapshot20261122.3+2a76723f2ed4` (asset names use '.' for '~'). libc6 sha256
d1c340260fa775f3a267f159d1512129808fc5182eed7201c1f6846378a3297f.

## Findings

- **F1 (headline):** LD-PHDR-ONCE-2: -59,874 to -112,150 user instructions per dynamic start (-18.1% to -23.2%) on
  both classes, -14,591 for loading libresolv; no fault or syscall changes.
- **L1 (RULING NEEDED, localedef):** a source that defines only the six POSIX categories compiles (exit 1, warnings,
  as stock) but libniji writes only those six category files; `setlocale(LC_ALL, ...)` on the result returns NULL on
  libniji. Stock writes all twelve and the result loads. Recommend a story: write the six other categories with
  their defaults when the source omits them.
- **L2 (RULING NEEDED, localedef):** charmaps and sources that use POSIX symbolic names (`<a>`, `<space>`, XBD 6.4 /
  7.3) are refused: a charmap without `<Uxxxx>` names gives `the charmap has no mappings`; a source over POSIX names
  gives `unknown symbolic name` even when the charmap carries `<Uxxxx>` after the bytes. Stock accepts both (exit 1,
  warnings). Recommend a story: resolve charmap symbolic names in sources (XBD 7.3).
- **L3 (RULING NEEDED, spec gap):** localedef(1) documents a search of the current directory and of `I18NPATH` for
  `-i` / `-f` names; libniji searches only the default directories (exit 4); stock finds the files in both places.
  Neither is in the spec. Recommend: add the clause, then implement it with L1/L2, before LOCALE-SOURCES-GEN ships
  sources that users may want to override.
- **P2 (INFORMATIONAL):** two new lintian warnings on `usr/bin/localedef` (no-manual-page; shared-library-lacks-
  prerequisites, the static PIE read as a shared library).
- **K3 (INFORMATIONAL, kernel):** kernel instructions for empty main and clock_gettime are +17k to +20k on both
  classes (3 libraries -12k to -14k); inside the ~50k run-to-run band, syscalls and faults equal. Shown only.
- **M (INFORMATIONAL, pre-existing):** mint unchanged (40 internal rows differ from stock, none changed).

SUPERSEDED: mei y14l-1 (two FAIL lines that came from the test, not from libniji. (1) It expected no output
directory, but stock also leaves an empty one for a missing source. (2) Its b1 source had only the six POSIX
categories, which is finding L1. It was rerun as y14l-2 with the six locale(5) categories added and the six-category
case kept as a shown line).
Other rc 1 jobs end on their known lines (roots, yohane, y14, y14r, drp, drp-ref222), as in 1122B.

## Next step

Coordinator: announce snapshot-2026-11-22.3 with the start-up table above and #14's state (localedef works from
user-supplied sources with `<Uxxxx>` names; no locale sources or charmaps shipped yet, LOCALE-CHARMAPS-GEN /
LOCALE-SOURCES-GEN); rule on L1-L3.
