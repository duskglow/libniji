COMPLETE: snapshot-ready at e4010743214ea636aae0784fb8c36806ca4ffad7 (no blocking finding; mei smoke round refused, see Refusals)

# RELEASE-SNAPSHOT-MVEC: snapshot with libmvec.so.1 and libc6-dbg, validation of e40107432

**State:** head to tag `e4010743214ea636aae0784fb8c36806ca4ffad7` "Merge task/sys-dbg-package-3 into next"
(origin/next at the start, 2026-11-17 07:09:14 UTC, epoch 1794899354). Branch `task/release-snapshot-mvec` = that
head + handoff/ only (plants file, this note, the report json); no product or packaging change. `git log --oneline
30d07e74b..e40107432`: 39 lines, 32 non-merge commits (SYS-LIBMVEC-2/-3, SYS-DBG-PACKAGE(-2/-3), MATH-POW-EDGES,
PROBE-NIJI-TREE, RELEASE-SNAPSHOT-NIJI1-2's handoff, view regeneration). Role keke `~/w/n3`; job scripts and logs in
keke `~/work/RELEASE-SNAPSHOT-MVEC/` (NIJI1's debs.sh, gate.sh, smoke.sh, chain-*.sh, validate.sh (lint part),
pkgdiff.sh and NIJI1-2's roots.sh, swapverdict.sh (unchanged), stage.sh, install.sh, inside.sh with head, epoch,
serial 2, role and labels changed; new: debscheck.sh, fresh.sh, mvec.sh + m/ for this head's expected changes;
tree/ = `git archive e40107432`).

## Results vs snapshot-2026-11-17 (keke, all on e40107432's products)

| check | job | result | vs 11-17 |
|---|---|---|---|
| system + coexist builds | debs-1 | 7 + 2 packages, `2.39+niji20261117.2-1` / `0.0~snapshot20261117+e4010743214e`; builds, check-system, check-symbols, check.sh rc 0 | +libc6-dbg |
| check-system / check-symbols / coexist check | debscheck-2 | `check: PASS (7 packages, version 2.39+niji20261117.2-1)`; `check-symbols: PASS (4435 symbol lines, 13 libraries)`; `check: PASS (2 packages)` | 6 -> 7 packages; +218 lines, +libmvec.so.1 |
| package contents | pkgdiff-1 | only: libmvec.so.1 in libc6 and niji6; libmvec.a + libmvec.so in libc6-dev and niji6-dev; new libc6-dbg (43 paths, 17 .debug files); version strings | each traced to SYS-LIBMVEC / SYS-DBG-PACKAGE |
| empty root | roots-1 | `root: RESULT pass=19 fail=0` (now installs libc6-dbg too: 7 packages + 3 stand-ins) | same |
| swapped trixie root | roots-1 | `swap: RESULT same=56 differ=7`; `verdict: commands=63 same=56 known=7 bad=0` | identical (ldd-1..7 still DL-TRACE-INTERP's known difference) |
| libc6-dbg + gdb, trixie and excalibur | roots-1 | `dbg: RESULT pass=5 fail=0 pending=2` each: no line before, `dpkg -i libc6-dbg` rc 0, then `Line 102 of "boundary/src/export.rs"` for setlocale; the running-program stop is PENDING (loader rendezvous, #6151) | new |
| ldconfig -p after a fresh install | roots-1 (fresh.sh) | `libmvec.so.1 (libc6,x86-64) => /lib/x86_64-linux-gnu/libmvec.so.1`; libc6-dbg beside: 17 installed objects have their .debug file (same build-id) | new |
| build-id resolution | debscheck-2 | `DBG: 17 of 17 ELF objects resolve` (readelf -n of each stripped object = its libc6-dbg file's); no `.gnu_debuglink` (system.sh:44, by design); `.debug_line` in 5 of 17 | new |
| lintian trixie / excalibur | lint-1 | 10 E / 57 W each | 10 E / 40 W: +17 W, below |
| coexist proofs | lint-1 | trixie and excalibur `pass=21 fail=0` each | same |
| lua_reload, cxx_dlclose | smoke-keke-1 | ok 1/1 both sides (libc.so.6 sha256 643b6e68...) | keke same; **mei not run (refused)** |
| gate | gate-1 | clippy --all-features -D warnings, xtask clippy, static, records check, affected --base origin/next (0 suites: no diff): rc 0 | - |
| plants | plants-1 | 6 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY/NO-MSG (handoff/RELEASE-SNAPSHOT-MVEC-plants.txt, NIJI1-2's six on this head) | same as 11-17 |
| 4(b) install | install-1 | 24 ok, 0 FAIL; staged system .debs identical to debs-1's (7 of 7) | same (6 -> 7 debs) |

**Sizes:** largest package now libc6-dbg 8,631,332 bytes (new). Largest before: libc6-dev 3,001,884 -> 3,027,384
(+libmvec.a). libc6 2,693,420 -> 2,701,088; niji6 2,677,140 -> 2,685,964; niji6-dev 2,999,564 -> 3,034,248.

**Lintian +17 W per suite (E set identical):** libc6-dbg: `copyright-without-copyright-notice`,
`debian-changelog-line-too-long`, `initial-upload-closes-no-bugs` (same kinds as the other packages' first upload), and
12 x `debug-file-with-no-debug-symbols` (the 12 debug files without DWARF: libm, libmvec, libpthread, libdl, librt,
libutil, libanl, libnsl, libresolv, libBrokenLocale, libc_malloc_debug, getent; they carry the symbol table only);
libc6: `extended-description-line-too-long line 4` and `undeclared-elf-prerequisites (libc.so.6 libm.so.6)
[libmvec.so.1]` (libc6's override covers only libc.so.6's own prerequisite).

## The Yohane fix (yohane-2, swapped trixie root, Debian's own binaries, no rebuild)

`yohane: YOHANE-MVEC pass=14 fail=0`. libniji's libc6, libc-bin, libc6-dev, libc-dev-bin swapped in (dpkg rc 0), the
root's libc.so.6, libm.so.6, libmvec.so.1 and loader are the package's (sha256) after the install; `ldconfig -p`:
`libmvec.so.1 (libc6,x86-64) => /lib/x86_64-linux-gnu/libmvec.so.1`.
* **`apt-get install -y graphviz` in the swapped root is refused by apt (rc 100), not by libmvec:** `util-linux :
  PreDepends: libc6 (>= 2.41) but 2.39+niji20261117.2-1 is to be installed`. The forced swap into trixie (glibc 2.41)
  leaves that relation unmet and apt then installs nothing (the known "swap needs --force-depends" state of earlier
  snapshots; a Yohane root built against libniji has no such relation). So apt downloaded graphviz and the 69 .debs it
  lacks on the stock root (no C library package among them), and `dpkg -i` installed them on the swapped root: rc 0,
  graphviz `install ok installed` (postinst `dot -c` ran on libniji), `dpkg --audit` clean.
* `dot -V`: `dot - graphviz version 2.42.4 (0)`, rc 0; `dot -c` rc 0; `dot -Tsvg` renders (3 nodes).
* NEEDED (readelf -d): `/usr/lib/x86_64-linux-gnu/libgvc.so.6.0.0 (libgvc6): 0x0000000000000001 (NEEDED) Shared
  library: [libmvec.so.1]`; also libpathplan.so.4.0.0 (libpathplan4) and graphviz/libgvplugin_neato_layout.so.6.0.0;
  `ldd` resolves `libmvec.so.1 => /lib/x86_64-linux-gnu/libmvec.so.1`.
* C program (m/mvec.c, compiled on the stock root, linked after the swap against libniji's libc6-dev with `-lmvec -lm`,
  dynamic: NEEDED libmvec.so.1, libm.so.6, libc.so.6; and `-static`): `_ZGVbN2v_cos` and `_ZGVdN4v_cos` on 0, 0.5, 1,
  -2, pi, 10, 1e6, -0.75: every lane bit-identical to scalar cos (e.g. `cos(0.5) = 0.87758256189037276
  (0x3fec1528065b7d50)`), both builds `mvec: all lanes equal the scalar cos`.

## Documentation currency

(a) README.md + docs/ (53 files, doccheck.py of RELEASE-SNAPSHOT-CXX-DOCS): 238 links, 0 broken; 197 `cargo xtask`
mentions, 0 unknown; one new missing path since 11-17: differences-from-glibc.md:506 `records/pending/
SYS-LIBMVEC-2-1.toml` (assigned: records/decisions/D-10/D-10.124.toml). The rest as at 11-17 (saturation views,
:307 SL-HEADERS pending path); saturation/15 `bench/c/--only.c` quotes an error message, not a reference.
(b) **pass** (install-1): docs/install-packages.md followed literally in a fresh trixie root against the staged
assets (NIJI1-2's install.sh/inside.sh, version changed): `sha256sum -c SHA256SUMS`, published names (`.` for `~`),
`apt install ./niji6_... ./niji6-dev_...`, the niji-gcc/niji-g++ builds run, purge leaves the package set, /etc and
/usr as before: 24 ok, 0 FAIL; the 7 staged system .debs (libc6-dbg included) byte-identical to debs-1's. The page
gives no install command for libc6-dbg (system flavour, table row only); it installs with `dpkg -i` after libc6 in
root-system, dbg-system (trixie, excalibur) and fresh.sh.
(c) changes since snapshot-2026-11-17 and their sentence:
* libmvec.so.1 / libmvec.a / libmvec.so (SYS-LIBMVEC): differences-from-glibc.md:502-506 (true: 216 exports, each
  lane the scalar result, checked above). **Stale:** install-packages.md:13 (niji6's library list), :14 (niji6-dev),
  libc6 row ("as in niji6") and :124 (libc6-dev) do not list libmvec.so.1, libmvec.a, libmvec.so.
* libc6-dbg (SYS-DBG-PACKAGE): install-packages.md:126 says what it is (where a system-flavour installer looks).
  **Missing:** that it is optional; **partly false:** "symbol table and line tables of every shared object ...": 12 of
  17 debug files have no line tables. No sentence for the coexist flavour (there is no niji6 debug package).
* line tables in the release products (c9378a18c): only visible through libc6-dbg; none needed.
* MATH-POW-EDGES (pow sNaN -> quiet NaN, pow(-1, huge even) = 1): no difference remains; none needed.
* PROBE-NIJI-TREE: developer tool; none needed. No docs/RELEASE-NOTES.md section for this snapshot (coordinator).

## Staged (keke `~/release-staging/snapshot-20261117.2/`, serial 2; stage-1; `sha256sum -c` 9 OK)

| asset | bytes | sha256 (first 20) |
|---|---|---|
| libc6_2.39+niji20261117.2-1_amd64.deb | 2701088 | fcad206b0a7698f0c2cd |
| libc6-dev_2.39+niji20261117.2-1_amd64.deb | 3027384 | 18bfac34b24d2d237c98 |
| libc6-dbg_2.39+niji20261117.2-1_amd64.deb | 8631332 | f708ee73864967880dfe |
| libc-bin_2.39+niji20261117.2-1_amd64.deb | 372328 | 359c7bbd51ee520cffdd |
| libc-dev-bin_2.39+niji20261117.2-1_amd64.deb | 1952 | 62e9816c66fb580c6f08 |
| libc-l10n_2.39+niji20261117.2-1_all.deb | 1932 | bf8f4ff734d1c939f314 |
| locales_2.39+niji20261117.2-1_all.deb | 1932 | e4b22db46d4fdb64b1a5 |
| niji6_0.0.snapshot20261117+e4010743214e_amd64.deb | 2685964 | 2b18e0b916f60a24a9f9 |
| niji6-dev_0.0.snapshot20261117+e4010743214e_amd64.deb | 3034248 | bffe3771592e5f2c484a |

Plus SHA256SUMS and MANIFEST (private: names the build host). Nothing published, no tag.

## Jobs, superseded, slips

JOBS (keke, rc 0): release-snapshot-mvec-gate-1, -debscheck-2, -pkgdiff-1, -roots-1, -lint-1, -yohane-2,
-smoke-keke-1, -stage-1, -install-1, -plants-1. Evidence, not in JOBS: -debs-1 (rc 1: its builds and the three
package checks rc 0, and its .debs are the ones validated and staged; only my added DBG check failed, because it also
required `.gnu_debuglink`, which system.sh deliberately omits). Superseded: -debscheck-1 (rc 1: also required
`.debug_line` in every debug file), -yohane-1 (rc 1: apt's refusal above, before the download-then-dpkg route).
Rule slips (mine): one local heredoc (`python3 - <<'X'`) to derive debscheck.sh, and one `sed -i` on this note
(the plants row) instead of the Edit tool; no effect on results.

## Refusals

`"${VSSH[@]}" 'mkdir -p ~/work/RELEASE-SNAPSHOT-MVEC/tree' && git archive e4010743214ea636aae0784fb8c36806ca4ffad7 |
"${VSSH[@]}" 'tar xf - -C ~/work/RELEASE-SNAPSHOT-MVEC/tree && ...'` with VSSH set for mei (preparing mei's tree for
the mei smoke round): refused by the permission layer. Not retried: the mei lua_reload/cxx_dlclose round was not run.

## Next

Coordinator: tag e40107432 and publish `~/release-staging/snapshot-20261117.2/` (needs the mei smoke round, or a ruling
that keke's suffices). Docs story: the INFORMATIONAL sentences in (a) and (c). Packaging story: libc6's lintian override
for libmvec.so.1's prerequisites; the 12 debug files without DWARF (C layer `-g`, SYS-DBG-PACKAGE next step 2). Do not
redo: builds, checks, roots, lint, Yohane, staging.
