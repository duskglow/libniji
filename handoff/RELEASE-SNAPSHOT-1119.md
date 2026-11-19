COMPLETE: snapshot-ready at fb63eeec8 (snapshot-2026-11-19.1 staged; #10 and #18 pass end to end and fail on 18.2's packages; every other check matches snapshot-2026-11-18.2)

# RELEASE-SNAPSHOT-1119: snapshot validation of fb63eeec8

**State.** Validated head `fb63eeec883de8da35e9b153715159a91f5c58c2` "Merge task/strptime-signed-year into next"
(origin/next at the start, 2026-11-19 06:03 UTC; commit time 04:52:21 UTC, epoch 1795063941). Branch
`task/release-snapshot-1119` = that head + handoff/ only (no product or packaging code changed). Role mei `~/w/s3`
(Skylake-SP); keke = Zen 1. Job scripts: mei and keke `~/work/RELEASE-SNAPSHOT-1119/`, made by `copy.sh` + `subst.sed`
from `~/work/RELEASE-SNAPSHOT-1118B-2/` (head, epoch, serial 20261119.1, role s3, labels, staging dirs; pkgdiff now
against snapshot-20261118.2; inside.sh's coexist version `0.0~snapshot20261119+fb63eeec883d`, equal to the staged
names). New this run: `y10.sh` + `y10/` (dyn.sh's recipe; #10 and #18). tree/ = local `git archive` of fb63eeec8.

**Landed since snapshot-2026-11-18.2** (`git log --oneline 557dfdaa6..fb63eeec8`: 84 lines; first-parent merges):
release-snapshot-1118b-2 (handoff only), **sl-headers-s17-3** (headers slice 17, generated in the build tree),
**dl-canonical-plt-7** (GitHub #10), **strptime-signed-year** (GitHub #18), two train view regenerations.
`git merge-base --is-ancestor` origin/task/{dl-canonical-plt-7,strptime-signed-year,sl-headers-s17-3} fb63eeec8: all yes.

## Results against snapshot-2026-11-18.2 (all on fb63eeec8's products; mei unless noted)

| check | job | result | vs 18.2 |
|---|---|---|---|
| smoke lua_reload, cxx_dlclose | -smoke-mei-1 (builds products), keke -smoke-keke-1 (mei's products) | `SMOKE: RESULT bad=0` both hosts; libc.so.6 sha256 084b6d8c..., ld.so c0589ffb... | same (new build) |
| gate | -gate-1 rc 0 | clippy --all-features -D warnings, xtask clippy, static, records check, affected --base origin/next: all RC 0 | same |
| system debs, check-system, check-symbols | -debs-1 | `check: PASS (7 packages, version 2.39+niji20261119.1-1)`; `check-symbols: PASS (4438 symbol lines, 13 libraries)` | same |
| coexist debs + check | debs-1 | builds (prefixed interpreter trace passes); `check: PASS (2 packages)` | same |
| debs-1 rc 1 | debs-1 | DBG only: 17 of 19 ELF objects; no libc6-dbg file for `/usr/bin/iconv`, `/usr/bin/locale` (ruling #6321) | same |
| package comparison | keke -pkgdiff-1 rc 0 | 9 packages, file counts unchanged (libc6 75, libc-bin 22, libc6-dev 38, libc6-dbg 44, niji6 29, niji6-dev 48, placeholders 7); libc6-dbg 5 build-ids replaced; sizes within 17 KB; control fields only versions | same |
| empty root | -roots-1 rc 0 | `root: RESULT pass=19 fail=0` | same |
| swapped trixie root | roots-1 | `verdict: commands=65 same=63 known=2 bad=0` (ldd-6/-7 loader trace output, D-31.15) | same |
| libc6-dbg + gdb | roots-1 | `dbg: RESULT pass=5 fail=0 pending=2` trixie and excalibur | same |
| fresh install, `ldconfig -p` | roots-1 | `fresh: RESULT fail=0` | same |
| lintian trixie / excalibur | -lint-1 rc 0 | 10 E / 60 W each; normalised tag lists (build-ids, versions masked) identical to 18.2's, 70 lines each | same |
| coexist proofs | lint-1 | `proof: RESULT suite=trixie pass=21 fail=0`, excalibur `pass=21 fail=0` | same |
| #9 many objects | -many-1 rc 0 | `YOHANE-MANY-OBJECTS pass=9 fail=0` | same |
| #16 rerun | -yohane-1 rc 1 | 16 PASS / 2 FAIL; PASS/FAIL lines identical to 18.2's run (the documented header gap) | same |
| #22 | -dyn22-1 rc 1 | verdict, class and FAIL lines identical to 18.2's run with addresses masked (`dyn: RESULT stock fail=0`, `niji fail=9`: the 9 probe-side lines explained in RELEASE-SNAPSHOT-1118B.md) | same |
| #10 + #18 | -y10-2 rc 1 | below: 19 PASS, 1 FAIL (bare-name dlopen, pre-existing) | fixed (18.2 fails, -y10-ref182-1) |
| stage | keke -stage-1 rc 0 | 9 assets + SHA256SUMS + MANIFEST; `sha256sum -c` 9 OK | - |
| install instructions | keke -install-1 rc 0 | `inst: RESULT fails=0`; staged system debs identical to debs-1's (7 of 7) | same |
| plants | -plants-1 | handoff/RELEASE-SNAPSHOT-1119-plants.txt (18B-2's six, paths/ids/version changed); result in the report json | - |
| docs | local doccheck.py on the tree | 23 problems, the same 23 as 18.2 (two differences-from-glibc.md line numbers shift by one) | same |

Superseded: -y10-1 (first probe version: post-swap rebuild needs C headers the system flavour does not ship, a
documented limitation; dl.c stopped at the bare-name dlopen). Not a product result.

## #10 and #18 end to end (swapped trixie root, gcc 14.2.0-19, GNU ld 2.44; probes built on the stock root)

Probes: the issue's shape (`lib.c` `-shared -fPIC` defines `f`; a second library takes `&f`; `main.c` compares `&f`
with both) as `-no-pie -fno-pic` and as `-pie -fPIE`; the same with `dlopen` + `dlsym`; binutils 2.44's own ld tests
(Debian `binutils_2.44.orig.tar.xz`, sha256 c41a0272... as in `binutils_2.44-3.dsc`; only those members extracted)
`pr19031` (no PIE), `pr22842` (PIE), `indirect-extern-access-2a` (no PIE) / `-2b` (PIE), built with the
`x86-64.exp` / `linux-x86.exp` "Run" recipes; `tp.c` for #18. `main-nopie`, `dl-nopie`, `pr19031`, `iea-2a` hold
canonical PLT entries (`f` / `indirect_extern_access` UND with a non-zero value in `--dyn-syms`, e.g. dl-nopie's
`f` 0x401060); `pr22842` (PIE) has `test` UND with value 0x1040.

| probe | stock | libniji fb63eeec8 (y10-2) | libniji 18.2 (y10-ref182-1) |
|---|---|---|---|
| main-nopie | PASS | PASS | FAIL `&f != lib_f(), &f != lib2_f()` |
| main-pie | PASS | PASS | PASS |
| dl-nopie (dlopen by path) | PASS `&f == lib_f()`, `== dlsym(RTLD_DEFAULT)`; dlsym(handle) != &f | same | FAIL `&f != lib_f(), != dlsym(RTLD_DEFAULT)` |
| dl-pie | PASS, dlsym(handle) == &f | same | PASS |
| pr19031 | OK | OK | (no output) |
| pr22842 | PASS | PASS | Aborted, rc 134 |
| iea-2a / iea-2b | PASS / PASS | PASS / PASS | (no output) / PASS |
| tp `"-1-1-1"` `%F`, `"+2024"` `%Y`, `"-1"` `%s` | NULL, NULL, NULL | NULL, NULL, NULL | consumed 6 (year -1901), 5, ... |
| tp `"+0200"` / `"-0130"` `%z`, `"2024-01-02"` `%F` | consumed 5 (gmtoff 7200 / -5400), 10 | identical lines | - |

libniji's lines equal the stock run's except one, the only FAIL of y10-2:
`bare-name dlopen: libt10.so (loaded) ok, libt10b.so (not loaded) ok` (stock) vs `fails, fails` (libniji) in
dl-nopie and dl-pie: the executable's DT_RUNPATH `$ORIGIN` holds both libraries; dlopen(3) searches the caller's
DT_RUNPATH. Same on 18.2's packages, so not introduced by this head. The dlsym comparison of #10 was made through a
full-path dlopen and passes.

## #6 / #21 (`git merge-base --is-ancestor`)

No fix landed since 557dfdaa6: no origin/task/heap-address-space{,-2..-7} or iconv-name-match{,-2,-3} tip is an
ancestor of fb63eeec8.

## INFORMATIONAL

* Loader: a bare-name `dlopen` from an executable whose DT_RUNPATH is `$ORIGIN` fails for a library in that
  directory, loaded or not (dlopen(3), docs/RELEASE-NOTES.md "the same search as ... dlopen ... DT_RUNPATH"); present
  in 18.2 too. Route to a loader story; not blocking (no change since 18.2, outside #10/#18).
* `dlsym(handle, "f")` through a handle of the defining library, in a no-PIE executable holding a canonical PLT
  entry for `f`, returns an address other than `&f` (`&f`, `lib_f()` and `dlsym(RTLD_DEFAULT)` agree); in the PIE
  it equals `&f`. Stock and libniji give the same relations; recorded so nobody expects otherwise.
* SL-HEADERS-S17-3 is not user-visible in the packages: libc6-dev / niji6-dev ship no headers (38 / 48 files,
  unchanged); the 16 slice 17 headers exist only in the build tree (`target/niji/include`).
* docs (route to a docs story): docs/RELEASE-NOTES.md still ends at 2026-11-17 (nothing on 18.1, 18.2, 19.1).
  User-visible since 18.2: #10 "a function's address is the same in an executable and every library when the
  executable holds a canonical PLT entry for it"; #18 "strptime's number conversions take no sign; %z keeps its
  sign" (the D-23.7 difference row is gone from docs/differences-from-glibc.md; no strptime sign row remains).
  install-packages.md install and verify: pass against the staged assets.
* products.tgz is 27 MB (18.2's 61 MB): the 18.2 role held stale `target/niji/rust*` rlibs from an older build; this
  role's target was fresh. Products themselves: +16 slice 17 headers in `target/niji/include`.

## Staged assets (keke `~/release-staging/snapshot-20261119.1/`, private)

libc6, libc6-dbg, libc6-dev, libc-bin, libc-dev-bin (`_2.39+niji20261119.1-1_amd64.deb`), libc-l10n, locales
(`_2.39+niji20261119.1-1_all.deb`), niji6, niji6-dev (`_0.0.snapshot20261119+fb63eeec883d_amd64.deb`), SHA256SUMS,
MANIFEST. Nothing to GitHub, no tag.

## Exact next step

Coordinator: tag/publish snapshot-2026-11-19.1 from the staged directory at fb63eeec8 when the owner says so. Not to
redo: anything above. Plants file: handoff/RELEASE-SNAPSHOT-1119-plants.txt.
