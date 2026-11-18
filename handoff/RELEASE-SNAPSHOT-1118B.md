WIP (leg 1 of RELEASE-SNAPSHOT-1118B): BLOCKING so far: the coexist flavour does not build at 852c1b495

# RELEASE-SNAPSHOT-1118B: snapshot validation of 852c1b495 (GitHub #22, dynamic-section addresses)

**State.** Head to validate `852c1b49581c8a1c046416d222a6fa0fa9edb58c` "Regenerate train-owned views" (origin/next at
the start, 2026-11-18 16:50 UTC; commit time 1795017415). Branch `task/release-snapshot-1118b` = that head + handoff/
only. Role mei `~/w/s2` (synced, tree dd0e1c162e7b). Job scripts: mei and keke `~/work/RELEASE-SNAPSHOT-1118B/`
(RELEASE-SNAPSHOT-1118's with head, epoch 1795017415, serial 2, role s2, labels release-snapshot-1118b-* changed;
stage.sh's note rewritten for this snapshot; new: dyn.sh + d/ for #22, chain-keke.sh). Local scratch
`~/w/.scratch/RELEASE-SNAPSHOT-1118B/` (old/ = 1118's scripts, new/ = this run's, doccheck.py, doccheck.txt). No
product or packaging code changed. mei = Skylake-SP role host, keke = Zen 1.

**Landed since snapshot-2026-11-18.1** (`git log --oneline 923df67e..852c1b49`: 105 lines, 11 merges): dl-dynamic-reloc
(#22; tip ebe6baa20 is an ancestor of 852c1b495, not of 923df67e), layout-ldso-window-2, sl-headers-registry,
sl-headers-s09-4, handback-inloop-checks-2, release-snapshot-1118 (handoff), layout-ldso-order-3, pkg-manifest-4, plus
train view regenerations.

**#6 / #21** (`git merge-base --is-ancestor`): no fix landed since 923df67e. Every origin/task/heap-address-space{,-2..-7}
and iconv-name-match{,-2,-3} tip is an ancestor of neither head; sys-iconv-charmaps, -coverage, -coverage-2 were
already in 923df67e.

## Results so far (all mei unless noted)

| check | job | result | vs 18.1 |
|---|---|---|---|
| smoke lua_reload + cxx_dlclose (mei) | release-snapshot-1118b-smoke-mei-1 rc 0 | `SMOKE: RESULT bad=0` | same |
| gate | release-snapshot-1118b-gate-1 rc 0 | not yet read (ended fast: role = origin/next) | |
| system debs + check-system + check-symbols | release-snapshot-1118b-debs-1 rc 1 | `check: PASS (7 packages, version 2.39+niji20261118.2-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)`; DBG 17 of 19 (iconv, locale: as 18.1) | symbols same |
| **coexist build** | debs-1 | **FAIL**: `cargo xtask so` (prefix /usr/lib/niji) stops at `trace-at-build: tracing the interpreter failed`: `trace_gdb: the program read host files the tree does not supply (canon_host.sh): /usr/lib/niji/etc/ld.so.preload` (tools/hottext/trace_gdb.py:517; the ldso trace tools/hottext/ldso_trace.sh came with LAYOUT-LDSO-ORDER-3, f79f1e35f, not in 923df67e). The path does not exist on either host: the loader's attempted read of `<prefix>/etc/ld.so.preload` is what the rule rejects, so it is deterministic. No niji6/niji6-dev .debs; check.sh rc 2 | **NEW, BLOCKING** |
| #9 many | release-snapshot-1118b-many-1 rc 0 | `YOHANE-MANY-OBJECTS pass=9 fail=0`; n=70/200 main/dl output identical | same |
| #16 rerun | release-snapshot-1118b-yohane-1 rc 1 | 16 PASS / 2 FAIL, the header gap (`include_next <stdint.h>`) | same |
| #22 first run | release-snapshot-1118b-dyn22-1 rc 1 | SUPERSEDED (probe defect: it opened each object's file by dlpi_name) | |
| docs (doccheck.py on the head's tree) | local | 23 problems, the same 23 as 18.1 (differences-from-glibc.md lines +2 after two slice-9 header rows) | same |

**#22 finding from dyn22-1 (keep):** in the swapped trixie root libniji's `dl_iterate_phdr` reports `dlpi_name`
`libc.so.6` and `ld-linux-x86-64.so.2` (bare DT_NEEDED names) and `''` for the start-time library found through
`$ORIGIN/`; the stock root reports `/lib/x86_64-linux-gnu/libc.so.6`, `/lib64/ld-linux-x86-64.so.2`,
`/tmp/d/libstart22.so`. spec/behaviour/dynlink.md R-3 says libc and ld.so by absolute path. The d_ptr values seen
(main, dlopened lib, _DYNAMIC) were absolute and DT_RUNPATH via DT_STRTAB printed `$ORIGIN/`.
dyn22-2 (probe finds files through /proc/self/maps, compares name lines) and dyn22-ref181-1 (same probe on
snapshot-20261118.1's .debs, logs-dyn22-ref181: is the name behaviour new?) are running.

## Running at the refresh (mei unless noted)

release-snapshot-1118b-roots-1, -lint-1 (lint's coexist proof steps will fail: no coexist .debs), -dyn22-2,
-dyn22-ref181-1, -plants-1 (`/mnt/zram/claude/release-snapshot-1118b/plants-1`); keke: -smoke-keke-1.

## Exact next step

1. `~/bin/jobwait` the mei jobs above; keke `~/bin/jobwait release-snapshot-1118b-smoke-keke-1`. Read summaries:
   roots (`root: RESULT`, `verdict:`, `dbg: RESULT`, `fresh: RESULT`), lint (E/W counts vs 18.1's 10 E / 60 W each;
   the system lint steps only), dyn22-2 (`YOHANE-DYNAMIC`, `dyn22:` lines, the class/name diff), ref181, plants
   (`CAUGHT`), gate (`=== RC` lines).
2. Relay system .debs mei -> keke `~/work/RELEASE-SNAPSHOT-1118B/debs/system` (sha256 checked); run pkgdiff on keke
   (needs debs/coexist too; with none it reports niji6/niji6-dev MISSING: that is the finding).
3. Do NOT stage (stage.sh needs coexist .debs and the snapshot is blocked); report BLOCKING with RULING NEEDED:
   fix the interpreter trace for a prefixed build (recommend a fix story on LAYOUT-LDSO-ORDER-3's trace: the
   `<prefix>/etc/ld.so.preload` read is a product path, not a host input), then rerun this snapshot (serial 2 unused).
4. precheck, report json, handback-check.
