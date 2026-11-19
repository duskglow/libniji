COMPLETE: snapshot-ready at 557dfdaa6 (snapshot-2026-11-18.2 staged; coexist builds again; #22 fixed end to end; every other check matches snapshot-2026-11-18.1)

# RELEASE-SNAPSHOT-1118B (run 2, RELEASE-SNAPSHOT-1118B-2): snapshot validation of 557dfdaa6

**State.** Validated head `557dfdaa6b33cdfc650866e9ce3fa7505453fe4c` "Merge task/ldso-trace-prefix into next" (origin/next at
the start, 2026-11-18 23:29 UTC; commit time 22:03:40 UTC, epoch 1795039420). Branch `task/release-snapshot-1118b-2` =
run 1's handoff commits + a merge of that head + handoff/ only (no product or packaging code changed). Role mei `~/w/s2`
(Skylake-SP); keke = Zen 1. Job scripts: mei and keke `~/work/RELEASE-SNAPSHOT-1118B-2/` (run 1's, head, epoch, work dir
and labels changed; tree/ = local `git archive` of 557dfdaa6 copied with rsync). Run 1 (852c1b495) results are superseded.

**Landed since snapshot-2026-11-18.1** (`git log --oneline 923df67e..557dfdaa6`: 123 lines, 13 merges): dl-dynamic-reloc
(#22), layout-ldso-window-2, sl-headers-registry, sl-headers-s09-4, handback-inloop-checks-2, release-snapshot-1118
(handoff), layout-ldso-order-3, pkg-manifest-4, **sl-headers-vis-fix** (struct timespec where struct stat is visible,
WIFCONTINUED, features-time64.h / gnu/stubs.h names), **ldso-trace-prefix** (the build-time interpreter trace runs a
prefixed loader with its own `<prefix>/etc` files absent: run 1's block), train view regenerations.

## Results against snapshot-2026-11-18.1 (all on 557dfdaa6's products; mei unless noted)

| check | job | result | vs 18.1 |
|---|---|---|---|
| step 0 (inherited head) | release-snapshot-1118b-2-lint0-1 rc 0 | clippy --all-features -D warnings, static | - |
| gate | release-snapshot-1118b-2-gate-1 rc 0 | clippy (both), static, records check, affected (branch = next + handoff) all RC 0 | same |
| system debs, check-system, check-symbols | release-snapshot-1118b-2-debs-1 | `check: PASS (7 packages, version 2.39+niji20261118.2-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)` | same |
| coexist debs + check | debs-1 | builds (`tracing the interpreter (prefix /usr/lib/niji)` passes); `check: PASS (2 packages)` | same (run 1's block gone) |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects, no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale` (ruling #6321); same build-ids as run 1 | same (18.1: rc 1 for this, kept out of JOBS) |
| smoke lua_reload, cxx_dlclose | -smoke-mei-1 (builds products), keke -smoke-keke-1 (mei's products) | `SMOKE: RESULT bad=0` both hosts; libc.so.6 sha256 ad0d7ef4..., ld.so 16f3ed79... | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc-bin 22, libc6-dev 38, libc6-dbg 44, niji6 29, niji6-dev 48, placeholders 7); libc6-dbg 5 build-ids replaced (getconf, ld.so, ldconfig, ldd, libc.so.6); sizes within 8 KB; control fields only versions; none missing | same |
| empty root | -roots-1 rc 0 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6/-7 loader trace output, D-31.15) | same |
| libc6-dbg + gdb | roots-1 | `dbg: RESULT pass=5 fail=0 pending=2` trixie and excalibur | same |
| fresh install, `ldconfig -p` | roots-1 | `fresh: RESULT fail=0` (17 installed objects with their .debug) | same |
| lintian trixie / excalibur | -lint-1 rc 0 | 10 E / 60 W each; normalised tag lists (build-ids, versions) identical to 18.1's, 70 lines each | same |
| coexist proofs | lint-1 | `proof: RESULT suite=trixie pass=21 fail=0`, excalibur `pass=21 fail=0` | same |
| #9 many objects | -many-1 rc 0 | `YOHANE-MANY-OBJECTS pass=9 fail=0` | same |
| #16 rerun | -yohane-1 rc 1 | 16 PASS / 2 FAIL (header gap `include_next <stdint.h>`, then cm-full rc 127); PASS/FAIL lines identical to run 1 | same |
| #22 | -dyn22-1 rc 1 | below; rc 1 = the 9 probe-side FAILs of run 1 | fixed (18.1 crashed) |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` (24 ok); system debs identical to debs-1's (7 of 7) | same |
| plants (handoff/RELEASE-SNAPSHOT-1118B-2-plants.txt) | -plants-1 rc 0 | `6 plants: 6 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG`; 3 baselines ok | run 1's WARN-NO-MSG gone (postrm msg re-aimed) |
| docs | local doccheck.py on the tree | 23 problems, the same 23 as 18.1 and run 1 (two differences-from-glibc.md line numbers shift) | same |

## #22 end to end (swapped trixie root, distribution gcc, `-pie -fPIE`, `-Wl,-rpath,'$ORIGIN/' --enable-new-dtags`)

job release-snapshot-1118b-2-dyn22-1; values as offset from `dlpi_addr` (absolute value - dlpi_addr):

| object | STRTAB / SYMTAB / GNU_HASH offset | link-time value | verdict |
|---|---|---|---|
| main (PIE) | 0x678 / 0x3d8 / 0x3b0 | same (file) | absolute |
| libc.so.6 | 0x16510 / 0x5788 / 0x298 | same (file) | absolute |
| ld.so | 0x398 / 0x2a8 / 0x260 | same (file) | absolute |
| start-time lib (`$ORIGIN/`, dlpi_name '') | 0x318 / 0x288 / 0x260 | 0x318 / 0x288 / 0x260 (stock run, same file) | absolute |
| dlopened lib | 0x318 / 0x288 / 0x260 | 0x318 / 0x288 / 0x260 (stock run, same file) | absolute |
| vDSO | values 0x300 / 0x1c8 / 0x168 | link-time, as expected | - |
| `_DYNAMIC` | = main's | | absolute |

systemd pattern: `DT_RUNPATH d_val=0x148, DT_STRTAB + d_val -> "$ORIGIN/"`, `dyn: PASS DT_RUNPATH via DT_STRTAB is $ORIGIN/`.
Stock run: `dyn: RESULT stock fail=0`. libniji: `dyn: RESULT niji fail=9`, the same 9 as run 1 and as 18.1's reference
run: 8 `nofile` (the probe finds an object's file through the /proc/self/maps line covering `dlpi_addr`; for the two
libraries libniji maps that line names no file, so their values were compared with the stock run's by hand above) and
`reports start not at all` (dlpi_name `''`, classed "other"). The 18.1 reference run (run 1's dyn22-ref181-1) was not
repeated: it measures 18.1's .debs, not this head.

## #6 / #21 (`git merge-base --is-ancestor`)

No fix landed since 923df67e: no origin/task/heap-address-space{,-2..-7} or iconv-name-match{,-2,-3} tip is an ancestor
of 557dfdaa6 (nor of 923df67e); the two merges since run 1 are ldso-trace-prefix and sl-headers-vis-fix.

## INFORMATIONAL

* Run 1's inside.sh had the coexist version typed wrong (`+852c1b495810` for `+852c1b49581c`); never run (nothing was
  staged). This run's copy uses `+557dfdaa6b33`, which build.sh produced. Next snapshot: derive it from the staged names.
* `dl_iterate_phdr` names (bare DT_NEEDED names, `''` for a `$ORIGIN/` library) and the nameless /proc/self/maps line of a
  libniji-mapped library: unchanged from run 1 (route to a loader story).
* docs: docs/RELEASE-NOTES.md still ends at 2026-11-17 (nothing on 18.1, 18.2, #22). User-visible since 18.1: #22 (d_ptr
  entries hold run-time addresses); headers slice 9 and the visibility fixes (niji6-dev: struct timespec where struct stat
  is visible, WIFCONTINUED, features-time64.h / gnu/stubs.h names); differences-from-glibc.md one line shorter than at
  852c1b495. install-packages.md install and verify: pass against the staged assets. Route to a docs story.

## Staged assets (keke `~/release-staging/snapshot-20261118.2/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261118.2-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261118.2-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261118+557dfdaa6b33_amd64.deb`), SHA256SUMS,
MANIFEST. Nothing to GitHub, no tag.

## Exact next step

Coordinator: tag/publish snapshot-2026-11-18.2 from the staged directory at 557dfdaa6 when the owner says so. Not to
redo: anything above. Plants file: handoff/RELEASE-SNAPSHOT-1118B-2-plants.txt.
