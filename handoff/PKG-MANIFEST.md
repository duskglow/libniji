# PKG-MANIFEST: package contents as data (PKG-MANIFEST-4: merged with next, ready to land)

State: branch `task/pkg-manifest-4` (continues `task/pkg-manifest-3`, `-2`, `task/pkg-manifest`),
now merged with origin/next 923df67e9 (merge 62b227a75). Role keke `~/w/n3`. Spec:
`~/state/briefs/PKG-MANIFEST.md` ("## The change", "## Proof") plus PKG-MANIFEST-2's
byte-identity requirement and ruling #6370 (PKG-MANIFEST-4 brief).

## Done

* Runs 1-3: `tools/deb/manifest.tsv`, `control.tsv`, `description/<pkg>`, `manifest.py` (lint,
  rows, packages, control, member, check); build.sh / system.sh install, strip and write controls
  from the tables; the checks run `manifest.py check`; `crates/core/tests/deb_manifest.rs`;
  `rcrt1.o` row; lint refuses unshipped products.
* Run 4 (ruling #6370): merged origin/next. Its edits to `system.sh` and `check-system.sh` were
  NOT taken as script lines (both scripts keep the branch's side); each moved into the data:
  - system.sh `need locale; put .../usr/bin/locale` + strip unneeded: manifest row
    `system libc-bin usr/bin/locale file locale 0755 unneeded`.
  - system.sh charmaps loop (`products/charmaps/*` -> libc6 `usr/share/i18n/charmaps/`, 0644):
    43 rows, source `charmaps/<name>` (a file row's source may now be `<dir>/<name>`).
  - system.sh `need charmaps/ISO-8859-15` and check-system.sh's count against
    `spec/encoding/charmaps.tsv`: `manifest.py lint` (`charmap_errors`) checks the charmap rows
    are exactly that table's names, each system libc6 `usr/share/i18n/charmaps/<name>` 0644 none;
    `build.sh`'s `need` refuses any missing row product.
  - check-system.sh ISO-8859-15 `<code_set_name>` grep: `manifest.py check` now checks every
    shipped charmap names itself in its `<code_set_name>` line.
  - check-system.sh allow-list lines (`usr/share/i18n/...`, `usr/bin/locale`): covered by
    `manifest.py check` (exactly the rows' paths).
  - `lint --products` also looks inside each product directory a file row reads from
    (`charmaps/`); new test `an_unshipped_file_in_a_shipped_product_directory_fails_lint`.
  - build.sh (system NIJI_LIBDIR=lib/x86_64-linux-gnu) and root-system.sh merged cleanly (build
    setting and run-time check, not package contents).
  - `description/libc-bin` lists its programs literally (next's text) instead of `$programs`:
    with the locale row `$programs` would add `locale` and break byte identity.
* Proof `pkg-manifest-4-proof` (`~/work/PKG-MANIFEST-4/proof.sh`; products from
  `pkg-manifest-4-products`, the 923df67e9 tree, both flavours, release, full profile): all 9
  .debs IDENTICAL to next's scripts (7 system, 2 coexist); 43 charmaps and usr/bin/locale shipped;
  check-system / check / check-symbols / root-system PASS; parity inventory IDENTICAL; rc 0. The
  system loader now opens `/lib/x86_64-linux-gnu/libc.so.6` (next's libdir change); the proof
  checks that.

## Jobs (keke)

Final head d15162aff (code): `pkg-manifest-4-lint2` (clippy + static, rc 0),
`pkg-manifest-4-products` (rc 0), `pkg-manifest-4-proof` (rc 0), `pkg-manifest-4-plants` (5 of 5
CAUGHT, no WARN; `handoff/PKG-MANIFEST-4-plants.txt`), `pkg-manifest-4-gate` (escalates to FULL
by its own rule: `*/libc-so.ld`; still running at hand-back, the previous FULL gate took ~92 min). Step 0: `pkg-manifest-4-lint0` (inherited head b240c3a23, rc 0). Superseded:
`pkg-manifest-4-lint1` (static fs-order: two read_dir loops in the new test without FS-ORDER).

## Open / follow-up

* libc-bin's description (next's text, kept for byte identity) does not name `locale` and says
  "The C library's other programs (locale, localedef) are not part of this build yet" although
  libc-bin ships /usr/bin/locale. Fix: put `$programs` back in `description/libc-bin` and drop
  `locale` from that sentence (a data-only change; the control text changes on purpose).
* Optional: a CI step running the base-vs-head byte comparison for any tools/deb change
  (`~/work/PKG-MANIFEST-4/proof.sh` is the recipe).

## Not to redo / notes

* A proof's "the product names X" check must be behavioural (strace), not a byte grep.
* libc6-dbg's file set is unchanged: iconv and locale are strip `unneeded`.
* Plants file: `handoff/PKG-MANIFEST-4-plants.txt` (run from keke `~/work/PKG-MANIFEST-4/plants.txt`);
  the system-flavour plant reads keke `~/work/PKG-MANIFEST-4/proof/d-system-base`.

## Next step

Land once `pkg-manifest-4-gate` is green (if red: run only the failing suite).
