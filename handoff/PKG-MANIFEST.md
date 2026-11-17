# PKG-MANIFEST: package contents as data (checkpoint 1, WIP)

State: branch `task/pkg-manifest`, cut from origin/next 59c31ec19 (contains SYS-CONTROL-PARITY-2 and
SYS-DEV-LINKS-2). Role keke `~/w/n3` (not synced yet, no jobs run). Brief: `~/state/briefs/PKG-MANIFEST.md`.
Stopped at the context checkpoint (~150k) after the data and tool step; nothing is wired into the build yet.

## Done (WIP commit)

* `tools/deb/manifest.tsv`: 80 sorted rows, both flavours (flavour both/coexist/system, package, path,
  kind file/text/gzip/link/changelog/dir, source, mode, strip none/unneeded/debug/archive, note). A
  `both` row names libc6/libc6-dev; coexist maps it to niji6/niji6-dev under `usr/lib/niji/`. Rows
  transcribe build.sh + system.sh exactly (iconv is strip `unneeded`: today it gets no libc6-dbg file).
* `tools/deb/control.tsv` (package, field, value; sorted; `$version $abi $provides` placeholders; member
  rows `triggers`, `symbols`, `debug`) and `tools/deb/description/<pkg>` (9 files, `$abi`, `$programs`).
* `tools/deb/system/libc.so`, `tools/deb/coexist/libc.so`: the two linker scripts, byte-equal to today's
  put_text output.
* `tools/deb/manifest.py`: lint [--products DIR], rows, packages, debug-package, symbols-package,
  control, member (conffiles/triggers), check FLAVOUR DEBDIR (exact path set, types, modes, owners,
  link targets, text contents, strip state via readelf, control members, control text, Installed-Size,
  md5sums). Smoke-tested locally (lint OK; control output matches today's libc-bin text except the
  generated `$programs` order: "getconf, getent, iconv, ldconfig, ldd", was "ldconfig, getconf, ...").

## Next step (exact)

1. build.sh: shared helpers `install_rows` (loop over `manifest.py rows $flavour`: file -> need+put,
   text -> put from tools/deb, gzip -> gzip -9n, link -> put_link, changelog -> per-flavour printf|gzip,
   dir -> mkdir), `strip_rows` (unneeded/debug: `strip --strip-unneeded --remove-section=.comment
   --remove-section=.note`; archive: `strip --strip-debug --remove-section=.comment
   --enable-deterministic-archives`), `write_control PKG` (manifest.py control + md5sums + member
   conffiles/triggers), finish (chmod DEBIAN 0644 then the DEBIAN rows' modes, dirs 0755, touch,
   dpkg-deb per `manifest.py packages`). Coexist uses them; drop put_text.
2. system.sh: install_rows; debug split for strip=debug rows into `manifest.py debug-package system`
   (skip with --symbols-only); strip_rows; symbols.py for `symbols-package`; its control first, exit on
   --symbols-only; other controls; finish.
3. check-system.sh / check.sh: call `manifest.py check <flavour> DIR` (count FAIL lines); drop the
   per-package expect/ctrl/conffiles/triggers/path-case blocks; SONAME, dev-link-resolves and dbg loops
   from `manifest.py rows`. root-system.sh: conffiles list and package count from the manifest.
4. `crates/core/tests/deb_manifest.rs`: runs `manifest.py lint --products target/niji` (names the three
   data files as paths so `affected` selects it). Not in `cargo xtask static`: main.rs/so.rs edits
   escalate affected to FULL; the brief allows "the packaging check's own lint".
5. Proof on keke: products for both flavours once (build.sh full mode, RAM work dir), package with base
   (`git archive origin/next tools/deb`) and head scripts via --products, `cmp` each .deb, `dpkg-deb -c/-I`
   diffs (expected: libc-bin Description line only); checks, root-system.sh, parity inventory; plants
   (row removed vs prebuilt debs, unlisted file shipped, wrong mode, unsorted, extra row); merge-tree demo.

## Not to redo / notes

* xtask has no single product list (names spread over so.rs, placeholders.rs, libmvec.rs); lint checks
  file rows against the products directory instead.
* Process slip this run: one heredoc (`python3 - <<'X'`) used for a local patch; no effect on the tree.
