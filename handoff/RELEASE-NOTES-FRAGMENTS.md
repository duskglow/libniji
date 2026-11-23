# RELEASE-NOTES-FRAGMENTS: one release-note file per story, assembled at release

State: branch `task/release-notes-fragments`, code head `e5f52eb1e` (this note and the report json are committed on top
of it). Decision in force: **D-X.166** (records/decisions/D-X/D-X.166.toml). Role keke:~/w/n2.

## What changed

- `docs/release-notes.d/<STORY-ID>.md`: one note per story, front matter `section: <changes|compat|known-limitations|
  contributors>` and Markdown list items. Format and workflow: `docs/release-notes.d/README.md`.
- `cargo xtask release-notes [show | check | release --heading <text> [--at <rev>] [--intro F] [--outro F]
  [--dry-run]]` (`xtask/src/release_notes.rs`). Assembly is by section (`SECTIONS` order), then by file name.
  `release` writes the section above the newest released one, keeps every other byte, and deletes the fragments
  it used. Running it again with the same heading is a no-op. `--at <rev>` releases only the fragments of the
  snapshot's commit (unchanged since), so a note that landed after the tag stays for the next snapshot.
- Static check `release-notes` (42nd entry of `static_list`, so in `cargo xtask static` and in precheck's `static`):
  malformed fragment, unknown section, stray file, `## Unreleased` or a repeated heading in the shared file, and the
  assembled notes must insert.
- `docs/RELEASE-NOTES.md`: the Unreleased section (one entry, libthread_db, D-22.17, story SYS-THREAD-DB-2 per its
  record) moved to `docs/release-notes.d/SYS-THREAD-DB-2.md`; a two-line preamble pointer replaces it.
  CONTRIBUTING.md has one bullet on the rule.

## Proof (scratch: ~/w/.scratch/RELEASE-NOTES-FRAGMENTS/)

- Merge: in a scratch clone of this branch, story-a and story-b each add a fragment. Merging a then b and b then a
  both exit 0 and give identical `docs/release-notes.d` trees. The same two notes added to the old shared Unreleased
  list (at the parent commit) conflict: `merge rc=1 (docs/RELEASE-NOTES.md)`. Script: mergeproof.sh, log
  mergeproof.log.
- Assembly: `cargo xtask release-notes` on the role (job release-notes-fragments-unit1) prints 500 bytes,
  byte-identical (`diff` empty) to lines 8-13 of the old file (the Unreleased heading and its entry). Old lines 15-end
  equal new lines 11-end (`cmp`): released text is untouched.
- Unit tests: 8 in `release_notes::tests` (order independence, section order, unknown section, 13 malformed forms,
  shared-doc rules, release and rerun, `--at`, tree check).
- Plants: handoff/RELEASE-NOTES-FRAGMENTS-plants.txt, 7 plants (unknown section in a fragment caught by the unit test
  and by `cargo xtask precheck --only static`; assembly without the sort; unknown section accepted; Unreleased allowed;
  release rerun; `--at` ignored). Results: see the report json / hand-back.

## Open / next

- The ops side (not editable here): SNAPSHOT-PROCEDURE.md step 3 should draft the notes from `cargo xtask
  release-notes` on the validated head, and step 8's story should run `cargo xtask release-notes release --heading
  "<heading>" --at <tag> --intro <file> --outro <file>` instead of hand-editing the file. backlog/TEMPLATE.md has no
  release-notes sentence today. Proposed: "A change users may notice adds `docs/release-notes.d/<STORY-ID>.md`
  (format: that directory's README.md); never edit `docs/RELEASE-NOTES.md`."
- Stories already in flight that add a line under `## Unreleased` will now fail the `release-notes` static check
  (by design) and need to move their line into a fragment. CI's appendmerge is not involved for fragments (new
  files never conflict).
- Not done (not asked): the preamble line naming the newest snapshot is still edited by hand at release.

Do not redo: the section list is a decision (D-X.166). Adding a section means one `SECTIONS` row plus a README row.
