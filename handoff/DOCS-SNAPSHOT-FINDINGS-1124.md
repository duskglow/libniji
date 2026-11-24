# DOCS-SNAPSHOT-FINDINGS-1124 (handoff, 2026-11-24)

State: branch `task/docs-snapshot-findings-1124`; docs and packaging-description change only, no library code.

## Done
* **L3:** `tools/deb/manifest.py` rewraps a description line longer than 80 columns after placeholder expansion;
  libc-bin's template puts `$programs` in a sentence of its own. Checked on the generated control text of libc-bin,
  libc-dev-bin, locales and libc-l10n: longest line 79. NOT VERIFIED with lintian: the lintian host (mei, chroot
  `lint-1`) was refused to this run (ssh to mei denied by the classifier), keke has no lintian. The expected count is
  one fewer W (43 -> 42); the dispatcher or the next snapshot run must confirm.
* **D1:** `locales` description and `docs/install-packages.md` now say what it ships (287 locale sources and the `C`
  source, SUPPORTED, charmaps). `libc-l10n` stays a placeholder: no translations are built (manifest has only doc
  rows), so the brief's premise that it is no longer a placeholder is false for `libc-l10n`; docs say so.
* **D3:** sections for `snapshot-2026-11-23.1` and `24.1` added to docs/RELEASE-NOTES.md from the published releases
  (GitHub API via curl; `gh` is not installed). Removed the three fragments those releases cover (UTIL-14-4,
  DOCS-CATCHUP-1121-3, SYS-THREAD-DB-2). New fragments: THREAD-CREATE-COST-2, UNWIND-EXIT-COST-2, ICONV-LIST-ALIASES-2,
  LOCALE-C-UTF8-SOURCE. Not given a fragment, with reason: HEAP-WALK/HEAP-FIND (24.1 covers #21 size classes),
  SHSTK-OPT-IN/ENABLE (24.1 covers), the L2 date format (24.1 covers), LOCALE-COLLATE-TAILORINGS-4 (a test-file move),
  LOCALEDEF-COLLATE-GAPS-2 (merge and ruling citation; its code effect is in the LOCALE-C-UTF8-SOURCE fragment).
  Numbers in the fragments are the stories' own Skylake-SP counts from their reports (not re-measured here).
* **D4:** no test of COMPAT-12 exists; `records/compat/COMPAT-12.toml` (the source of the generated row) now says the
  test was removed with the client. `docs/compat-flags.md` is a train-owned generated view and was not edited.

## Next step
Confirm the lintian W count on a rebuilt libc-bin; regenerate views on the train. The `docs/differences-from-glibc.md`
`iconv -l` row (D-6.20) still says 982 names; the ICONV-LIST-ALIASES-2 count is 1028: an owner of that row should update it.
