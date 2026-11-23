# CATALOG-REPLACED-FILE-RACE (handoff, one page)

**State:** branch `task/catalog-replaced-file-race`; head in `handoff/reports/CATALOG-REPLACED-FILE-RACE.json`.
Decision in force: **D-7.23** (new; reserved id). Reads: D-7.21 (where a locale name is searched), D-7.5.

## What was wrong (the premise was misattributed; coordinator accepted the challenge, #8189)
- The `@ line 91 ... catalog file replaced on disk` hunk is a **recorded** difference (KNOWN, D-7.5 caching). The old
  `line_diff` printed every hunk, recorded ones included, so the first hunk was always that one; gate summaries keep
  only the first lines. The replaced-file lines never flipped: in all 28 failing logs on mei, reference `[old]` and
  libniji `[new]`.
- The extra line (1169 vs 1170) was always `-- LC_ALL=de_DE.UTF-8@euro, no LANGUAGE` (and the NL_CAT_LOCALE @euro
  case): the reference printed `setlocale(6, [de_DE.UTF-8@euro]) failed`, libniji the `order:`/`never:` lines.
- **Deciding condition: the host's system locale directory.** setlocale searches `/usr/lib/locale` after the LOCPATH
  directories (setlocale(3)). On 2026-11-23 06:32 `locales-all` was installed on mei (apt history), creating per-locale
  directories including `de_DE@euro` (ISO-8859-15). Every one of the 28 failures is after that time. Black-box probe
  (`setlocale(LC_ALL, name)`, LOCPATH = target/test-locales holding de_DE.UTF-8):
  - directories present: reference and libniji (next at e25184748) both `de_DE.UTF-8@euro -> NULL`, `de_DE@euro -> ok`, `de_DE.utf8 -> ok`;
  - `/usr/lib/locale` hidden (unshare -r -m, tmpfs): both `de_DE.UTF-8@euro -> de_DE.UTF-8@euro` (the record's answer),
    `de_DE@euro -> NULL`, `de_DE.utf8 -> NULL`.
  - Failing branches were based before D-7.21 (LOCALE-PER-DIR-LOAD, landed the same day), whose libniji did not search
    the default directory after LOCPATH, so it still succeeded. On current next the test passed by agreement while both
    sides failed the case ("note: the reference differs from the record"): the @euro case silently covered nothing.
  - The reference has no built-in C.UTF-8: with an empty `/usr/lib/locale` its C.UTF-8 cases fail; it loads
    `/usr/lib/locale/C.utf8` (installed with the C library). libniji has it built in.

## The fix (test only; no libniji code changed; libniji already matches the reference in both states)
- `nscache_mode.sh [--hide-system-locales] on|off|plain`: in the private mount namespace, before the drop to the user,
  `/usr/lib/locale` becomes a tmpfs holding only `C.utf8`, bound back from a descriptor on the real directory.
  `plain` = no variable changed (the reference's run). Other users of the wrapper are unchanged (flag is opt-in).
- `catalogs_equal_the_reference` runs all three runs (reference included) with the flag; `catalogs_run.sh` refuses
  (exit 3, "holds more than C.utf8") to run the compared cases otherwise, so an unhidden run fails every time on any
  host with locales installed, never some of the time.
- `line_diff` lists unrecorded runs first and marks recorded ones `(recorded)`.
- Result on mei (locales-all installed, load ~30): "the reference equals the record"; test passes.

## Not to redo
- Do not probe catalog reload timing: it never flipped. Do not change libniji's locale search: it matches the reference.

## Open / next step
- INFORMATIONAL: `locale_products` (`oracle/locale_diff.c` names `en_US`, `en_US.utf8`, `de_DE.UTF-8@euro`, `french`
  with LOCPATH) reads the host's system locale directory the same way, and failed on mei after 06:32 today ("lines that
  differ from the reference: 3"). Recommend a follow-up story: run it under `nscache_mode.sh --hide-system-locales
  plain` (or document that it compares against the host's installed set). C.UTF-8-only users (newlocale_mask, wstream,
  iconv, printf probes) depend only on C.utf8, which ships with the C library.
- `locale_data` asserts "the gate machines have a locale archive"; that failed on mei at 11:51 today (archive rewritten
  18:10). Host state, not this story.

Plants: `handoff/CATALOG-REPLACED-FILE-RACE-plants.txt`. Report: `handoff/reports/CATALOG-REPLACED-FILE-RACE.json`.
