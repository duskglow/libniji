# PKG-COEXIST-SERIAL

Branch task/pkg-coexist-serial. Code commit f3ff26171 (the head after it only adds this note and the report).

## Premise
The brief asked for the coexist version `0.0~snapshotYYYYMMDD.<n>+<commit>`. It was already on next
(DEB-COEXIST-SERIAL: tools/deb/version.sh, build.sh comment, docs/install-packages.md,
crates/core/tests/deb_version_order.rs with plants in handoff/DEB-COEXIST-SERIAL-plants.txt). No change was needed there; this
run only verified it.

## Done
- libc-bin's description now uses `$programs` (ldconfig, zic, getconf, getent, iconv, ldd, locale, localedef, zdump) and no longer
  says localedef is absent. This is the expected byte change in libc-bin's control file.
- Descriptions checked against the manifest rows each package ships (system flavour; coexist for niji6/niji6-dev):
  - libc6: loader, libc/libm/libmvec/libdl/libpthread/librt/libresolv/libutil .so files present: text true.
  - libc6-dev: libc.so, libm.so, start files, static archives, no usr/include rows: "headers not part of this build" true.
  - libc-dev-bin: no gencat row: true. libc-l10n: only docs: true. libc6-dbg: debug rows only: true.
  - niji6, niji6-dev: sysroot, niji-gcc, niji-g++ present: true.
  - locales: shipped charmaps and localedef is in libc-bin, so "locale compiler not part of this build" was stale: reworded
    (a second byte change, in locales' control file).
- New test `a_package_that_ships_programs_lists_them_in_its_description` (deb_manifest.rs): every system-flavour package with
  usr/bin or usr/sbin files names them with `$programs`.

## Results
- Gate (affected, base origin/next): pkg-coexist-serial-gate: 1 suite, 6 tests passed.
- Plants: pkg-coexist-serial-plants: 2 CAUGHT (serial dropped from version.sh; description hand-written), 0 warnings.
- pkg-coexist-serial-build2 (job script, scratch): coexist build twice with NIJI_DEB_SERIAL=4: names `0.0~snapshot20261123.4+f3ff26171e7f`,
  `check.sh` PASS, the two builds' deb sha256 identical; dpkg --compare-versions for the brief's gt/lt cases ok.
- clippy and `cargo xtask static` clean on the role.

## Informational
- The build logged: `layout: verdict cache: could not store ... (an ok line holds a newline); the check passed`. Not mine; unexamined.

## Next
Nothing for this story.

## PKG-COEXIST-SERIAL-2 (continuation)
The CI red (deb_manifest tables_are_sorted_well_formed_and_name_built_products, 757258c50) said: product gencat is in the CI role's target/niji and no file row ships it. No source on next builds gencat (xtask/src/so.rs builds no gencat; the only gencat mentions are tests, probes and records), so the CI role's product directory held a leftover from another story's build, not an output of this branch. On a fresh `cargo xtask so` of this head all 6 deb_manifest tests pass (shiki, label pkg-coexist-serial-2-debtest). No code change was needed or made. When a utilities story starts building gencat it must add its libc-dev-bin file row and drop the placeholder text in tools/deb/description/libc-dev-bin.
Informational: the lint reads whatever is in target/niji, so a stale product from an earlier build on the same role turns this test red; xtask so does not remove products it no longer builds.
