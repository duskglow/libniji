# SYS-DEV-LINKS: libBrokenLocale.so.1, the missing development links and archives

State: branch `task/sys-dev-links-2` (SYS-DEV-LINKS' `task/sys-dev-links` e201bf454 with origin/next
21b03d08c merged in). HEAD: see `handoff/reports/SYS-DEV-LINKS-2.json`.

## SYS-DEV-LINKS-2: the merge of origin/next (CI conflict)

next (with SYS-CONTROL-PARITY-2) had landed libresolv.so.2 / libresolv.so in the system packages (issue
#5), the control-field parity (libc6-dev Depends/Provides) and the libc6 lintian override, in the same
four files. Resolution keeps both sides: libc6's library loop ends `... libresolv.so.2
libBrokenLocale.so.1`; libc6-dev has the `libresolv.so` link and this story's three links;
check-system.sh checks both SONAME lists, layout paths and both link blocks (devlinks, resolv);
root-system.sh runs devlinks and resolv in the root; docs/install-packages.md lists both (and no longer
says libc6 lacks libresolv.so.2). The `bl-lib` plant's literal followed the new loop text.
Proof on the merged head (role s1 on mei): `sys-dev-links-2-pkg` rc 0 (system check PASS, check-symbols
PASS 4214 lines, root-system all PASS incl. devlinks MB_CUR_MAX 4, resolv and krb5; coexist check.sh
PASS; niji6-dev links present; parity compare: new rows only next's libresolv rows (3 class-c private
or data symbols, its shlibs line) plus the expected `shlibs: libBrokenLocale 1 libc6 (>= 2.39)`),
`sys-dev-links-2-lint` (clippy all features, static) rc 0, gate `sys-dev-links-2-gate` rc 0
(`affected --run --base origin/next`: 4 suites, 463 passed / 0 failed, peak 2.0G), plants
`sys-dev-links-2-plants` rc 0 (6 CAUGHT, 0 WARN). The changes touch no start-up code, so the
start-up checks were not run. Superseded: none (`sys-dev-links-2-lint0` was step 0's check of the
inherited head, rc 0).
Version question (coordinator #6144/#6146): `libm-2.41.a` (audit row 13) is the reference's versioned
static libm name; libniji is the 2.39 ABI, so no 2.41-named file is shipped.
Brief: `~/state/briefs/SYS-DEV-LINKS.md` (gap 13 of `plan/package-parity-audit.md`).

## What changed

* `tools/deb/system.sh`: libc6 ships `libBrokenLocale.so.1` (the D-27.9 product, `__ctype_get_mb_cur_max`
  override). libc6-dev ships `libBrokenLocale.a` (the override), the empty `libg.a`, `libmcheck.a`,
  `libpthread_nonshared.a` (D-27.9's `ARCHIVES`), and the links `libanl.so -> libanl.so.1`,
  `libBrokenLocale.so -> libBrokenLocale.so.1`, `libc_malloc_debug.so -> libc_malloc_debug.so.0`.
* `tools/deb/build.sh` (niji6-dev): the `libanl.so` and `libc_malloc_debug.so` links too (it already had
  the libraries, archives and the `libBrokenLocale.so`/`libresolv.so` links).
* Proof: `tools/deb/smoke/devlinks.c` prints `MB_CUR_MAX` in C.UTF-8 (built-in locale): 4 when
  libBrokenLocale's override is linked, 6 (the C library's) otherwise. `check-system.sh`: the links name
  libc6's libraries and resolve, the archives are present, `libBrokenLocale.so.1`'s SONAME; devlinks links
  dynamically with `-Wl,--no-as-needed -lanl -lBrokenLocale -lmcheck -lg -lc_malloc_debug` (needs exactly
  libanl.so.1 libBrokenLocale.so.1 libc_malloc_debug.so.0 libc.so.6) and statically (prints 4; a control
  link without `-lBrokenLocale` prints 6). `root-system.sh`: the dynamic devlinks runs in the dpkg-installed
  root and prints 4. `check.sh` (niji6): niji-gcc links devlinks with the same `-l`s and each shared
  library is needed.
* `docs/install-packages.md`: the package tables list the new files.

## Results (mei, role s1)

* Packaging proof job (`~/work/SYS-DEV-LINKS/pkg2.sh`; both flavours built from the role tree, release
  profile): check-system PASS, check-symbols PASS, root-system 16 pass / 0 fail (dynamic devlinks
  prints `MB_CUR_MAX 4` in the root), check.sh PASS.
* `tools/deb/parity.py inventory` + `compare` on the new system packages against the committed reference
  inventory (the network fetch and the ranking scan were not rerun: they only change with the reference):
  closed: the 8 class-a rows (libBrokenLocale.so.1, libg.a, libmcheck.a, libBrokenLocale.a,
  libpthread_nonshared.a, libBrokenLocale.so, libanl.so, libc_malloc_debug.so), class-b
  libBrokenLocale.so.1, class-e `symbols: libBrokenLocale.so.1 libc6 #MINVER#`. New: only
  `e libc6 shlibs: libBrokenLocale 1 libc6 (>= 2.39)` (ours-only, the version-number shape every
  library's shlibs line already has, 2.39 against the reference's 2.41) and the symbols line count
  (4135 -> 4137). No new file, soname or symbol gap.
* Gate (direct tier, `cargo xtask affected --run --base origin/next`: niji::deb_symbols_products, xtask
  unit tests, static): `sys-dev-links-gate-3` rc 0, peak 1.4G.
* Plants (`sys-dev-links-plants-2`, `handoff/SYS-DEV-LINKS-plants.txt`): 6 plants on system.sh and
  build.sh, 6 CAUGHT, no WARN; each repackages the pkg job's products from the planted copy and runs
  the flavour's check.
* Superseded: `sys-dev-links-gate`, `sys-dev-links-pkg` (green, older head), `sys-dev-links-plants`
  (refused: pattern syntax), `sys-dev-links-gate-2`, `sys-dev-links-pkg-2` (stopped to resync).

## Open / not done

* `libnss_compat.so`, `libnss_hesiod.so` (in the audit's list) are NOT shipped: their libraries
  (`libnss_compat.so.2`, `libnss_hesiod.so.2`) are not built (audit gap 16, SL-NSS), so the link would
  dangle and `-lnss_compat` would fail with a dangling-link error instead of "not found". They belong
  with SL-NSS (compat); hesiod has no library planned. Their gap rows stay.
* `libm-2.41.a` (audit row 13, not in this brief's list) is not shipped: a version-named copy of libm.a.
* `libresolv.so.2` stays out of the system libc6 (SYS-LIBRESOLV-2 ships it).
* `results/package-parity/` is not regenerated (it is the audit's snapshot; the comparison above was
  written to a scratch copy).

## Next step

None for this story. SL-NSS: when `libnss_compat.so.2` lands, add it to libc6 and the
`libnss_compat.so` link to the loop in `system.sh`'s libc6-dev section (and to `check-system.sh`'s list).
