# SYS-CONTROL-PARITY: package control fields match what Debian builds depend on

State: branch `task/sys-control-parity` (from origin/next 9e93a44b0), code commit fb23a22ec; this note and
the report json on top (handoff/ only). Brief: `~/state/briefs/SYS-CONTROL-PARITY.md`; source of the work:
plan/package-parity-audit.md rows 7, 8 and 25 (`results/package-parity/`).

## What changed (fb23a22ec)

* `tools/deb/system.sh`: libc6-dev `Depends` adds `linux-libc-dev, libcrypt-dev, rpcsvc-proto`
  (ref-control.tsv:165); `Provides: libc-dev (= <version>)` (was unversioned; ours-control.tsv:81);
  libc-l10n `Multi-Arch: foreign` (ref-control.tsv:69); Priority as the reference: libc6 `optional` (was
  `required`), locales and libc-l10n `standard` (were `optional`); libc-bin and the dev packages unchanged
  (already equal).
* Conffiles (item 3), settled from the spec and the code: libniji reads neither file, so libc-bin ships
  neither. `/etc/bindresvport.blacklist`: spec 31.14 and D-31.5 say it is not installed (Sun RPC is not
  implemented). `/etc/default/nss`: no clause in the spec, and no product code under crates/ or csys/ opens a
  file under `/etc/default`. The two `conffiles` gap rows stay, by design. `/etc/locale.alias` is
  SL-LOCALEDEF's.
* `tools/deb/check-system.sh`: checks each field above (Depends and Provides exactly, Priority on all five
  non-Essential packages, libc-l10n Multi-Arch); the exact conffiles check already rejects an extra
  conffile.
* `tools/deb/root-system.sh`: the empty-root proof now builds three empty stand-in packages
  (`linux-libc-dev`, `libcrypt-dev`, `rpcsvc-proto`, version `0~niji-standin`) and installs them with the six,
  so dpkg configures libc6-dev without `--force-depends`; the count is 9; purge removes the stand-ins too.
* `tools/deb/parity.py compare`: in relation fields each side's own Version reads as `<version>`, so
  `libc6 (= <own version>)` / `libc-dev (= <own version>)` on both sides are no longer two gap rows.
* `docs/install-packages.md`: the package table names the new fields.

No decision record: the fields follow the reference packages (no deviation, no new interface).

## Proof (keke, role n3; jobs on code head fb23a22ec)

* `sys-control-parity-lint` rc 0: `cargo xtask clippy`, full `cargo clippy --workspace --all-targets
  --all-features -D warnings`, `cargo xtask static`.
* `sys-control-parity-gate` rc 0: `cargo xtask affected --run --base origin/next`; `affected: ok`, 0 SKIPPED
  lines (the change is tools/deb and docs; no Rust test reaches tools/deb, so the packaging proof is the pkg job).
* `sys-control-parity-pkg` rc 0 (`~/work/SYS-CONTROL-PARITY/pkg.sh`, RAM dir /dev/shm/claude/SYS-CONTROL-PARITY/pkg-1):
  both flavours built from the role tree; `check.sh` PASS (2 packages); `check-system.sh` PASS (6 packages,
  2.39+niji20261116.1-1); `root-system.sh` 15 PASS / 0 FAIL (six packages + three stand-ins installed, no
  trigger pending, hello/threads/C++ programs run, purge leaves the empty root).
* Parity rerun (same job; `parity.py inventory` of the new system packages + `compare` against the committed
  reference inventory; the network steps `index`/`fetch-ref`/`rank` were not rerun, the reference is
  unchanged): gaps.tsv 3028 -> 3012 rows. Gone: libc6-dev Depends linux-libc-dev, libcrypt-dev, rpcsvc-proto;
  libc6-dev Provides (both rows); libc6-dev Depends libc6/libc-dev-bin (= version) (4 version-only rows);
  libc6, locales, libc-l10n Priority (6 rows); libc-l10n Multi-Arch. Changed, not new: libc-bin and libc-dev-bin
  `Depends: libc6 (= <version>)` (ours only; the reference states `(>> 2.41), (<< 2.42)` there: a real
  difference, left as is). No new row. results/package-parity/ is the audit's snapshot and was not rewritten.
* `sys-control-parity-plants` rc 0: handoff/SYS-CONTROL-PARITY-plants.txt, 9 plants on system.sh (each new
  Depends name, the unversioned Provides, libc-l10n Multi-Arch, the three Priority values, an extra
  bindresvport.blacklist conffile): 9 CAUGHT by check-system.sh with the named FAIL line, no WARN lines.
* `apt-get build-dep` probe (evidence, not a gate job; `~/work/SYS-CONTROL-PARITY/builddep.sh` + `bd-inner.sh`,
  results /dev/shm/claude/SYS-CONTROL-PARITY/bd-4): a trixie root (mmdebstrap, http mirror, signatures
  checked with the keyring), libniji's libc6 and libc-bin swapped in with dpkg, then `apt-get install` of
  libniji's libc6-dev and libc-dev-bin (local files) installed linux-libc-dev 6.12.107-1, libcrypt-dev
  1:4.4.38-1 and rpcsvc-proto 1.4.3-1 from the archive (none installed before); `apt-get build-dep hello`
  (Build-Depends: debhelper-compat, texinfo; no linux-libc-dev) resolved with libniji's libc6-dev as the libc-dev
  and installed build-essential and gcc; `/usr/bin/rpcgen` present.
  Limits, all from the probe root, not libniji: (1) the root leaves out util-linux, whose
  `PreDepends: libc6 (>= 2.41)` makes apt refuse every install once libc6 is 2.39 (the version-level gap of
  SYS-COMPAT-VERSIONS; runs builddep2 showed it in the stock `--variant=apt` root); so build-dep ended rc 100
  because tex-common's and libxml-sax-perl's postinst call `getopt` (util-linux): "getopt: command not
  found", exit 127. (2) `gcc -E` of `<crypt.h>` stops at `sys/cdefs.h`: libniji's libc6-dev ships no headers
  yet (SL-HEADERS). Superseded probe runs: builddep (no keyring at the default path), builddep2 (stock root:
  apt refuses; https in the chroot failed certificate checks), builddep3 (custom root without libc-bin).

## Open / next

* Nothing open for this story. Remaining rows of audit item 25 (libc6 `Depends: libgcc-s1`, `Recommends`,
  `Suggests`, debconf members, `Breaks` lists) were not in this story's scope.
