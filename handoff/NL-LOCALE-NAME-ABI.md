# NL-LOCALE-NAME-ABI(-2,-3): _NL_LOCALE_NAME, static_assert, _STDIO_H, complete struct __locale_struct, sweep

**State (run 4, merge only):** branch `task/nl-locale-name-abi-4` = `-3` plus a merge of origin/next 227f90c04
(`e6515ebc7`) and regenerated headers (`10d6c9573`); role keke:~/w/n2. Conflicts kept both intents: one
`static_assert` block in assert.h.in (both sides had the same condition); `_NL_LOCALE_NAME` stays a header_probe
candidate (next's stage2_names already lists `static_assert`); s10 runs next's `gettext_units` and this branch's
`locale_struct_units`; this branch's `static_assert_units` was dropped as a duplicate of next's s05 check (defined
exactly where visibility.tsv shows it, usable). assert.h and stdio.h regenerated (git had text-merged stdio.h).
Jobs: `nl-locale-name-abi-4-lint` (clippy + static, green), `nl-locale-name-abi-4-test2` (so, headers-gen, full
header_check, s07_locale_symbols, xtask tests). The run-1 plants on `static_assert assert.h` name the dropped unit;
next's s05 is the check now. Ruling #8664 (keep the non-NULL LC_ALL slot, add a differences row and a registered
future compat flag) is NOT done in this merge-only run: it is the next step.

**Earlier state:** branch `task/nl-locale-name-abi-3` (continues `-2` and `-1`, from origin/next ecc8c95e3), role keke:~/w/r1.
Head: see `handoff/reports/NL-LOCALE-NAME-ABI-3.json`. Decisions: none new. `_STDIO_H` and `__locale_struct` are
`headers/reserved.tsv` emit rows (owner rulings R-H2, R-H3); the rest follows reference probes and ISO C 7.2.
No differences row, no compat option (one RULING NEEDED below).

## Commits
* Run 1: `93027447b` _NL_LOCALE_NAME (template, visibility row, s10 form rows, nl_langinfo(_l) cases);
  `3770f6ab5`, `e499daade` static_assert. Run 2: `f55e4b249` `_STDIO_H` + header_check P1G guard units.
* Run 3: `53fdbc8fb` struct __locale_struct complete (reserved.tsv row; 11 regenerated headers; s10
  `locale_struct_units`: size, alignment, member offsets of layouts.tsv in every header visibility.tsv shows it, for
  posix200809, xopen700, gnu17, cxx17; complex.h skipped with the reason); `d42fa164c` s07_locale_symbols
  `sym_nl_langinfo_l` asserts each real category's `__names` slot equals `_NL_LOCALE_NAME`'s answer; plants file
  `handoff/NL-LOCALE-NAME-ABI-3-plants.txt` (2; results at its foot); release note extended.

## Proofs (keke, run 3; scripts in ~/work/nl-locale-name-abi-3/: lgp.sh, gt.sh, names.sh, all2.sh)
* **liblocale-gettext-perl 1.07-7** (trixie source via pkgsrc_root.sh): built as Debian does against libniji's
  headers, linked to libniji, tests under trixie's perl 5.40.1 on libniji's loader, LC_ALL = C, C.UTF-8,
  en_US.UTF-8, bg_BG.utf8: bind.t and use.t ok everywhere, the same as the reference build on the host library;
  frconvert/jaconvert/raw print no result on BOTH sides (their data generator; no locale with translations here).
  Negative control (headers without `_NL_LOCALE_NAME`): every test fails with Yohane's exact "got first handshake key
  0xf280080, needed 0xeb80080". Job `nl-locale-name-abi-3-proof2` (final headers), first run `-lgp1`.
* **gnulib localename**: Yohane's stage 2 failed findutils, gettext, gnutls28, grep, libunistring, m4, sed on
  "invalid use of undefined type 'struct __locale_struct'" (logs only; no gnulib source opened). gettext-runtime
  0.23.1 configured on the reference, its localename-unsafe object rebuilt with libniji's headers: old headers fail at
  Yohane's line (3049:31); with `_NL_LOCALE_NAME` alone still the same failure; with the complete struct it compiles.
  The other six packages' full rebuilds: NOT VERIFIED (same error, same fix expected).
* **`__names` runtime** (names.c, both libraries): layout 232/8, offsets as layouts.tsv; slots 0-5, 7-12 identical
  to the reference for C, POSIX, en_US.UTF-8, mixed newlocale, duplocale, uselocale, duplocale(LC_GLOBAL_LOCALE).
  Slot 6 (LC_ALL) differs: reference "C" or NULL depending on how the object was made; libniji LC_CTYPE's name.

## Earlier (runs 1-2, still true)
* `_NL_LOCALE_NAME(c)` = `_NL_ITEM(c, 0xffff)`; perl HS_KEY 0xeb80487 / sizeof(PerlInterpreter) 3768 equal to the
  reference. Sweep: only `_NL_LOCALE_NAME` changed a layout; reference-only names left (no layout change):
  `va_copy` in printf.h, `_IOC_SIZE` sys/soundcard.h, `CHARBITS` values.h, `_PATH_RESCONF` resolv.h, S_IS* ftw.h,
  `SPACE` scsi/scsi.h. Run 3 adds: `struct __locale_struct` (completeness, not a macro) was the other build-breaker.

## Open
* Ruling #8664 (resolved): keep the non-NULL LC_ALL slot of `__names`; add a docs/differences-from-glibc.md row
  (slot 6 contents per construction path, from the run-3 names.c probe) and a registered future compat flag
  (`ops reserve --story <story> COMPAT`). No code change. Owed: next run on this chain.
* INFORMATIONAL: libniji's C++ `<complex.h>` shows no `locale_t` (the reference's reaches it through libstdc++).
* Next step: the train (FULL: generated headers); then Yohane's stage-2 rebuild of the seven gnulib packages and
  liblocale-gettext-perl against the next snapshot.
