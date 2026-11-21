# UTIL-GETCONF (UTIL-5): getconf

**State.** Branch `task/util-getconf` (from origin/next `98081ab6c`), role keke `~/w/n1`. Code commit `4708762f9`;
this note and `handoff/reports/UTIL-GETCONF.json` are the commit after it.

## What is on the branch

* `csys/util/getconf.c`: the front end over Liella's `sysconf`, `confstr`, `pathconf`. `cargo xtask so`
  (`xtask/src/so.rs` `run_getconf`, `getconf_names`) generates `target/getconf-gen/getconf-names.h` from
  `spec/abi/getconf.tsv` joined with `spec/abi/sysconf.tsv` and links `target/liella/getconf` as a static PIE over
  `libc.a` + `liella-rcrt1.o` (no PT_INTERP, checked). Unit test `so::tests::getconf_names_join_the_two_tables`.
* `spec/abi/getconf.tsv`: 414 names the reference accepts (320 listed by `-a`, in `-a` order; 94 unlisted aliases
  such as `TRACE`, `VERSION`, `ADVISORY_INFO`), each with its constant and `print` (`unsigned` only for `ULONG_MAX`).
* Shipped: `tools/deb/system.sh` puts it at `/usr/bin/getconf` in libc-bin (stripped like ldconfig);
  `tools/deb/check-system.sh` allows the path. NOT VERIFIED: no package was built in this run (UTIL-LDCONFIG's open
  question 1 may still block `build.sh --flavour system`).
* Harness (`xtask/src/transcript/`): `host-value: <line prefix>` (rest of line `@HOST@` in the record and in record
  comparisons; reference vs Liella compared in full) and `mask-value: <line prefix>` (masked on every side; used only
  for `_AVPHYS_PAGES`). Documented in `docs/transcripts.md`. `liella_default` maps `getconf` to
  `target/liella/getconf`.
* `validation/transcripts/getconf/`: 25 transcripts recorded from the reference on keke: `-a` with `/`, `.`, a missing
  path and `--`; usage cases; sysconf/confstr/pathconf values; `ULONG_MAX`; an alias; unknown names; bad option;
  every `-v` form probed. `-a` transcripts mark 54 host-dependent names `host-value` (sysconf.tsv `varies`, every
  pathconf name, caches, processors, memory, process limits).
* `records/pending/UTIL-GETCONF-1.toml` (D-NEW, series D-31): static front end, generated table, identity strings.

## Probe-settled (reference, black box; scratch scripts not committed)

Usage text on stderr + status 2 for: no operand, `-v` alone, `-va`, sysconf/confstr name with a path, pathconf name
without one, `-a` with two operands or followed by `-v`. Unknown name (also `-x`, `--foo`, `-h`, `-V`, `--usage`,
`--he`, `_SC_ARG_MAX`, `NOSUCH /`): ``getconf: Unrecognized variable `X'`` status 2. `-v SPEC` / `-vSPEC` accepted
for any SPEC (no re-exec seen; `GETCONF_DIR` ignored), then optional `--`; `-v -a X` takes `-a` as the spec; `-a --`
uses `--` as the path; `-- -a` is an unknown name. pathconf error: `getconf: pathconf: P: <strerror>` status 3; in
`-a` an unanswerable path name prints no value, status 0. sysconf -1 prints `undefined` (empty in `-a`), except
`ULONG_MAX` (prints 18446744073709551615). `-a` = `%-35s` + value. Write errors (closed stdout, /dev/full): status 0.
`--help`/`--version` work only as the first operand (extras ignored).

## Open / not done

* `--help` and `--version` are not compared (identity strings, plan Q3): Liella's `--help` is the reference's text
  without the distribution's bug-reporting lines; `--version` prints `getconf (Liella) <confstr version>`.
* Not probed: an unknown name followed by two operands (Liella reports the name as unknown); `-v X --help`.
* `getconf.tsv` was produced by scratch probe scripts (candidate names from every constant in sysconf.tsv, run against
  the reference, mapped by the POSIX name rules plus 8 hand mappings such as `LOGNAME_MAX`->`_SC_LOGIN_NAME_MAX`).
  A `spec/tools/getconf_probe.py` that regenerates it is a possible follow-up.
* Rule slips in this run (reported, no effect on the tree): one heredoc to python and one `sed -i` on
  `xtask/src/transcript/mod.rs` before switching to the Edit tool / python file rewrites.

## Next step

Read `util-getconf-gate` and `util-getconf-plants` on keke (`~/bin/jobwait util-getconf-gate util-getconf-plants`).
Then UTIL-SHAPE can measure this static getconf against a dynamic build (one link line in `run_getconf`).
Read: plan/utilities.md §2.3/§5, this note, docs/transcripts.md, spec §31.5.
