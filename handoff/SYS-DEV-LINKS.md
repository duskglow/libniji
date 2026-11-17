# SYS-DEV-LINKS: libBrokenLocale.so.1, the missing development links and archives

State: branch `task/sys-dev-links` (from `origin/next` cc18a11ef). HEAD: see `handoff/reports/SYS-DEV-LINKS.json`.
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
