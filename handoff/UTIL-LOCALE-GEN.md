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

## Leg 3 results (head cbe50753d, SUPERSEDED by the lock fix)
* keke util-locale-gen-verify2: unit 26 passed; transcripts 9 ok; transcripts test ok; clippy clean.
  util-locale-gen-static3: records check + static green. util-locale-gen-plants2: 2/2 CAUGHT (results in the
  plants file). util-locale-gen-gate2: red on ONE test, `so::tests::a_product_build_leaves_no_earlier_product_for_
  the_manifest_lint` (charmaps missing from the real target/niji: kill-mine had stopped a `cargo xtask so`
  mid-build at 19:10); rerun alone, util-locale-gen-rerun1: 1 passed. INFORMATIONAL: that test sits in the gate's
  "tests that run no products" group but reads the real products.
* util-locale-gen-root3 HUNG in root-system's `dpkg -i locales` (2 entries selected): locale-gen blocked in flock
  on /usr/lib/locale, its own lock taken by the first locale's localedef run through another descriptor (a real
  defect: the packaged-root check caught it). Fixed in `crates/boundary/src/localedef.rs` (Disk::lock keeps the
  locked dirs; a second lock of the same dir is already held). root-system.sh has no timeout around dpkg: consider
  one (a hang there blocks the job until killed).

## Running (leg 3 started them on the lock-fix head; the next leg collects)
* keke util-locale-gen-verify3 (unit + so + transcripts + clippy + static), util-locale-gen-gate3 (direct gate),
  util-locale-gen-plants3, util-locale-gen-root4 (`final-root.sh <head> ~/work/util-locale-gen/d4`: debroot +
  reconf; read `grep "root: \(FAIL\|RESULT\)\|^reconf" ~/logs/util-locale-gen-root4.log` and the `== reconf niji`
  / `== reconf diff` blocks: the debconf proof, stock trixie vs libniji swapped in).

## Open
* RULING NEEDED: `/etc/locale.alias` ships with no aliases; recommend a story deriving aliases from CLDR language
  names, or none.
* Release note says the postinst applies the debconf choices: keep only if root4's reconf proves it, else reword.

## Next step
`~/bin/jobwait` the four labels above (one call per 10 min); fix any red (rerun only that job); copy plants3's
results.txt over handoff/UTIL-LOCALE-GEN-plants.txt; precheck `--jobs` those labels on the final head; write
handoff/reports/UTIL-LOCALE-GEN.json (impact: correctness lock fix, coverage 10 transcripts + 5 unit tests + 2
plants + root-system locales block; unblock: Yohane locale-gen); handback-check; hand back.
