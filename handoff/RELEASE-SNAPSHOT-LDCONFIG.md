INCOMPLETE: checkpoint at ~150k context; validation of 20dbed16c half done (jobs below), nothing staged yet

# RELEASE-SNAPSHOT-LDCONFIG: snapshot validation of 20dbed16c (ldconfig #16, ldd trace output)

**State.** Head to tag `20dbed16c07ef65b3a83c8045662774c50c4270c` "Regenerate train-owned views" (origin/next at the
start, 2026-11-17 11:44 UTC; commit time 1794911006). `git log --oneline e40107432..20dbed16c`: 45 lines; merges:
rwlock-try-ebusy, ldconfig-dev-alias (#16), SYS-ABI-DT-RELR(-4: GLIBC_ABI_DT_RELR and the other symbol-less version
nodes), util-7 (iconv in libc-bin), dl-trace-interp-2 (ldd / ld.so --list output; system build pins
`lib/x86_64-linux-gnu`), release-snapshot-mvec's handoff. Branch = that head + handoff/ only. Role mei `~/w/s2`.
Scripts: mei and keke `~/work/RELEASE-SNAPSHOT-LDCONFIG/` (MVEC's with head 20dbed16c, epoch 1794911006, serial 3,
role s2, labels `release-snapshot-ldconfig-*`); changed beyond that: chain-role/chain-debs (the role is on mei: mei
builds and runs roots/lint/yohane, keke runs its smoke round on `tree/` + mei's products, pkgdiff, stage, install);
debs.sh carries debscheck-2's DBG check (.gnu_debuglink reported, not required); swapverdict.sh: this head's 65
swap commands (+relr-locale, relr-zdump) and the raw diff printed for any KNOWN ldd case; new yohane.sh + y/
(pre.sh, post.sh, rep.sh, cm.c) for #16 in place of mvec.sh. `tree/` = local `git archive` dir, rsync'd.

## Results so far (all on 20dbed16c's products)

| check | job (host) | result |
|---|---|---|
| system + coexist builds, check-system, check-symbols, coexist check | debs-1 (mei) rc 1 | builds rc 0; `check: PASS (7 packages, version 2.39+niji20261117.3-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)` (+3 vs 4435); `check: PASS (2 packages)`. rc 1 only from DBG: **finding** below |
| gate | gate-1 (mei) rc 0 | clippy --all-features, xtask clippy, static, records check, affected (0 suites) |
| lua_reload, cxx_dlclose | smoke-mei-1 (mei) rc 0, smoke-keke-1 (keke) rc 0 | ok 1/1 both sides on both hosts; libc.so.6 sha256 753a0526... |
| package comparison | pkgdiff-1 (keke) rc 0 | not yet read |
| #16 end to end | yohane-1 (mei) rc 1 | 16 PASS, 2 FAIL: the FAILs are a full `gcc cm.c -lcmocka` IN the swapped root (`# include_next <stdint.h>` not found: compilation terminated) and so cm-full rc 127; not yet read in detail, nor the #16 lines (cm-obj, compiled on the stock root, link/run line, ldconfig -v, readlink, rep.sh both orders) |
| roots, lint | roots-1, lint-1 (mei) | running at the checkpoint |

**Finding (packaging, not blocking in my view):** libc6-dbg has no debug file for `/usr/bin/iconv` (DBG: 17 of 18 ELF
objects resolve; `libc-bin/usr/bin/iconv build-id ca4f2cd8...: debug file build-id ''`). Cause: tools/deb/system.sh:134,
the list of objects whose debug file goes to libc6-dbg, names ldconfig, getconf, getent, ldd but not iconv (UTIL-7
added iconv to the strip list at :150 only). docs/install-packages.md:126 says libc6-dbg covers "every program of
libc-bin". Fix: add `"$bin/usr/bin/iconv"` to :134 (packaging story).

**Docs (INFORMATIONAL so far):** doccheck.py (RELEASE-SNAPSHOT-CXX-DOCS's): 238 links, 197 xtask mentions, 21
problems, the identical set to snapshot-20261117.2 (none new). install-packages.md:123 (libc-bin row) does not list
`/usr/bin/iconv` (dynamic). RELEASE-NOTES.md has no section after 2026-11-16 (coordinator); its :110 still says
"`ldd` is next, then `iconv`". differences-from-glibc.md gained the `ldd -v` row (D-31.15): check it against the
swap's ldd cases once roots-1 is read.

## Exact next step

1. Read mei `~/logs/release-snapshot-ldconfig-roots-1.log` (`verdict:` lines: expect commands=65 known=0; any KNOWN
   line prints the raw difference = a finding; root/dbg/fresh RESULT lines) and `-lint-1.log` (`lint: RESULT` per
   suite vs 10 E / 57 W; expect new tags for libc-bin's iconv), keke `-pkgdiff-1.log`.
2. yohane-1: read `~/work/RELEASE-SNAPSHOT-LDCONFIG/logs-yohane/out/post.txt` and `pre.txt` (#16 evidence); find why
   gcc in the swapped root misses stdint.h (libniji's libc6-dev headers vs gcc's include-fixed stdint.h: a finding
   unless the script is at fault).
3. Plants: sync the branch (handoff/RELEASE-SNAPSHOT-LDCONFIG-plants.txt), run `cargo xtask plants` in mei ~/w/s2,
   `--dir /mnt/zram/claude/RELEASE-SNAPSHOT-LDCONFIG/plants-1`; label release-snapshot-ldconfig-plants-1.
4. Stage on keke (stage.sh -> `~/release-staging/snapshot-20261117.3/`, debs already in keke ~/work/.../debs, same
   sha256 as mei's), then install.sh (4(b)). Then the report (first line, table vs 20261117.2, #16 output, assets).

RULING NEEDED (proposed): tag 20dbed16c with the libc6-dbg iconv gap noted, or wait for the one-line fix.
Recommendation: wait only if the coordinator re-cuts anyway; the gap affects debugging of iconv only.
