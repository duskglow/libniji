COMPLETE: snapshot-ready at 923df67e9 (GitHub #9 verified end to end; staged as snapshot-20261118.1, not tagged)

# RELEASE-SNAPSHOT-1118: snapshot validation of 923df67e9 (GitHub #9: more than 64 objects)

**State.** Head to tag `923df67e9350163af92787ac25e5f8d6c85d2429` "Merge task/headers-abi-regen into next"
(origin/next at the start, 2026-11-18 06:43 UTC; commit time 1794966029). Branch `task/release-snapshot-1118` = that
head + handoff/ only (plants file, this note, report json). Role mei `~/w/s3`. Job scripts: mei and keke
`~/work/RELEASE-SNAPSHOT-1118/` (RELEASE-SNAPSHOT-LDCONFIG's, with head, epoch, serial 1, role and labels changed;
fixed: inside.sh's coexist version uses a 12-character sha; new: many.sh + m/ for #9, chain-keke.sh). No product or
packaging code changed. Results name CPU classes outside handoff/; here: mei = Skylake-SP role, keke = Zen 1.

**What landed since snapshot-2026-11-17.3** (`git log --oneline 20dbed16c..923df67e9`: 131 lines, 19 merges):
dl-startup-objects (#9), dl-dlopen-new-objects (#9 via dlopen), sock-null-optval (#17), sys-iconv-coverage-2 (#6),
util-8-4 (the `locale` command), ldconfig-limits (input caps removed), sem-named-growth, sigmask-default-2,
sl-headers-s06-2 and sl-headers-s14 (generated headers; not packaged yet), headers-abi-regen, audit-hardcoded-limits-2,
scrub-reference-crashes (record wording), docs-snapshot-niji1, trace-time-pin, trace-numa-pin,
xtask-affected-tool-tests (tooling), release-snapshot-ldconfig-2 (handoff).

## Results (all on 923df67e9's products; the .debs relayed mei -> keke with sha256 checked, 9 of 9)

| check | job (host) | result | vs snapshot-2026-11-17.3 |
|---|---|---|---|
| system + coexist builds, check-system, check-symbols, coexist check | release-snapshot-1118-debs-1 (mei) rc 1 | `check: PASS (7 packages, version 2.39+niji20261118.1-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)`; coexist `check: PASS (2 packages)`; rc 1 = DBG only: 17 of 19 ELF objects, no debug file for `/usr/bin/iconv` and `/usr/bin/locale` | symbols same; DBG +1 gap (`locale`, new in UTIL-8; same cause as iconv) |
| gate (clippy --all-features, xtask clippy, static, records, affected) | release-snapshot-1118-gate-1 (mei) rc 0 | all steps RC 0, peak 2.3G | same |
| lua_reload, cxx_dlclose | release-snapshot-1118-smoke-mei-1, -smoke-keke-1 rc 0 | `SMOKE: RESULT bad=0` both hosts | same |
| package comparison | release-snapshot-1118-pkgdiff-1 (keke) rc 0 | libc6 30 -> 75 files (+43 charmaps in `/usr/share/i18n/charmaps/`, #6); libc-bin 21 -> 22 (+`/usr/bin/locale`); libc6-dbg build-ids changed (44 entries); libc6-dev 38 files (still no headers); niji6 29, niji6-dev 48 unchanged; others versions only; none missing | as expected |
| empty root | release-snapshot-1118-roots-1 (mei) rc 0 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0`; ldd-6/-7 differ only in the libc.so.6 / libm.so.6 version-needs blocks (D-31.15) | same |
| libc6-dbg + gdb (trixie, excalibur) | roots-1 | `dbg: RESULT pass=5 fail=0 pending=2` each | same |
| fresh install | roots-1 | `fresh: RESULT fail=0`; 17 installed objects have their .debug | same |
| lintian trixie / excalibur | release-snapshot-1118-lint-1 (mei) rc 0 | 10 E / 60 W each; coexist proofs pass=21 fail=0 each | +1 W: `libc-bin: no-manual-page [usr/bin/locale]` (UTIL-8); E set identical |
| #9 end to end | release-snapshot-1118-many-1 (mei) rc 0 | `YOHANE-MANY-OBJECTS pass=9 fail=0` | new check |
| #16 rerun | release-snapshot-1118-yohane-1 (mei) rc 1 | 16 PASS / 2 FAIL: the same header gap (`include_next <stdint.h>`) | same |
| staging | release-snapshot-1118-stage-1 (keke) rc 0 | keke `~/release-staging/snapshot-20261118.1/`: 9 .debs, SHA256SUMS (all OK), MANIFEST | serial 1 of 11-18 |
| install instructions (staged assets) | release-snapshot-1118-install-1 (keke) rc 0 | `inst: RESULT fails=0` (24 ok); system .debs identical to debs-1's (7 of 7) | same |
| plants (handoff/RELEASE-SNAPSHOT-1118-plants.txt) | release-snapshot-1118-plants-1 (mei) rc 0 | `6 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG`; 3 baselines ok | same as 17.3 |

Not in JOBS (rc 1, evidence read above, as at 17.3): debs-1 (rc 1 = the DBG finding only; every check passes) and
yohane-1 (rc 1 = the header finding). Diagnostic: release-snapshot-1118-mtest-1 (mei, rc 0) ran m/gen.sh + m/run.sh on the host's own C
library with N = 70 to check the #9 scripts before the root job.

## #9 (Yohane Linux: "too many objects" past 64), swapped trixie root, libniji's libc6/libc-bin/libc6-dev/libc-dev-bin

Built on the stock root before the swap (gcc 14): N = 70 and 200 libraries `libm9_I.so`; `main` needs all N
(71 / 201 DT_NEEDED); `libtop.so` needs all N; `dl` needs only libc.so.6 and dlopens libtop.so. After the swap the
root's libc.so.6, libm.so.6, loader and ldd are the package's (sha256), `/lib64/ld-linux-x86-64.so.2` ->
`/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2`. Output (libniji):

```
n=70  main: rc 0, sum 2485,  objects 74     dl: rc 0, sum 2485,  objects 4 -> 75
n=70  ldd main / ldd libtop.so: all 70 listed once, 0 not found
n=200 main: rc 0, sum 20100, objects 204    dl: rc 0, sum 20100, objects 4 -> 205
n=200 ldd main / ldd libtop.so: all 200 listed once, 0 not found
program output identical to the stock root's for all four runs; many: RESULT fail=0
```

## #17 and #6 ancestry (`git merge-base --is-ancestor`)

Both are NEW in this snapshot: neither branch tip (sock-null-optval fedc4cb44, sys-iconv-coverage-2 4c5977192) nor
merge (360b1c2e4, f5ce99498) is an ancestor of 20dbed16c; both tips are ancestors of 923df67e9. Same for
dl-startup-objects (130ac999e) and dl-dlopen-new-objects (abf1d31fd).

## Documentation currency (INFORMATIONAL, for a docs story; not fixed here)

* doccheck.py (RELEASE-SNAPSHOT-CXX-DOCS's): 244 links, 197 xtask mentions, 23 problems; vs 17.3's 21: one fixed
  (SL-HEADERS-S05-3-1.toml), new: differences-from-glibc.md:542 and :543 cite `records/pending/UTIL-8-2-1.toml`,
  which does not exist (no records/pending/); install-packages.md:22 `include/liella/` is the rename table's former
  name (false positive).
* install-packages.md:178-180 says iconv opens "only 58 of the 1180" names and ISO-8859-15 does not open: stale after
  #6 (SYS-ICONV-COVERAGE-2: 338 open with libniji's charmaps). The sentence on Debian's `iconv` program not starting
  (#6) was not re-checked here.
* docs/RELEASE-NOTES.md has no section after 2026-11-17: nothing on #9, #17, #6, `locale`, ldconfig limits.
* The 11-17.3 items still stand: install-packages.md libc-bin row lacks `/usr/bin/iconv` (now also `locale`); the
  header sentence (:59, "programs compile with the system's headers") does not hold for the system flavour.
* Install and verify instructions work against the staged assets (install-1). Not BLOCKING.
* User-visible changes since 17.3, one sentence each: programs and dlopen calls may load any number of objects
  (#9); getsockopt/setsockopt and the name calls pass a NULL buffer to the kernel (#17); libc6 ships 43 charmaps
  generated from Unicode mapping tables and iconv loads charmaps without a cap (#6); libc-bin ships `locale`;
  ldconfig has no input caps; named semaphores grow past 256; sigprocmask with a null set follows POSIX.

## Findings

* RULING (carried from 17.3, unchanged): system-flavour libc6-dev ships no headers (38 files; slices 6 and 14 are
  generated in the tree, not packaged), so a C compile in a swapped root fails (#16 rerun's 2 FAILs).
* libc6-dbg lacks debug files for `/usr/bin/iconv` (ruling #6321) and now `/usr/bin/locale`: the strip/debug list
  in tools/deb/system.sh (:143) names ldconfig, getconf, getent, ldd only. Same class as #6321.
* The coexist niji6 ships no charmaps (29 files), so the coexist flavour's iconv sees only the distribution's.
* DL-STARTUP-OBJECTS landed with a `WIP:` subject (4bec25784) on next (record only).

## Exact next step

Coordinator: tag 923df67e9 from keke `~/release-staging/snapshot-20261118.1/` (the header gap stands as at 17.3).
