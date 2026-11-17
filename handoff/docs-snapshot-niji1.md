# DOCS-SNAPSHOT-NIJI1: user docs catch up with snapshot-2026-11-17

State: branch `task/docs-snapshot-niji1`, cut from `origin/next` at 7f7e6c9e4. Prose in README.md and docs/, plus
one narrow allowance in the old-name static check (xtask/src/oldname.rs) so the rename table can stand. Decisions
in force: D-X.114 (the rename), D-X.113 (header generator, the `__STDC_ISO_10646__` row). No generated view edited.

## What changed (before -> after)

1. #6213, docs/install-packages.md, system `libc6-dev` row. Before: "No headers yet: plain `gcc` compiles with the
   system's glibc headers". After: "No C headers yet: this `libc6-dev` replaces Debian's, so a root using the system
   packages has no `/usr/include/stdio.h` and cannot compile C until libniji's own headers land (plan/headers.md).
   The packages beside glibc (`niji6-dev`) are not affected: there the system's glibc headers stay".
2. #6214, rename table. Before: README.md:3 "libniji was called Liella until November 2026." only. After: a section
   "Names before libniji" in docs/install-packages.md with the old-to-new table (the published notes' rows plus
   `-lliella_native`, `.note.liella.*` notes and the Rust crates, from D-X.114), linked from README.md:3-5 and from
   the release notes' section.
3. #6215, the four Yohane fixes. Before: no sentence for #8, #7, #6 (#5 only as a file in the package table).
   After: install-packages.md, end of the system-flavour section, "What a Debian binary can expect of these
   packages" (#7 the ten GLIBC_2.2.5 versions, #5 libresolv.so.2 / libresolv.so / empty libresolv.a, #8 EAI_AGAIN,
   #6 partial iconv: EUC-JP, 58 of 1180 names, ISO-8859-15 and Debian's `iconv` program not yet); the iconv section
   of differences-from-glibc.md says coverage is partial (a scope gap, not a deliberate difference) with the same
   numbers; the release notes' section has all four.
4. #6216, differences-from-glibc.md `__STDC_ISO_10646__` row. Before: "SL-HEADERS-S05 decision record
   (`records/pending/SL-HEADERS-S05-3-1.toml`; plan/headers.md section 0)". After: "D-X.113 (SL-HEADERS-S05;
   plan/headers.md section 0)".
5. docs/RELEASE-NOTES.md: section "libniji: fifth pre-release, 2026-11-17 (`snapshot-2026-11-17`)" above the history
   marker, from ~/state/briefs/snapshot-2026-11-17-notes.md, headings one level down as in the file. The name table
   is replaced by a pointer to install-packages.md (the release notes above the marker may not name the old name).
   README.md's pre-release paragraph (was 2026-11-15, `bade54e0`) now names this snapshot.

## The check change

`cargo xtask static`'s old-name check refused any former name outside history, so the brief's table could not
stand anywhere a package user reads. Allowance added: table rows (lines starting `|`) under the heading
`## Names before libniji` of docs/install-packages.md, up to the next `## ` heading. Unit test
`rename_table_rows_are_allowed_only_in_their_section` (no spec clause: project policy). Plants:
handoff/DOCS-SNAPSHOT-NIJI1-plants.txt, 3 of 3 CAUGHT (job `docs-snapshot-niji1-plants`, mei).

## Results (mei, role s2)

- `docs-snapshot-niji1-static-2`: clippy, static (old-name, links) and `cargo test -p xtask --bin xtask oldname`
  (6 passed), rc 0. SUPERSEDED `docs-snapshot-niji1-static` (rc 1: the guide's rename sentence used `;` where the
  allowed sentence has `.`; fixed).
- `docs-snapshot-niji1-plants`: 3 CAUGHT, 0 WARN.
- SUPERSEDED `docs-snapshot-niji1-gate` (direct tier, rc 1: 472 passed, 1 failed,
  `the_shipped_ldconfig_passes_the_transcripts`). Cause: the role's `target/niji/ldconfig` was not built from this
  tree (static's partial-write line said so) and an xtask-only change does not rebuild the products.
  `docs-snapshot-niji1-transcripts` (`cargo xtask so`, then `cargo test -p xtask --test transcripts`): 3 passed, rc 0.
- `docs-snapshot-niji1-gate-2`: direct tier rerun on the final tree (result in the report json).

## Open / informational

- RULING NEEDED (light): the brief said "only prose"; the table needed the old-name allow-list extended (above).
  Recommendation: keep the narrow allowance; the alternative is no table outside the GitHub release.
- INFORMATIONAL: install-packages.md lists `libc6-dbg` among the system packages, while the 11-17 notes put
  `libc6-dbg` under "What's next" (it may have landed on next after the snapshot; not checked).
- INFORMATIONAL: differences-from-glibc.md row "getaddrinfo when every source is unavailable" (`EAI_FAIL`) is about
  name-service sources, not an unreachable nameserver (#8, `EAI_AGAIN`); left as it is, not re-probed.

Next step: none for this story beyond landing. Plan rows to read: D-X.114, D-X.113.
