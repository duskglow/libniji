# SL-FLAVOUR: the system build and Liella's packages under Debian's C library names

**State:** branch `task/sl-flavour` (from origin/next `2d5625a3e`), code head `7c83f6980`; this note and
`handoff/reports/SL-FLAVOUR.json` on top. Role mei `~/w/s1`. Plan: plan/system-libc.md §3.1-3.3, §6 item 2;
owner rulings R1 (#5535), R2 (#5536). Pending record `records/pending/SL-FLAVOUR-1.toml` (D-NEW, series D-X,
amends D-X.94; CI assigns the id). Plants `handoff/SL-FLAVOUR-plants.txt`.

## What it does

* `tools/deb/build.sh --flavour system` (default `coexist` unchanged): the same tree built with `LIELLA_PREFIX=/`,
  `LIELLA_LIBDIR=usr/lib/x86_64-linux-gnu` (the `/` prefix builds as it stands: plan §3.3's NOT VERIFIED is settled),
  packaged by `tools/deb/system.sh` as libc6, libc-bin (Essential), libc6-dev (Provides libc-dev), libc-dev-bin,
  locales, libc-l10n; Source liella; version `2.39+liella<YYYYMMDD>.<n>-<rev>` (`LIELLA_DEB_SERIAL`,
  `LIELLA_DEB_REVISION`, default 1), no epoch. Conffiles: libc6 `/etc/ld.so.conf.d/x86_64-linux-gnu.conf`; libc-bin
  `/etc/ld.so.conf`, `/etc/ld.so.conf.d/libc.conf`, `/etc/gai.conf` (all written from the man pages and the multiarch
  paths). libc-bin postinst copies `/usr/share/libc-bin/nsswitch.conf` to `/etc/nsswitch.conf` when absent.
  Triggers: libc6 `activate-noawait ldconfig`, libc-bin `interest-await ldconfig`; **the handler is a documented
  no-op** until SL-LDCONFIG ships ldconfig. Scripts honour `DPKG_ROOT`.
* Not shipped (said in the packages and the record): headers (SL-HEADERS, R4), crti.o/crtn.o (SL-CRT), symbols/shlibs
  (SL-SYMBOLS), programs (SL-BIN-CORE, SL-LDCONFIG, SL-LOCALEDEF), `/etc/bindresvport.blacklist` (s31_14_c_1),
  `/etc/default/nss`. libc-dev-bin, locales and libc-l10n are documentation-only placeholders.
* `tools/deb/check-system.sh` (identity, fields, layout per package, control members, conffiles, triggers, loader
  prefix, SONAMEs, plain-gcc link with PT_INTERP `/lib64/ld-linux-x86-64.so.2`, static hello);
  `tools/deb/root-system.sh` (empty root, `unshare -r` + `dpkg --root --force-script-chrootless`, chroot runs);
  `tools/deb/lint.sh` (lintian in an mmdebstrap unshare chroot: keke and mei have no lintian).

## Results (mei, code head 7c83f6980)

* `sl-flavour-deb-3` rc 0: build (prefix /), repack of the same products **byte-identical**, `check: PASS (6
  packages, version 2.39+liella20261114.1-1)`; empty root **pass=8 fail=0 wait=1**: six packages installed, no trigger
  pending, conffiles registered, nsswitch.conf from the template, `dpkg --verify` clean, **static hello runs in the
  root**, `ld.so --list` in the root resolves `libc.so.6 => /usr/lib/x86_64-linux-gnu/libc.so.6`, purge leaves the
  root empty (and nsswitch.conf). WAIT: the dynamic hello starts the packaged loader through `/lib64` and stops at
  "the library search is not available in this build" (DL-8, plan §6 item 1).
* lintian 2.122.0 `--pedantic` in an excalibur chroot (LAN mirror, Devuan keyring; base-files
  13.8+deb13u7devuan1, so `Breaks: base-files (<< 13.3~)` is met): **17 E / 41 W**. E: `no-shlibs` x10 (libc6:
  shlibs/symbols are SL-SYMBOLS, plan §6 item 3; NOT fixed here), `no-code-sections` x6 (the empty archives, as
  RELEASE-PACKAGING-DEB), `shared-library-is-executable` (the loader). W seen: `lacks-unversioned-link-to-shared-library`
  (the .so links of placeholders: SL-SONAMES), `copyright-without-copyright-notice`, `debian-changelog-line-too-long`,
  `initial-upload-closes-no-bugs`, `non-multi-arch-lib-dir [usr/lib64/]` (the loader link, plan §3.3),
  `shared-library-lacks-prerequisites`, `undeclared-elf-prerequisites`; full list in mei
  `~/work/SL-FLAVOUR/logs3/lint-excalibur.log`.
* `sl-flavour-plants-2` rc 0: **8 CAUGHT** (lib64 link, Essential, Provides, conffile, trigger, conffile name, a
  libc6-dev file in libc6, postinst nsswitch), no WARN lines.
* `sl-flavour-gate-2` rc 0: cargo clippy `--all-features -D warnings`, `cargo xtask clippy`, `static`, `records
  check`, direct tier **397 passed, 0 failed**, peak 1.3G.
* Superseded: `sl-flavour-deb-1` (repack differed: `--products` left the commit out; fixed), `sl-flavour-deb-2` and
  `sl-flavour-plants-1` (check-system lacked base-files' /lib links and read `Essential: no` as unexpected; fixed),
  `sl-flavour-gate-1` (green, older head).

## Next

* SL-SYMBOLS adds `DEBIAN/shlibs` + `symbols` to libc6 (clears the 10 `no-shlibs` errors); check-system.sh's libc6
  control-member list then gains them. SL-LDCONFIG replaces the trigger no-op. Once DL-8 lands, root-system.sh's WAIT
  turns PASS with no change.
* Rule slips (mine): one Bash heredoc (`python3 - <<EOF`) for the first build.sh edit; read the reference's
  installed libc-bin.triggers file on mei (its comment text), copied nothing from it.
