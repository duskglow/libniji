# PKG-MANIFEST: package contents as data (checkpoint 2, PKG-MANIFEST-2, WIP)

State: branch `task/pkg-manifest-2` (continues `task/pkg-manifest`), cut from origin/next 59c31ec19.
Role keke `~/w/n2`. Brief: `~/state/briefs/PKG-MANIFEST.md` ("## The change", "## Proof") plus
`~/state/briefs/PKG-MANIFEST-2.md` (packages byte-identical to today, description program order as
today). Stopped at the context checkpoint (~145k) with the proof and gate jobs started.

## Done

* Run 1 (c1d051f0c): `tools/deb/manifest.tsv` (80 rows), `control.tsv`, `description/<pkg>`,
  `system|coexist/libc-so.ld`, `manifest.py` (lint, rows, packages, debug-package, symbols-package,
  control, member, check).
* Run 2 (5bb3eb4bb): build.sh helpers `install_rows`, `rows_with`, `strip_rows`, `write_control`,
  `finish` (both flavours; no per-file shell lines, `put_text` gone); system.sh: debug split over
  `rows_with debug` into `manifest.py debug-package`, symbols into `symbols-package`, controls from
  the tables. check-system.sh / check.sh run `manifest.py check <flavour>` (FAIL lines counted;
  rc != 0 with no FAIL line is a failure too); SONAME, dev-link-resolves and debug-file loops read
  `manifest.py rows`; the restated expect/ctrl/conffiles/triggers/path-case blocks are gone (R2
  version, Source and Maintainer identity kept). root-system.sh: package count and conffiles from
  the manifest (its purge line keeps a fixed package order: dpkg ordering, code not data).
  `$programs` = usr/sbin then usr/bin, each sorted: gives today's "ldconfig, getconf, getent,
  iconv, ldd" (checked locally). conffiles member sorted. New test `crates/core/tests/deb_manifest.rs`
  (lint --products target/niji; coexist build with --products then `manifest.py check coexist`);
  `deb_symbols_products.rs` STEP names the data files.
* Step 0 (inherited head e0b9ce0e7): clippy + static green (job `pkg-manifest-2-lint0`).
* Merge demo (local, plumbing commits, nothing pushed): two rows (libnss_files.so.2,
  libthread_db.so.1) added to the manifest on head merge clean (`git merge-tree --write-tree`);
  the same two added to the base's system.sh `for so in` line: CONFLICT in tools/deb/system.sh.

## Jobs started on 5bb3eb4bb (keke)

* `pkg-manifest-2-proof`: `~/work/PKG-MANIFEST-2/proof.sh` (copy in the session scratch): builds
  system and coexist products once (release profile, prefixes / and /usr/lib/niji) in
  `~/work/PKG-MANIFEST-2/proof/src`, packages each with the base scripts
  (`~/wl/src/59c31ec1.../tools/deb`) and the head scripts (`--products`, fixed SOURCE_DATE_EPOCH and
  NIJI_DEB_COMMIT), prints IDENTICAL/DIFFERENT per .deb (with `dpkg-deb -c/-I` diffs), then runs
  check-system.sh, check.sh, check-symbols.sh, root-system.sh and a parity inventory diff
  (base vs head; the network audit itself is not rerun: identical .debs give identical inventories).
* `pkg-manifest-2-gate`: `cargo xtask affected --run` in `~/w/n2` under `~/bin/gate 16G`.

## Next step (exact)

1. Read both logs (`~/bin/jobwait pkg-manifest-2-proof pkg-manifest-2-gate`, then grep `proof:` and
   `check:`/`root:` FAIL lines; the gate with `~/bin/test-summary`). Any DIFFERENT .deb means a
   manifest or helper bug: fix it, never the comparison.
2. Plants: finish `handoff/PKG-MANIFEST-plants.txt` (draft; check docs/plants.md on products for
   `cargo test -p niji --test deb_manifest` and `\t` in perl patterns), run through `cargo xtask
   plants` under plain jobrun with `--dir /dev/shm/claude/PKG-MANIFEST-2/plants-1` after the proof
   job (two plants read its base .debs).
3. Final clippy + static on the role, report json, `ops handback-check`.

## Not to redo / notes

* The libc6-dbg file set is unchanged: iconv is strip `unneeded` (SYS-DBG-DWARF adds its dbg file
  after this lands).
* xtask has no single product list; lint checks file rows against the products directory.
* Process slips: one empty no-op heredoc in run 2 (no effect), one heredoc in run 1.
