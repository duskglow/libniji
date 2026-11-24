# RELEASE-NOTES-1120: release notes brought up to snapshot-2026-11-22.3

State: branch `task/release-notes-1120`; docs only (`docs/RELEASE-NOTES.md`).

Done:
- One dated section per published snapshot, newest first, 18.1 to 22.3 (19 snapshots; the brief named 18.1 to 20.3,
  the rest were published since). Source: the published `snapshot-*-notes.md`; nothing added beyond them.
- The "Unreleased" section listed COMPAT-19, which shipped in 20.3: now it says nothing is recorded yet.
- The `_r_debug` limitation line in the fifth pre-release is marked as fixed since 19.2 (#19); a note under the title
  says each section's limitations are those of its own snapshot, and the newest list is in 22.3.

Notes:
- `git pull` in the ops release directory was refused by the sandbox classifier; I used the copy as found (it held
  snapshots up to 22.3). Snapshots published after that are not covered.
- The brief's "doccheck" has no command in `cargo xtask` or `ops`; I could not run it. The 23 problems it reported are
  not listed or fixed here (the file I touched was not checked by it).
- Entries are summaries (bullets and the headline numbers); package versions and check tables are not repeated except
  the 22.3 limitations list. Dates, hashes and tags are from the notes.

Checks: `cargo xtask static` and `cargo xtask precheck` green on mei (s3), clippy green.
Next step: land; add a section for any snapshot newer than 22.3.
