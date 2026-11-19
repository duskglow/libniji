COMPLETE: snapshot-ready at d496d4c25 (snapshot-2026-11-19.2 staged; #19 passes end to end in the swapped trixie root and fails on 19.1's packages; every other check matches snapshot-2026-11-19.1 or improved, each difference explained below)

# RELEASE-SNAPSHOT-1119B: snapshot validation of d496d4c25

**State.** Validated head `d496d4c25cb16f57cc4ea3390a551b6c3b74a58c` "Merge task/plants-build-slowdown into next"
(origin/next at the start, 2026-11-19 18:34 UTC, and still at the end; commit time 17:20:15 UTC, epoch 1795108815).
Branch `task/release-snapshot-1119b` = that head + handoff/ only (no product or packaging code changed). Role mei
`~/w/s2` (Skylake-SP); keke = Zen 1. Job scripts: mei and keke `~/work/RELEASE-SNAPSHOT-1119B/`, made by `copy.sh` +
`subst.sed` from `~/work/RELEASE-SNAPSHOT-1119/` (head, epoch, serial 20261119.2, role s2, labels, staging dir, pkgdiff
against snapshot-20261119.1). New this run: `y19.sh` + `y19/` (y10.sh's recipe; #19) and `dbg11.sh` + `dbg11/` (one
probe, below). tree/ = local `git archive` of d496d4c25.

**Landed since snapshot-2026-11-19.1** (`git log --oneline fb63eeec8..d496d4c25`: 92 lines; first-parent merges):
release-snapshot-1119 (handoff only), **headers-gen-complete**, gate-dev-no-full, test-scratch-base,
**headers-tables-complete**, **dl-23-2** (includes **dl-23**, GitHub #19), train view regenerations,
**plants-build-slowdown**. `git merge-base --is-ancestor` origin/task/{dl-23, dl-23-2, plants-build-slowdown,
headers-gen-complete, headers-tables-complete} d496d4c25: all yes. #6 / #21: no origin/task/heap-address-space* or
iconv-name-match* tip is an ancestor (no fix landed).

## Results against snapshot-2026-11-19.1 (all on d496d4c25's products; mei unless noted)

| check | job | result | vs 19.1 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | -smoke-mei-1 (builds products), keke -smoke-keke-1 (mei's products) | `SMOKE: RESULT bad=0` both hosts; libc.so.6 sha256 896e4a46..., ld.so 2db5ffdb... | same (new build) |
| gate | -gate-1 rc 0 | clippy --all-features -D warnings, xtask clippy, static, records check, affected --base origin/next: all RC 0 | same |
| system debs, check-system | -debs-1 | `check: PASS (7 packages, version 2.39+niji20261119.2-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines (19.1: 4438): + `_r_debug@GLIBC_2.2.5 2.2.5` and + the `(symver)NIJI_PRIVATE` line (ld.so exports `_dl_debug_state@@NIJI_PRIVATE`, FUNC; D-27.62) | +2, both DL-23 |
| coexist debs + check | debs-1 | builds; `check: PASS (2 packages)` | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects; no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale` (ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged except libc6-dbg 44 -> 42 (5 build-ids replaced; two new ids share the `11/` directory and `71/` already existed: 5 `.debug` files as before, 2 directories fewer); sizes within 19 KB; control fields: versions only | same |
| empty root | -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6/-7 loader trace output, D-31.15), as in 19.1 | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur (19.1: pass=5 fail=0 pending=2; DL-23 turned the pending gdb checks into real ones) | FINDING 1 |
| fresh install, `ldconfig -p` | roots-1 | `fresh: RESULT fail=0` | same |
| lintian trixie / excalibur | -lint-1 rc 0 | 10 E / 60 W each; normalised logs (build-ids, versions, temp names masked) identical to 19.1's | same |
| coexist proofs | lint-1 | `proof: RESULT suite=trixie pass=21 fail=0`, excalibur `pass=21 fail=0` | same |
| #9 many objects | -many-1 rc 0 | pass=9 fail=0 | same |
| #16 rerun | -yohane-1 rc 1 | 16 PASS / 2 FAIL; masked PASS/FAIL lines identical to 19.1's (the documented header gap) | same |
| #22 | -dyn22-1 rc 1 | `dyn: RESULT stock fail=0`, `niji fail=8` (19.1: 9) | improved, FINDING 2 |
| #10 + #18 | -y10-1 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen, pre-existing); masked lines identical to 19.1's y10-2 | same |
| #19 | -y19-1 rc 0 | `y19: RESULT fail=0`, below | fixed (19.1: -y19-ref191-3 fails) |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0`; staged system debs identical to debs-1's (sha256, 7 of 7) | same |
| plants | -plants-1 rc 0 | handoff/RELEASE-SNAPSHOT-1119B-plants.txt (19.1's six, ids/paths/version changed): `6 plants: 6 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG` | same |
| docs | local doccheck.py on the tree | 23 problems, the same 23 as 19.1 (differences-from-glibc.md line numbers shift) | same |
| dbg 1.1 probe | -dbg11-1 rc 0 | below (FINDING 1) | - |

Superseded: -y19-ref191-1 / -2 (probe script fixes: gnat1 prints nothing for `--version`; `info sharedlibrary`
before the dlopen; gnatmake log prefix; -2 stopped). -y19-ref191-3 is the valid 19.1 reference.

## #19 end to end (swapped trixie root: gcc-14 / gnat-14 14.2.0-19, gdb 16.3-1; probes built on the stock root)

Root's libc.so.6, libm.so.6, loader and ldd are the package's (sha256 PASS); libc6, libc-bin, libc6-dev, libc-dev-bin
2.39+niji20261119.2-1. libniji d496d4c25 (y19-1) vs 19.1's packages (y19-ref191-3):

| check | libniji d496d4c25 | libniji 19.1 |
|---|---|---|
| `readelf --dyn-syms -W` staged ld.so | `40 OBJECT GLOBAL DEFAULT _r_debug@@GLIBC_2.2.5` | not exported |
| Debian gnat1 (`/usr/libexec/gcc/x86_64-linux-gnu/14/gnat1`, EXEC, `R_X86_64_COPY _r_debug@GLIBC_2.2.5`) | `gnat1 --version`: rc 0 (prints nothing, as on the stock root) | `undefined symbol: _r_debug` |
| issue's minimal check, no PIE (`extern struct r_debug _r_debug`, COPY reloc) | `r_version=1 r_state=0 r_map=set`; `dlopen libm.so.6: ok, r_map entries before=4 after=5, r_state=0`, rc 0 | `undefined symbol: _r_debug` |
| same as PIE | identical line, rc 0 | `undefined symbol: _r_debug` |
| gdb, no PIE / PIE | `stop-at-main=yes sharedlibrary-at-main=[ ld-linux-x86-64.so.2 libc.so.6 ] bp-in-dlopened-lib=yes sharedlibrary-there=[ ... libg19.so ] g19_fn(2)=42 exited-normally=yes` | at main lists only ld.so; no stop in libg19.so |
| gnatmake-14 of a one-line Ada unit | `gcc-14 -c`, `gnatbind-14 -x`, `gnatlink-14 hello.ali` rc 0; runs: `ada: hello from gnat` | rc 127 |

`lines that differ from the stock run: (none)`.

## Headers since 19.1 (generated from the tables now)

libc-include differs from 19.1's tree in 14 files (+added -removed lines): argp.h +8-8, fmtmsg.h +4-4, getopt.h
+1-1, mcheck.h +7-7, printf.h +1-1, stdio.h +4-1, stdlib.h +83-3, sys/auxv.h +12-3, sys/epoll.h +4-4, sys/pidfd.h
+63-1, sys/ptrace.h +37-8, sys/types.h +83-3, time.h +2-2, utmpx.h +8-1. The packages ship no headers (libc6-dev 38,
niji6-dev 48 files, unchanged); the swapped root's C and C++ builds against the staged headers: 63 same, 2 known, 0 bad.

## Findings (INFORMATIONAL; none blocks the snapshot)

1. **dbg `FAIL gdb in a running /usr/bin/true`** (trixie, excalibur): gdb stops and shows everything
   (`#0 setlocale () at boundary/src/posix/s07_locale.rs:18`, bt to `__libc_start_main`, `info sharedlibrary` with
   ld.so and libc.so.6), but prints `Breakpoint 1.1, ` and tools/deb/dbg-system.sh:90 wants `^Breakpoint 1, `.
   Probe -dbg11-1 (`break setlocale`, run, `info breakpoints`; Debian's libc6-dbg vs libniji's): stock has one
   location; libniji has `<MULTIPLE>`: 1.1 `setlocale at boundary/src/posix/s07_locale.rs:18` (the entry) and 1.2
   `setlocale at boundary/src/locale.rs:792` (an inner function named `setlocale` inlined into it). So a `continue`
   in gdb can stop a second time in the same call. Route to a DL/debug-package story: either name the inner function
   differently or let the check accept `1.1`; recommend the rename (the user-visible part). Not a regression (19.1
   had no rendezvous, so this check was pending).
2. **#22 dyn22: one fewer probe failure.** `dl_iterate_phdr` now reports the start-time `$ORIGIN` library (19.1:
   `reports start not at all`), and ld.so's and libc's `dlpi_name` are now the full paths, as on stock (19.1:
   `ld-linux-x86-64.so.2`, `libc.so.6`). Left: the start library's `dlpi_name` is `./libstart22.so` (stock
   `/tmp/d/libstart22.so`), and the four `nofile` lines (now classed `start`, 19.1 `other`) are the probe-side
   lines explained in RELEASE-SNAPSHOT-1118B.md. Route the relative `dlpi_name` to a loader story.
3. Loader: bare-name `dlopen` through the executable's DT_RUNPATH `$ORIGIN` fails (y10, as in 18.2 and 19.1).
4. **Docs** (route to a docs story; install-packages.md install and verify pass against the staged assets, so not
   blocking): docs/RELEASE-NOTES.md still ends at 2026-11-17 and its Known limitations (line 74) still say "no
   debugger rendezvous (`_r_debug`) ... yet". User-visible since 19.1: "ld.so exports `_r_debug@GLIBC_2.2.5` (a
   40-byte OBJECT) and keeps a program's COPY of it current, so programs such as gnat1 that copy it start and
   debuggers follow shared libraries" (recorded difference D-27.62, compat entry COMPAT-35 in
   docs/compat-flags.md, differences-from-glibc.md row "A program's copy of `_r_debug`"); `_r_debug` left
   docs/spec-symbol-gaps.tsv; `dl_iterate_phdr` names (finding 2); generated libc-include headers (build tree only).

## Staged assets (keke `~/release-staging/snapshot-20261119.2/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261119.2-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261119.2-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261119+d496d4c25cb1_amd64.deb`), SHA256SUMS,
MANIFEST. Nothing to GitHub, no tag.

## Exact next step

Coordinator: tag/publish snapshot-2026-11-19.2 from the staged directory at d496d4c25 when the owner says so (Yohane
Linux is waiting on #19). Not to redo: anything above. Plants: handoff/RELEASE-SNAPSHOT-1119B-plants.txt. Reports:
handoff/reports/RELEASE-SNAPSHOT-1119B.json.
