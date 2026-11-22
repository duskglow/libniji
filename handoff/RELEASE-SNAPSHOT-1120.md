COMPLETE: snapshot-ready at 50abc44ed (snapshot-2026-11-20.1 staged; #26 passes end to end in the swapped trixie root with Debian's gcc-14 sanitizers and fails on 19.2's packages; every other check matches snapshot-2026-11-19.2 or improved; ASan's report under libniji is cut short after its first lines: finding 1)

# RELEASE-SNAPSHOT-1120: snapshot validation of 50abc44ed

**State.** Validated head `50abc44ed51f255f6d4393ddbf7fb62895b5509b` "Regenerate train-owned views" (origin/next at the
start, 2026-11-20 06:17 UTC; commit time 05:38:53 UTC, epoch 1795153133). `git merge-base --is-ancestor 50abc44ed`
holds (it is the head; LDSO-TLS-STATIC-INFO's merge is fdfa38c6b). Branch `task/release-snapshot-1120` = that head +
handoff/ only (no product or packaging code changed). Role mei `~/w/s2` (Skylake-SP); keke = Zen 1. Job scripts: mei
and keke `~/work/RELEASE-SNAPSHOT-1120/`, made by `copy.sh` + `subst.sed` from `~/work/RELEASE-SNAPSHOT-1119B/` (head,
epoch, serial 20261120.1, labels, staging dir, pkgdiff against snapshot-20261119.2; `binutils-src` linked to 1119B's
for y10). New this run: `y26.sh` + `y26/` (y19.sh's recipe; #26) and `startup.sh` + `startup/` (step 5).

**Landed since snapshot-2026-11-19.2** (`git log --oneline d496d4c25..50abc44ed`: 141 lines; first-parent merges):
release-snapshot-1119b (handoff only), plants-build-2, gate-test-time-2, layout-dyn-phase-4, heap-foreign-tb-speed,
dyn-start-parity-3, test-pass2-native, audit-pre-selfhost, review-sl-headers-s07, review-sl-headers-s08,
tooling-papercuts-2, plants-copies-2, **dyn-start-speed-2**, **pass2-native-runner**, math-oracle-profile,
**layout-byte-budgets-2**, **ldso-tls-static-info** (GitHub #26), train view regenerations.

## Results against snapshot-2026-11-19.2 (all on 50abc44ed's products; mei unless noted)

| check | job | result | vs 19.2 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | -smoke-mei-1 (builds products), keke -smoke-keke-1 (mei's products) | `SMOKE: RESULT bad=0` both; libc.so.6 dd122389..., ld.so f5ecdc1c... | same (new build) |
| gate | -gate-1 rc 0 | clippy --all-features -D warnings, xtask clippy, static, records check, affected --base origin/next (0 of 302 targets selected): all RC 0 | same |
| system debs, check-system | -debs-1 | `check: PASS (7 packages, version 2.39+niji20261120.1-1)` | same |
| check-symbols | debs-1 | PASS, 4440 symbol lines (19.2: 4440); the symbols file equals 19.2's but the version | same, below |
| coexist debs + check | debs-1 | `check: PASS (2 packages)` | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects; no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale` (ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged except libc6-dbg 42 -> 44 (5 build-ids replaced, 17 `.debug` files as before; directories `11/ 8f/ de/` gone, `06/ 42/ 4f/ 9e/ b2/` new); sizes within 5 KB; control fields: versions only | same |
| empty root | -roots-1 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6/-7, D-31.15) | same |
| libc6-dbg + gdb | roots-1 (its only rc 1) | `dbg: RESULT pass=8 fail=1` trixie and excalibur: the `Breakpoint 1.1, ` line (19.2's finding 1) | same |
| fresh install, `ldconfig -p` | roots-1 | `fresh: RESULT fail=0` | same |
| lintian trixie / excalibur | -lint-1 rc 0 | 10 E / 60 W each; normalised E:/W: lines identical to 19.2's | same |
| coexist proofs | lint-1 | trixie `pass=21 fail=0`, excalibur `pass=21 fail=0` | same |
| #9 many objects | -many-1 rc 0 | pass=9 fail=0 | same |
| #16 rerun | -yohane-1 rc 1 | masked PASS/FAIL lines identical to 19.2's (the documented header gap) | same |
| #22 | -dyn22-1 rc 0 | `dyn: RESULT niji fail=0`, `dyn22: RESULT fail=0` (19.2: niji fail=8, rc 1) | improved, finding 3 |
| #10 + #18 | -y10-2 rc 1 | 19 PASS / 1 FAIL (bare-name dlopen); masked lines identical to 19.2's | same |
| #19 | -y19-1 rc 0 | `y19: RESULT fail=0`, pass=14 | same |
| #26 | -y26-1 rc 0 | `y26: RESULT fail=0`, pass=20, below | fixed (19.2: -y26-ref192-2 fails) |
| start-up | -startup-mei-1, keke -startup-keke-1 | below | improved |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0` | same |
| plants | -plants-1 rc 0 | handoff/RELEASE-SNAPSHOT-1120-plants.txt: `6 plants: 6 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG` | same |
| docs | local doccheck.py on the tree | 23 problems, the same 23 as 19.2 | same |

Superseded: -y10-1 (my copy.sh left out 1119B's `binutils-src` input; rerun as -y10-2), -y26-ref192-1 (the probe
compiled after the swap, where libniji's libc6-dev has no C headers; -2 links the stock objects instead).

## #26 end to end (swapped trixie root: gcc-14 / libasan8 / liblsan0 / libtsan2 / libubsan1 14.2.0-19)

Root's libc.so.6, libm.so.6, loader and ldd are the package's (sha256 PASS). Probes built on the stock root (sb), and
the same objects linked again by gcc on libniji (nb): identical results.

| check | stock trixie | libniji 50abc44ed (y26-1) | libniji 19.2 (y26-ref192-2) |
|---|---|---|---|
| `readelf --dyn-syms -W` ld.so | `FUNC GLOBAL DEFAULT _dl_get_tls_static_info@@GLIBC_PRIVATE` | `81 FUNC GLOBAL DEFAULT _dl_get_tls_static_info@@NIJI_PRIVATE` | not exported |
| ASan (`gcc -fsanitize=address`, the issue's reproducer) | main=yes, `ERROR: AddressSanitizer: heap-buffer-overflow`, rc 1 | main=yes, same first line, rc 1 (report cut short: finding 1) | main=no, `AddressSanitizer:DEADLYSIGNAL` |
| LSan | main=yes, `ERROR: LeakSanitizer: detected memory leaks`, rc 23 | same, full report identical to stock | main=no, SIGSEGV |
| TSan, `setarch -R` | main=yes, `WARNING: ThreadSanitizer: data race`, rc 66 | same, full report identical but the tid | main=no, SIGSEGV |
| TSan, ASLR on | `FATAL: ThreadSanitizer: unexpected memory mapping` in this run (data race in another: host mmap randomness) | the same FATAL line | the same FATAL line |
| UBSan | main=yes, `runtime error: signed integer overflow`, rc 0 | same | same (worked before) |

ASan report under libniji, first lines (addresses masked):
```
main reached
==N==ERROR: AddressSanitizer: heap-buffer-overflow on address 0xX at pc 0xX bp 0xX sp 0xX
WRITE of size 1 at 0xX thread T-1
    #0 0xX in main /tmp/y/asan.c:5
    #1 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX)
    #2 0xX  (/lib/x86_64-linux-gnu/libc.so.6+0xX)
    #3 0xX in __libc_start_main (/lib/x86_64-linux-gnu/libc.so.6+0xX)
    #4 0xX in _start (/tmp/y/sb/asan+0xX)
0xX is located 0 bytes after 8-byte region [0xX,0xX)
AddressSanitizer: CHECK failed: asan_descriptions.cpp:174 "((id)) != (0)" (0xX, 0xX) (tid=N)
```
Stock: `thread T0`, one libc frame fewer, then `allocated by thread T0 here:` with the malloc stack and the SUMMARY
(48 lines; libniji 27).

## Start-up (execcount: user instructions and page faults, exec to exit, ASLR off, median of 101; trivial PIE programs)

| program | class | glibc 2.39 | snapshot-2026-11-19.2 | 50abc44ed |
|---|---|---|---|---|
| empty main | Skylake-SP | 107,588 / 49 | 8,154,234 / 76 | 782,430 / 59 |
| 3 libraries | Skylake-SP | 142,804 / 65 | 8,425,597 / 98 | 1,033,916 / 75 |
| clock_gettime | Skylake-SP | 109,698 / 50 | 8,197,536 / 80 | 816,370 / 63 |
| empty main | Zen 1 | 101,473 / 48 | 8,154,061 / 78 | 782,233 / 59 |
| 3 libraries | Zen 1 | 136,624 / 64 | 8,425,385 / 99 | 1,033,683 / 75 |
| clock_gettime | Zen 1 | 103,612 / 50 | 8,197,391 / 83 | 816,203 / 64 |

(instructions / faults; products are each snapshot's `target/niji` from its smoke build.) No regression: about 10.4x
fewer instructions and 17-24 fewer faults than 19.2; still 7.2x-7.9x glibc's instructions and +10 to +14 faults.

## Findings (INFORMATIONAL unless stated)

1. **ASan report incomplete under libniji (RULING NEEDED on tagging).** The program starts, reaches `main` and ASan
   reports the overflow, but ASan does not know the main thread (`thread T-1`) and stops on its own internal check
   (`asan_descriptions.cpp:174 "((id)) != (0)"`) before the allocation stack and the SUMMARY line. gcc's ASan tests
   that match those lines will still fail. LSan, TSan and UBSan reports match stock. Route to a loader/sanitizer
   story (the main thread's registration with the ASan runtime). Recommend: tag now (every ASan program used to jump
   to 0; now it runs and reports) and tell Yohane the report is partial.
2. **Version node.** ld.so exports `_dl_get_tls_static_info@@NIJI_PRIVATE`, not `@GLIBC_PRIVATE` as the brief and
   glibc have (LDSO-TLS-STATIC-INFO followed R14-12: no GLIBC_PRIVATE node in libniji). libasan looks it up
   unversioned, so #26 works. check-symbols: no new line, since the symbols file covers the node with its existing
   `(symver)NIJI_PRIVATE` line (the file equals 19.2's but the version).
3. **#22 dyn22 now passes**: the start and dlopen libraries' dynamic sections are read and STRTAB/SYMTAB/GNU_HASH are
   `absolute (offset = link-time value)` (19.2: `its file's dynamic section could not be read`, `nofile`); prog
   PASS lines 19 -> 25. Not traced to one story.
4. Unchanged from 19.2: dbg `Breakpoint 1.1` (19.2 finding 1); bare-name dlopen through `$ORIGIN` DT_RUNPATH (y10);
   libniji's libc6-dev ships no C headers (so the #26 probes are compiled on the stock root).
5. **Docs** (route to a docs story; install-packages.md passes against the staged assets, so not blocking):
   RELEASE-NOTES.md still ends at 2026-11-17 and its Known limitations (line 74) still say "no debugger rendezvous
   (`_r_debug`) ... yet". No user doc changed since 19.2 (README, RELEASE-NOTES, differences-from-glibc, compat-flags,
   install-packages, spec-symbol-gaps identical; architecture.md and plants.md are developer docs). User-visible
   since 19.2: "ld.so exports `_dl_get_tls_static_info` (at `NIJI_PRIVATE`) and its exports are found by
   `dlsym(RTLD_DEFAULT/RTLD_NEXT)`, so programs built with gcc's `-fsanitize=address`, `leak` or `thread` start
   (D-27.65; GitHub #26)"; "a dynamically linked program's start-up runs about 10x fewer instructions and 17-24 fewer
   page faults than in snapshot-2026-11-19.2 (DYN-START-SPEED-2, DYN-START-PARITY-3, LAYOUT-DYN-PHASE-4)";
   "`dl_iterate_phdr` callers can read the dynamic sections of start-time and dlopen'd libraries (finding 3)".

## Staged assets (keke `~/release-staging/snapshot-20261120.1/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261120.1-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261120.1-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261120+50abc44ed51f_amd64.deb`), SHA256SUMS,
MANIFEST. Nothing to GitHub, no tag.

## Exact next step

Coordinator: rule on finding 1, then tag/publish snapshot-2026-11-20.1 from the staged directory at 50abc44ed when the
owner says so. Not to redo: anything above. Plants: handoff/RELEASE-SNAPSHOT-1120-plants.txt. Report:
handoff/reports/RELEASE-SNAPSHOT-1120.json.
