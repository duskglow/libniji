BLOCKING: not snapshot-ready at 852c1b495: the coexist flavour (niji6, niji6-dev) does not build (`cargo xtask so` with prefix /usr/lib/niji stops in the interpreter trace); everything else matches snapshot-2026-11-18.1 and #22 is fixed end to end

# RELEASE-SNAPSHOT-1118B: snapshot validation of 852c1b495 (GitHub #22, dynamic-section addresses)

**State.** Validated head `852c1b49581c8a1c046416d222a6fa0fa9edb58c` "Regenerate train-owned views" (origin/next at the
start, 2026-11-18 16:50 UTC, unchanged at 17:50; epoch 1795017415). Branch `task/release-snapshot-1118b` = that head +
handoff/ only (no product or packaging code changed). Role mei `~/w/s2` (Skylake-SP); keke = Zen 1. Job scripts: mei
and keke `~/work/RELEASE-SNAPSHOT-1118B/` (RELEASE-SNAPSHOT-1118's with head, epoch, serial 2, role s2 and labels
changed; new: dyn.sh + d/ for #22, chain-keke.sh). Nothing staged: `~/release-staging/snapshot-20261118.2/` does not
exist (serial 2 unused). Two legs (refresh 1 of 3).

**Landed since snapshot-2026-11-18.1** (`git log --oneline 923df67e..852c1b49`: 105 lines, 11 merges): dl-dynamic-reloc
(#22; tip ebe6baa20 is an ancestor of 852c1b495, not of 923df67e), layout-ldso-window-2, sl-headers-registry,
sl-headers-s09-4, handback-inloop-checks-2, release-snapshot-1118 (handoff), layout-ldso-order-3, pkg-manifest-4, train
view regenerations.

## Blocking finding (RULING NEEDED)

Coexist build, job release-snapshot-1118b-debs-1 (mei): `tools/deb/build.sh` (coexist) rc 1 at
`so: trace-at-build: tracing the interpreter failed (exit status: 1)` with
`trace_gdb: the program read host files the tree does not supply (canon_host.sh): /usr/lib/niji/etc/ld.so.preload`;
`check.sh` rc 2 (no niji6 / niji6-dev .debs). The interpreter trace (tools/hottext/ldso_trace.sh, from LAYOUT-LDSO-ORDER-3,
f79f1e35f, not in 923df67e) rejects the prefixed loader's read of `<prefix>/etc/ld.so.preload`, a path that exists on
neither host: deterministic, not host state. The system flavour (prefix /) builds. Recommend: a fix story on
LAYOUT-LDSO-ORDER-3's trace (the `<prefix>/etc/` reads of a prefixed build are product paths, not host inputs), then
rerun this snapshot on the fixed head (reuse serial 2). Do not tag 852c1b495.

## Results against snapshot-2026-11-18.1 (all on 852c1b495's products; mei unless noted)

| check | job | result | vs 18.1 |
|---|---|---|---|
| gate | release-snapshot-1118b-gate-1 rc 0 | clippy (both), static, records check, affected (0 suites: branch = next + handoff) all RC 0 | same |
| system debs, check-system, check-symbols | debs-1 (system steps rc 0) | `check: PASS (7 packages, version 2.39+niji20261118.2-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)` | same |
| **coexist debs + check** | debs-1 rc 1 | build fails (above); check rc 2 | **NEW, BLOCKING** |
| package comparison | keke release-snapshot-1118b-pkgdiff-1 rc 0 | 7 system packages: file counts unchanged (libc6 75, libc-bin 22, libc6-dev 38, libc6-dbg 44, 7 each for the placeholders); libc6-dbg 5 build-ids replaced; sizes within 8 KB; control fields only the version. `MISSING niji6`, `MISSING niji6-dev` | coexist missing (the block) |
| empty root | release-snapshot-1118b-roots-1 rc 0 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0`; ldd-6/-7 known (version-needs blocks, D-31.15) | same |
| libc6-dbg + gdb | roots-1 | `dbg: RESULT pass=5 fail=0 pending=2` (trixie, excalibur) | same |
| fresh install, `ldconfig -p` | roots-1 | `fresh: RESULT fail=0` | same |
| lintian trixie / excalibur | release-snapshot-1118b-lint-1 rc 1 | 10 E / 60 W each, tag set identical to 18.1's (build-ids and versions normalised); the two coexist proof steps rc 2 (no coexist .debs) | E/W same; proofs not run (the block) |
| lua_reload, cxx_dlclose | release-snapshot-1118b-smoke-mei-1, keke -smoke-keke-1 rc 0 | `SMOKE: RESULT bad=0` both hosts | same |
| #9 many objects | release-snapshot-1118b-many-1 rc 0 | `YOHANE-MANY-OBJECTS pass=9 fail=0` | same |
| #16 rerun | release-snapshot-1118b-yohane-1 rc 1 | 16 PASS / 2 FAIL (the header gap, `include_next <stdint.h>`) | same |
| plants (handoff/RELEASE-SNAPSHOT-1118B-plants.txt) | release-snapshot-1118b-plants-1 rc 0 | `6 plants: 5 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 1 WARN-NO-MSG`; 3 baselines ok | +1 WARN (below) |
| docs | local doccheck.py | 23 problems, the same 23 as 18.1 | same |

**WARN-NO-MSG, rs18b-postrm-not-packaged:** re-aimed after PKG-MANIFEST-4 (it skips the DEBIAN/postrm manifest row). The
mutant fails `build.sh` itself (`chmod: cannot access 'target/rpw/stage/libc-bin/DEBIAN/postrm'`) before
`check-system.sh` runs, so the defect is caught, one step earlier than the plant's `msg:` names. Next snapshot: set its
`msg:` to that chmod line or plant past the chmod.

## #22 end to end (swapped trixie root, distribution gcc, `-pie -fPIE`, `-Wl,-rpath,'$ORIGIN/' --enable-new-dtags`)

release-snapshot-1118b-dyn22-2 on 852c1b495; release-snapshot-1118b-dyn22-ref181-1 = the same probe on
snapshot-20261118.1's .debs (reference). Values as absolute and as offset from `dlpi_addr`:

| object | 852c1b495 STRTAB / SYMTAB / GNU_HASH offset | link-time value (file) | 18.1 debs |
|---|---|---|---|
| main (PIE) | 0x678 / 0x3d8 / 0x3b0 | same: absolute PASS | link-time values (FAIL) |
| libc.so.6 | 0x16510 / 0x5788 / 0x298 | same: PASS | FAIL |
| ld.so | 0x398 / 0x2a8 / 0x260 | same: PASS | FAIL |
| start-time lib (`$ORIGIN/`) | 0x318 / 0x288 / 0x260 | 0x318 / 0x288 / 0x260 (stock run, same file): absolute | not checkable (see below) |
| dlopened lib | 0x318 / 0x288 / 0x260 | 0x318 / 0x288 / 0x260 (stock run, same file): absolute | not checkable |
| vDSO | values 0x300 / 0x1c8 / 0x168 (link-time, as expected) | | same |
| `_DYNAMIC` | = main's, absolute PASS | | FAIL |

systemd pattern: `DT_RUNPATH d_val=0x148, DT_STRTAB + d_val -> "$ORIGIN/"` PASS (on 18.1's .debs the program
crashes, rc 139: the #22 report reproduced). Every object except the vDSO holds absolute addresses: **#22 fixed**.
The job's summary is `YOHANE-DYNAMIC pass=1 fail=2` because of 9 probe FAILs, all in ref181 too: 8 `nofile` (the probe
finds an object's file through the /proc/self/maps line covering `dlpi_addr`; for the start-time and dlopened libraries
loaded by libniji that line names no file, so it compared against the stock run's values above by hand) and 1
`reports start not at all` (that library's `dlpi_name` is `''`, so the probe classed it "other"; it is reported once).

## INFORMATIONAL

* `dl_iterate_phdr` names under libniji: `libc.so.6` and `ld-linux-x86-64.so.2` (bare DT_NEEDED names) and `''` for a
  library found through `$ORIGIN/`; the stock root reports absolute paths; spec/behaviour/dynlink.md R-3 says libc and
  ld.so by absolute path. Identical on 18.1's .debs (name lines diffed): not new. Route to a loader story.
* For a library mapped by libniji, the /proc/self/maps line covering `dlpi_addr` names no file (both heads).
* docs: docs/RELEASE-NOTES.md has no section after 2026-11-17 (nothing on 18.1's changes or #22); differences-from-glibc.md
  gained 2 slice-9 header rows (SL-HEADERS-S09-4). User-visible changes since 18.1: #22 (d_ptr entries hold run-time
  addresses), headers slice 9 (coexist niji6-dev). install-packages.md install/verify not run: nothing staged.
* **#6 / #21** (`git merge-base --is-ancestor`): no fix landed since 923df67e. No origin/task/heap-address-space{,-2..-7}
  or iconv-name-match{,-2,-3} tip is an ancestor of 852c1b495; sys-iconv-charmaps, -coverage, -coverage-2 were already
  in 923df67e.

## Staged assets

None (blocked). The validated system .debs are on mei `~/work/RELEASE-SNAPSHOT-1118B/debs/system/` and keke (same path,
sha256 checked, 7 of 7).

## Exact next step

Coordinator: open the fix story for the prefixed interpreter trace; after it lands, rerun this snapshot from
`~/work/RELEASE-SNAPSHOT-1118B/` on the new head (change head/epoch only; serial 2 still unused), then stage with
chain-keke.sh. Not to redo: #6/#21 answers, the doc check (unless docs change).
