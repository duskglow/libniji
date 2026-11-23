# Unreleased release notes: one file per change

A change users may notice gets a release note in this directory, one file per story, instead of a line in
[docs/RELEASE-NOTES.md](../RELEASE-NOTES.md). That file holds only released sections. Because every note is its own
file, two branches that each add a note never touch the same line and merge in either order (D-X.166).

## Adding a note

Create `docs/release-notes.d/<STORY-ID>.md` (capitals, digits and `-`; a second note of the same story:
`<STORY-ID>-2.md`):

```markdown
---
section: changes
---
- **`thing` (D-12.3):** what a user notices, in the words of the released notes, wrapped at 120 columns,
  continuation lines indented by two spaces.
```

The note is one or more Markdown list items: each line starts with `- ` or with two spaces, no blank lines and no
headings. To change or withdraw a note before it is released, edit or delete its file.

The sections, in the order they are assembled (`SECTIONS` in `xtask/src/release_notes.rs`):

| `section:` | Goes under |
|---|---|
| `changes` | the release heading itself: the list of changes every snapshot has |
| `compat` | `### Compatibility flags and differences` |
| `known-limitations` | `### Known limitations` |
| `contributors` | `### For contributors` |

Within a section the notes are ordered by file name, so the result does not depend on the order they landed in.

## Commands

```bash
cargo xtask release-notes            # print the unreleased notes, assembled
cargo xtask release-notes check      # the check `cargo xtask static` and the precheck run
cargo xtask release-notes release --heading "<text>" [--intro <file>] [--outro <file>] [--dry-run]
```

`check` fails when a fragment is malformed or names an unknown section, when this directory holds anything but
fragments and this README, or when `docs/RELEASE-NOTES.md` has an `## Unreleased` section or a heading twice.

`release` (at a snapshot or release) writes `## <text>`, the intro, the assembled notes and the outro into
`docs/RELEASE-NOTES.md` above the newest released section, keeps every other byte of that file, and deletes the
fragments it used. Run again with the same heading it changes nothing. The intro and outro are the release's own text
(the commit and tag line, the packages and known-limitations paragraph); `--dry-run` prints the section only. The
preamble line naming the newest snapshot is edited by hand.
