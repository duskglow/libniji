# UTIL-LOCALE-GEN: locale-gen, update-locale, validlocale (commands of the shipped localedef)

**State (leg 4, run 1, head 43231b0d0 + this note).** Branch `task/util-locale-gen` from origin/next `4540a7cc8`; role keke `~/w/n4`. Records
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


## Leg 4 results (head 43231b0d0, run 1 final)
* The leg-3 lock fix (ec93dfe4c) failed clippy's S8 lint (`Vec::push`, `to_vec` in niji-boundary).
  43231b0d0 keeps the held locks in a `TryVec<TryVec<u8>>` and allocates the record before taking the lock, so
  every lock the process holds is recorded.
* keke util-locale-gen-verify4 (rc 0): `cargo xtask clippy` clean, `cargo xtask static` green, records check ok,
  unit `-p niji-locale --test prog` 26 passed, `cargo xtask so` ok, transcript check 9 ok / 0 mismatched, transcripts
  test 1 passed.
* keke util-locale-gen-plants3 (head ec93dfe4c; plant targets `crates/locale/src/cmds.rs` and its tests are
  unchanged since): 2/2 CAUGHT, no WARN lines (`handoff/UTIL-LOCALE-GEN-plants.txt` already holds these results).
* keke util-locale-gen-root5 (rc 0, `final-root.sh 43231b0d0 ~/work/util-locale-gen/d5`): root-system
  `RESULT pass=31 fail=0`. That includes the locales lines: the postinst made /etc/locale.gen (nothing selected),
  /etc/locale.conf and /etc/default/locale -> ../locale.conf; re-configuring compiles the two selected entries (the
  root3 hang is gone); `locale -a` lists de_DE.utf8 and en_US.utf8. Debconf proof: preseed + `dpkg-reconfigure
  locales` in trixie with libniji's libc6/libc-bin/locales swapped in gives the same result as stock trixie 2.41
  (both locales generated, /etc/locale.conf LANG=de_DE.UTF-8, validlocale, setlocale, debconf answers). The only
  differences are the version lines and the swap step's own output. The release note's debconf sentence is
  therefore proven and stays.
* keke util-locale-gen-gate5 (direct gate on 43231b0d0, started 20:47Z): PENDING at hand-back.
  gate3 (red: clippy) and gate4 (poisoned) were stopped. gate4 failed because it ran in the role while
  verify4's `cargo xtask so` rebuilt target/niji ("products missing or half-built"). Never run a products build
  beside the gate in one role. Leg 3's gate2 was red only on the products-reading lint test, which passed rerun alone.

## Open
* RULING NEEDED: `/etc/locale.alias` ships with no aliases. Recommend a story that derives the aliases from CLDR
  language names, or shipping none.
* No direct test reaches the lock fix. `localedef` locks once per process; only locale-gen's in-process repeats
  reach a second lock, and they write under /usr/lib/locale, so today only the packaged-root check (root5) exercises
  it. A follow-up could run locale-gen in an unprivileged user+mount namespace with a tmpfs over /usr/lib/locale
  and plant the "already held" check.
* root-system.sh runs dpkg with no timeout, so a hang there blocks the job until it is killed. Consider adding one.

## Next step
Collect gate5: `~/bin/jobwait util-locale-gen-gate5` on keke; read `~/bin/test-summary ~/logs/util-locale-gen-gate5.log`.
If it is green, the story is ready for CI. If it is red, fix it and rerun only the failing suite.
