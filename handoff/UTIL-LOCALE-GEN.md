# UTIL-LOCALE-GEN: locale-gen, update-locale, validlocale (commands of the shipped localedef)

**State (leg 3, run 1).** Branch `task/util-locale-gen` from origin/next `4540a7cc8`; role keke `~/w/n4`. Records
D-31.36 (commands, locale-gen) and D-31.37 (update-locale, validlocale, package) written. Report:
`handoff/reports/UTIL-LOCALE-GEN.json`. Probe transcripts of the reference: `handoff/UTIL-LOCALE-GEN-probe.txt`
(trixie `locales` 2.41). Plants: `handoff/UTIL-LOCALE-GEN-plants.txt`.

## Done (do not redo)
* Code: `crates/locale/src/cmds.rs` (argv[0] dispatch in `crates/boundary/src/localedef.rs`, which also captures
  `PATH` and `DEFAULTCHARSET`); unit tests in `crates/locale/tests/prog.rs` (clause `s31_14_c_2`).
* Packaging: manifest rows, `tools/deb/system/locales.{config,postinst,postrm,templates}`, `locale.alias`,
  `tools/deb/smoke/locale.c`, the locales block of `tools/deb/root-system.sh`.
* Transcripts `validation/transcripts/{update-locale,validlocale}/` recorded against keke's reference (Ubuntu 24.04
  `locales` 2.39) and checked against the trixie probe. Leg 3 fixes found by them: update-locale's LANGUAGE check
  is one of the sanity checks (`--no-checks` skips it; probed on 2.39) and runs after the availability check
  (trixie probe); validlocale's usage line names a bare argv[0] by the PATH directory holding it. Dropped cases:
  `validlocale` usage (output depends on the host's PATH; unit test covers it); the empty input line of
  `update-locale/edit` (trixie writes `#`, 2.39 keeps it empty; libniji follows trixie, D-31.37).
* Docs: differences rows (`/etc/locale.alias` without aliases; update-locale's error has no check-program block),
  release note `docs/release-notes.d/UTIL-LOCALE-GEN.md`.
* debconf proof script (scratch, keke `~/work/util-locale-gen/`): `final-root.sh COMMIT OUT` = `debroot.sh`
  (build.sh --flavour system, check-system.sh, root-system.sh) then `reconf.sh` (two throwaway trixie roots,
  stock and with libniji's libc6/libc-bin/locales swapped in; preseed + `dpkg-reconfigure locales`; compare).

## Open
* RULING NEEDED: `/etc/locale.alias` ships with no aliases; recommend a story deriving aliases from CLDR language
  names, or none.
* See the report for the jobs on the final head and their results.

## Next step
Read the report json; if any job in `jobs` failed, fix that and rerun only it.
