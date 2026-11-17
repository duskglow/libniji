COMPLETE (validation): 20dbed16c is validated; the tag waits on one RULING NEEDED (the system flavour's libc6-dev has no C headers)

# RELEASE-SNAPSHOT-LDCONFIG(-2): snapshot validation of 20dbed16c (ldconfig #16, ldd trace output)

**State.** Head to tag `20dbed16c07ef65b3a83c8045662774c50c4270c` "Regenerate train-owned views" (origin/next at the
start of -1, 2026-11-17 11:44 UTC; commit time 1794911006). `git log --oneline e40107432..20dbed16c`: 45 lines; merges:
rwlock-try-ebusy, ldconfig-dev-alias (#16), SYS-ABI-DT-RELR(-4: GLIBC_ABI_DT_RELR and the other symbol-less version
nodes), util-7 (iconv in libc-bin), dl-trace-interp-2 (ldd / ld.so --list output; system build pins
`lib/x86_64-linux-gnu`), release-snapshot-mvec's handoff. Branch `task/release-snapshot-ldconfig-2` = that head +
handoff/ + one test-script fix (5383e0ba7, below). Role mei `~/w/s2`. Job scripts: mei and keke
`~/work/RELEASE-SNAPSHOT-LDCONFIG/` (MVEC's with head, epoch, serial 3, role and labels changed; yohane.sh + y/ for #16).
No product or packaging code changed.

## Results (all on 20dbed16c's products; the .debs are byte-identical on mei and keke, 9 sha256 compared)

| check | job (host) | result | vs snapshot-20261117.2 |
|---|---|---|---|
| system + coexist builds, check-system, check-symbols, coexist check | release-snapshot-ldconfig-debs-1 (mei) rc 1 | `check: PASS (7 packages, 2.39+niji20261117.3-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)`; coexist `check: PASS (2 packages)`; rc 1 = DBG only: libc6-dbg has no debug file for `/usr/bin/iconv` (17 of 18) | +3 symbol lines (GLIBC_ABI_DT_RELR nodes); DBG gap new (UTIL-7) |
| gate (clippy --all-features, xtask clippy, static, records, affected) | release-snapshot-ldconfig-2-gate-1 (mei) rc 0 | all steps RC 0 | same |
| lua_reload, cxx_dlclose | release-snapshot-ldconfig-smoke-mei-1, -smoke-keke-1 rc 0 | ok 1/1 both sides, both hosts | same |
| package comparison | release-snapshot-ldconfig-pkgdiff-1 (keke) rc 0 | libc-bin +`usr/bin/iconv` (20 -> 21 files); libc6-dbg build-ids changed (43 -> 44 entries); others: versions only | as expected |
| empty root | release-snapshot-ldconfig-2-root-2 (mei) rc 0 | `root: RESULT pass=19 fail=0` | same (roots-1 had 18/1: stale check, fixed) |
| swapped trixie root | release-snapshot-ldconfig-roots-1 (mei) | `verdict: commands=65 same=63 known=2 bad=0`; ldd-1..5 identical to stock | 56/7 known -> 63/2: ldd-6/-7 differ only as D-31.15 documents |
| libc6-dbg + gdb (trixie, excalibur) | roots-1 | `dbg: RESULT pass=5 fail=0 pending=2` each | same |
| fresh install | roots-1 | `fresh: RESULT fail=0`; 17 objects have their .debug | same |
| lintian trixie / excalibur | release-snapshot-ldconfig-lint-1 (mei) rc 0 | 10 E / 59 W each; coexist proofs pass=21 fail=0 each | +2 W, below |
| #16 end to end | release-snapshot-ldconfig-yohane-1 (mei) rc 1 | 16 PASS / 2 FAIL: the FAILs are the header gap (RULING NEEDED) | new check |
| staging | release-snapshot-ldconfig-2-stage-1 (keke) rc 0 | keke `~/release-staging/snapshot-20261117.3/`: 9 .debs, SHA256SUMS (all OK), MANIFEST | serial 3 |
| install instructions (coexist, staged assets) | release-snapshot-ldconfig-2-install-1 (keke) rc 0 | `inst: RESULT fails=0` (24 ok) | same |
| plants (handoff/RELEASE-SNAPSHOT-LDCONFIG-plants.txt) | release-snapshot-ldconfig-2-plants-1 (mei) rc 0 | `6 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG`; the root-system.sh baselines pass with the fix | same as NIJI1-2 |

Superseded: roots-1 (root 18/1 from the stale `/usr/lib` check, fixed by 5383e0ba7 and rerun as root-2);
release-snapshot-ldconfig-2-root-1 (rc 128: my job chained `git log` in the role, which is not a git tree; the root never ran).

**Swap ldd-6 / ldd-7 (`ldd -v /bin/sh`, `ldd -v python3`):** stock prints a `/lib/x86_64-linux-gnu/libc.so.6:` block
(ld-linux-x86-64.so.2 GLIBC_2.2.5, 2.3, 2.35, PRIVATE) and libm.so.6's block lists ld.so GLIBC_PRIVATE and libc.so.6
GLIBC_2.36, ABI_DT_RELR, 2.4, 2.2.5, PRIVATE; libniji prints no libc.so.6 block and libm.so.6 lists libc.so.6
GLIBC_2.14, GLIBC_2.2.5. Every other line is identical. This is the documented difference D-31.15
(differences-from-glibc.md:534): what the installed objects record in DT_VERNEED. Not a finding; one wording nit below.

**Lintian +2 W per suite (E set identical):** `libc-bin: extended-description-line-too-long line 2` and
`libc-bin: no-manual-page [usr/bin/iconv]`, both from UTIL-7's iconv in libc-bin. The libc6-dbg
`debug-file-with-no-debug-symbols` tag is the same tag on a new build-id.

**#16 (Yohane, libcmocka 1.1.7-3+b1 from trixie, swapped root):** `dpkg -i libcmocka0 libcmocka-dev` rc 0, the libc-bin
trigger ran; after it and after `ldconfig -v`: `libcmocka.so.0 -> libcmocka.so.0.8.0` (the real file),
`readlink -f libcmockery.so` = `/usr/lib/x86_64-linux-gnu/libcmocka.so.0.8.0` (no loop), 0 "(changed)" lines;
`ldconfig -p` lists `libcmocka.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libcmocka.so.0`; a stock-compiled object
links with `-lcmocka` in the root and runs (`[  PASSED  ] 1 test(s).`). Minimal reproducer `ldconfig -n`, both orders
(libcmocka.so.0 present / absent): libniji's output and resulting links are identical to stock ldconfig 2.41-12+deb13u4
(both print `libcmocka.so.0 -> libcmockery.so` under -v with the link left on the real file; absent: link created to
the real file). The 2 FAILs: `gcc cm.c -lcmocka` in the root and so `cm-full` rc 127.

**Cause of the 2 FAILs (settled; the script is not at fault):** `/usr/lib/gcc/x86_64-linux-gnu/14/include/stdint.h:9:
# include_next <stdint.h>` finds no `/usr/include/stdint.h`. libniji's libc6-dev (system flavour) has no
`/usr/include` entries at all (38 entries; same in snapshot-20261117.2: not a regression). Debian's libc6-dev owns
`/usr/include/stdint.h` (and every glibc header); `dpkg -i` of libniji's libc6-dev replaces that package, so dpkg
removes them. After a system-flavour install no C program that includes a libc header compiles. The coexist flavour is
unaffected (niji6-dev depends on the distribution's libc6-dev). install-packages.md:124 says "No headers yet: plain
`gcc` compiles with the system's glibc headers", which does not hold for the system flavour.

## Findings

* RULING NEEDED (blocks the tag by the 11-17 note): system-flavour libc6-dev ships no headers, so a swapped/system
  install removes `/usr/include/stdint.h` and every other libc header. Recommendation: tag 20dbed16c with this written as
  a known limitation of the system flavour ("compile elsewhere or use the coexist flavour"), since it is not a
  regression and only the yohane check exercises it; and cut a packaging/headers story (SL-HEADERS area) so libc6-dev
  carries a header set. If the owner wants system-flavour compiles to work in this snapshot, it needs that story first.
* libc6-dbg lacks iconv's debug file (tools/deb/system.sh:134 list; fix is SYS-DBG-DWARF's, ruling #6321: tag anyway).
* Docs (INFORMATIONAL, for a docs story; coordinator-owned files listed, not edited): install-packages.md:123 (libc-bin
  row) does not list `/usr/bin/iconv`; :124 header sentence (above); :126 says libc6-dbg covers every program of
  libc-bin (not iconv). RELEASE-NOTES.md has no section after 2026-11-16, and :110 still says "`ldd` is next, then
  `iconv`". differences-from-glibc.md:534 (D-31.15) says libm.so.6 lists "only ... `GLIBC_2.14` of `libc.so.6`"; it
  lists GLIBC_2.14 and GLIBC_2.2.5. doccheck.py: 238 links, 197 xtask mentions, 21 problems, the same set as .2.
* User-visible changes since snapshot-20261117.2: ldconfig keeps a soname link on the real file when a `.so` alias
  matches (#16); `ldd` / `ld.so --list` print the reference's paths and interpreter line; GLIBC_ABI_DT_RELR and the
  other symbol-less version nodes exported; `/usr/bin/iconv` in libc-bin; pthread_rwlock_try* EBUSY.

## Exact next step

Coordinator: rule on the header gap; then tag 20dbed16c from keke `~/release-staging/snapshot-20261117.3/` (or re-cut
after the headers story). Nothing else is left in this story.
