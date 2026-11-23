COMPLETE: snapshot-ready at b732d9b46 (#23 partly fixed: the issue's input matches stock; an invalid sequence cut short
before an ASCII byte still loses bytes in `col`, finding 1)

# RELEASE-SNAPSHOT-1120C: snapshot validation of b732d9b46 (#23 byte reads on wide streams, COMPAT19-TUNABLE)

**State.** Validated head `b732d9b46a41300dae6eea528aad20c5be5cebec` "Regenerate train-owned views" (origin/next at the
start, 2026-11-20 15:59 UTC; commit time 15:00:30 UTC, epoch 1795186830). `git merge-base --is-ancestor` holds for
b732d9b46 (it is the head; STDIO-WIDE-BYTE-READS-2 merge ffc9dd44a) and for 15ead43a2 (COMPAT19-TUNABLE).
THREAD-FORCED-UNWIND-3 (#25) has **not** landed and is not included. Branch `task/release-snapshot-1120c` = that head
+ handoff/ only (no product or packaging code changed). Role mei `~/w/s3` (Skylake-SP); keke = Zen 1. One leg.

**Scripts.** mei and keke `~/work/RELEASE-SNAPSHOT-1120C/`, made on mei by `copy.sh` + `subst.sed` (both in that dir)
from mei `~/work/RELEASE-SNAPSHOT-1120B/` (the full set: *.sh, d/ m/ y/ y10/ y19/ y26/ y26t/ startup/, the
`binutils-src` link), then copied to keke. Changed: head, epoch, serial 20261120.3, role mei s3 (chain-role.sh builds
in s3 and runs `smoke-mei-1`; keke's smoke runs on the unpacked `tree/` with mei's products), labels, staging dir,
pkgdiff against snapshot-20261120.2, start-up sets p202 (20.2's products.tgz) and p203 (this head's), paths of equal
length. `tree/` = `git archive b732d9b46`. New this run (sequencing and two checks, y26.sh's recipe): `run-all.sh`
(chain-role then chain-debs), `chain-y.sh` (y10/y19/y26/y26t/y23/c19 after debs-1), `chain-keke-pre.sh` (keke smoke,
then start-up), `y23.sh` + `y23/` (#23), `c19.sh` + `c19/` (COMPAT19), `cmp.sh` (masked comparison with 1120B's logs).

**Landed since snapshot-2026-11-20.2** (`git log --oneline 3f2a84520..b732d9b46`: 96 lines; first-parent merges):
compat19-tunable, review-locale-slots-growth and review-heap-address-space (reviews), release-snapshot-1120b (handoff
only), ld-split-3, wl-dynamic-kit, headers-gen-s07-hooks, layout-got-order, heap-foreign-pair-2,
**stdio-wide-byte-reads-2** (GitHub #23), train view regenerations.

## Results against snapshot-2026-11-20.2 (all on b732d9b46's products)

| check | job | result | vs 20.2 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | mei -smoke-mei-1 (builds products in s3), keke -smoke-keke-1 (mei's products) | `SMOKE: RESULT bad=0` both | same |
| gate | mei -gate-1 rc 0 (on b732d9b46), -gate-2 (final head, same gate.sh) | clippy --all-features, xtask clippy, static, records check, affected --base origin/next: all RC 0 | same |
| system debs, check-system | mei -debs-1 | `check: PASS (7 packages, version 2.39+niji20261120.3-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines, 13 libraries | same (4440) |
| coexist debs + check | debs-1 | `check: PASS (2 packages)` | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects (no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale`; ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc6-dbg 44, libc6-dev 38, libc-bin 22, niji6 29, niji6-dev 48, others 7); libc6-dbg: 6 build-ids replaced; control fields: versions only; sizes within 37 KB | same |
| empty root | mei -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6, ldd-7) | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (`Breakpoint 1.1`) | same |
| fresh install + ldconfig | roots-1 | `fresh: RESULT fail=0`; 119 masked lines, only the work-dir path differs | same |
| lintian | mei -lint-1 rc 0 | errors=10 warnings=60, trixie and excalibur; 46 masked lines identical to 20.2's | same |
| coexist proofs | lint-1 | `proof: RESULT pass=21 fail=0` trixie and excalibur | same |
| #16 rerun | mei -yohane-1 rc 1 | `fail=2` (libc6-dev ships no C headers); 31 masked lines identical | same |
| many objects (#9) | mei -many-1 rc 0 | `pass=9 fail=0`; 18 masked lines identical | same |
| #22 | mei -dyn22-1 rc 0 | `dyn22: RESULT fail=0`; 57 masked lines identical | same |
| #10 + #18 | mei -y10-1 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen through `$ORIGIN` DT_RUNPATH); 34 masked lines identical | same |
| #19 | mei -y19-1 rc 0 | `pass=14 fail=0` | same |
| #26 checks | mei -y26-1 rc 0 | `YOHANE-26 pass=20 fail=0`; 22 masked lines identical | same |
| #26 gcc ASan sample | mei -y26t-1 rc 0 | judge: stock 20/20, niji 20/20, nijibuilt 20/20 | same |
| **#23 col** | mei -y23-1 rc 1; 20.2: -y23-ref202-1 rc 1 | 11 PASS / 2 FAIL (below) | 20.2: 9 PASS / 4 FAIL |
| **COMPAT19** | mei -c19-1 rc 0; 20.2: -c19-ref202-1 rc 1 | `COMPAT19 pass=16 fail=0` (below) | 20.2: tunable not honoured |
| plants | mei -plants-1 rc 0 | 6 plants: 6 CAUGHT, 0 SURVIVED, 0 WARN (handoff/RELEASE-SNAPSHOT-1120C-plants.txt) | same |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` | same |
| docs | local doccheck.py (1120B's) on the head | 23 problems, the same 23 lines as 20.2 | same |

## #23 end to end (swapped trixie root, util-linux 2.41.5 `col` from bsdextrautils, `LC_ALL=C.UTF-8`)

Inputs (y23/gen.sh): **issue** = the issue's `a\xfccd\n`; **valid** = well-formed UTF-8 with col's controls (a
backspace overstrike of a 2-byte character, a 3-byte character, a tab, ESC 7); **invalid** = six lines, one per kind:
lone continuation byte, overlong 2-byte form, 0xff, a 3-byte sequence cut short before an ASCII byte, a 2-byte lead
followed by an ASCII byte, and a sequence cut short at end of file. Each run plain and `col -b`; output bytes by
`cmp`, exit status and stderr compared with the stock run in the same root.

| input | stock glibc | snapshot-2026-11-20.2 | b732d9b46 |
|---|---|---|---|
| issue (5 bytes) | 8 bytes, 1 line (`a\xfccd`: col writes the invalid byte as the text `\xfc`), rc 0 | 2 bytes (`a` + newline: the line is cut at the invalid byte), rc 0 | **cmp-identical to stock**, rc 0 |
| valid | 39 bytes / 4 lines (`-b`: 37) , rc 0 | identical | identical |
| invalid | 44 bytes / 6 lines, rc 0 | 2 bytes / 1 line (cut after `p`) | 38 bytes / 6 lines, rc 0: lines 1-3 and 6 identical; line 4 (`\xe2\x82` then `t`) stock `\xe2\x82t`, libniji `\x74`; line 5 (`\xc3` then `(u`) stock `\xc3(u`, libniji `\x28u` |

stderr empty and exit status 0 in every case on every library. Plain and `-b` give the same verdicts.

## COMPAT19-TUNABLE (swapped trixie root, gcc 14.2.0-19, binutils 2.44; probes built on the stock root)

Probes: `plain` (no request), `es` (`-Wl,-z,execstack`), `nested` (a GCC nested function taken by address: ld warns
"requires executable stack", GNU_STACK RWE), `dl` (no request) dlopening `libes.so` (`-z execstack`), and `es` run as
`ld.so ./es`. Each with GLIBC_TUNABLES unset, `niji.compat.execstack=0` and `=1`.

| probe | unset / `=0` (b732d9b46) | `=1` (b732d9b46) | 20.2, any setting |
|---|---|---|---|
| plain | runs, stack `rw-p` | runs, stack `rw-p` | runs |
| es, `ld.so es` | refused, rc 127: `ld.so: <path>: the program asks for an executable stack (PT_GNU_STACK with PF_X); refused; to accept it, start the process with GLIBC_TUNABLES=niji.compat.execstack=1 (COMPAT-19; not honoured in a secure process); a system allowlist (/etc/niji/execstack.d) is planned` | runs, stack `rwxp` | refused, message without the object or the ways in |
| nested | refused (same message) | runs, the nested function returns 7, stack `rwxp` | refused |
| dl + libes.so | main runs, `dlopen` fails: `<lib>: the object asks for an executable stack ...; refused (dlopen called from <prog>); to accept it, ...` | `dlopen` ok, `es_fn` = 42, stack `rwxp` | `dlopen` fails with `ELF load command address/offset not page-aligned` |

Documented behaviour (docs/tunables.md, RELEASE-NOTES "Unreleased", D-27.66) holds. The stock trixie root (its own
C library, recorded only) runs `es`/`nested` with `rwxp` stacks under every setting and refuses the `dlopen`.

## Start-up (execcount: user instructions / page faults, exec to exit, ASLR off, median of 101)

| program | class | glibc 2.39 | snapshot-2026-11-20.2 | b732d9b46 | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 786,103 / 57 | 788,635 / 57 | +2,532 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 1,037,723 / 73 | 1,044,146 / 73 | +6,423 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 820,043 / 61 | 822,638 / 61 | +2,595 |
| empty main | Zen 1 | 101,533 / 48 | 785,876 / 57 | 788,409 / 57 | +2,533 |
| 3 libraries | Zen 1 | 136,734 / 64 | 1,037,481 / 73 | 1,043,911 / 73 | +6,430 |
| clock_gettime | Zen 1 | 103,672 / 50 | 819,845 / 62 | 822,436 / 62 | +2,591 |

Product paths of equal length in every compared tree (jobs -startup-mei-1, -startup-keke-1). 20.2's column is
remeasured in this run (1120B's own numbers were ~180 lower on Skylake-SP, glibc's too: environment, not products).

## Findings

1. **#23 is fixed for the issue's input, not for every invalid sequence (RULING NEEDED: route a follow-up).** When a
   multibyte sequence is cut short by a byte that cannot continue it (`\xe2\x82` then `t`; `\xc3` then `(`), `col`
   under libniji loses the bytes of the cut-short sequence and treats the breaking ASCII byte as the invalid one: after
   `getwchar`'s EILSEQ, `getchar` returns the byte that ended the sequence instead of its first byte. D-12.27's own rule
   (and its differences row) says a byte read gets "the byte after the last wide character delivered", i.e. the
   sequence's first byte, so this is a libniji defect against its record, not a design difference. Data are lost
   silently (exit 0). The snapshot is still strictly better than 20.2 (which cut every line at the first invalid
   byte). Recommendation: tell Yohane #23's reported case is fixed and a cut-short sequence is a known limitation;
   STDIO-WIDE-BYTE-READS-3 with the two lines above as a conformance case. Not blocking (no regression vs 20.2), the
   coordinator's call.
2. **Start-up +2.5k instructions per program, +6.4k with 3 libraries** (+0.3% / +0.6%), page faults unchanged, both
   classes; the per-library part (~1.3k per library) points to the loader's per-object path (COMPAT19's
   dependency check moved into `open_candidate`, or LD-SPLIT-3). Not traced. INFORMATIONAL.
3. **Docs (route to a docs story; install-packages.md passes against the staged assets, so not blocking).** User docs
   changed since 20.2: docs/RELEASE-NOTES.md (new "Unreleased" section: COMPAT-19 only), docs/tunables.md (the
   `niji.compat.execstack` row), docs/differences-from-glibc.md (the executable-stack row; the byte-operations row
   replaces the `fputc` row), docs/compat-flags.md (COMPAT-19 row). RELEASE-NOTES has **no entry for #23**; its
   section is "Unreleased (on next, for the next snapshot)", not dated for this snapshot. doccheck: the same 23 lines.
   **User-visible changes since snapshot-2026-11-20.2:** (a) "byte reads (`getc`, `fread`, `fgets`, `getdelim`),
   `ungetc` and `fputc` on a wide-oriented stream work on its bytes in order, and after an invalid sequence `getchar`
   returns the invalid byte, so util-linux `col` passes invalid bytes through (STDIO-WIDE-BYTE-READS-2, D-12.27,
   GitHub #23; limitation: finding 1)"; (b) "`GLIBC_TUNABLES=niji.compat.execstack=1` accepts programs, start-up
   dependencies and `dlopen`ed libraries that ask for an executable stack and makes the stacks executable; refused by
   default, now with a message naming the object, the program or `dlopen` caller and the ways in (COMPAT19-TUNABLE,
   COMPAT-19, D-27.66)". Start-up cost: finding 2.
4. Unchanged from 20.2: dbg `Breakpoint 1.1`; bare-name dlopen through `$ORIGIN` DT_RUNPATH (y10); libc6-dev ships no
   C headers (yohane fail=2); DBG 17 of 19 (ruling #6321). 1120B's finding 1 (same-day coexist versions ordered by
   commit hex) still stands; this pair happens to sort right (`b732d9b46a41` > `3f2a84520118`).

## Staged assets (keke `~/release-staging/snapshot-20261120.3/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261120.3-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261120.3-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261120+b732d9b46a41_amd64.deb`), SHA256SUMS,
MANIFEST. `sha256sum -c SHA256SUMS`: 9 OK. Nothing to GitHub, no tag.

## Jobs

Named (prove the final head): mei gate-2, plants-1, smoke-mei-1, startup-mei-1, lint-1, many-1, dyn22-1, y19-1,
y26-1, y26t-1, c19-1; keke smoke-keke-1, startup-keke-1, pkgdiff-1, stage-1, install-1. Evidence only, not named:
debs-1, roots-1, yohane-1, y10-1 (rc 1 on their known, unchanged failures, rows above); y23-1 (rc 1: finding 1);
y23-ref202-1 and c19-ref202-1 (20.2's packages, the "before" side, rc 1 by design). gate-1 superseded by gate-2 (same
gate.sh; the final head adds handoff/ only).

## Exact next step

Coordinator: decide finding 1 (recommend: tag snapshot-2026-11-20.3 from the staged directory at b732d9b46 when the
owner says so, tell Yohane #23's case is fixed with the cut-short-sequence limitation, open STDIO-WIDE-BYTE-READS-3);
route finding 3 to a docs story. Not to redo: anything above. Plants: handoff/RELEASE-SNAPSHOT-1120C-plants.txt.
Report: handoff/reports/RELEASE-SNAPSHOT-1120C.json.
