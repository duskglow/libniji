# LDSO-PROGRAM-OPTIONS-REST: program mode's cost off the normal start, `--list` texts, dlopen test

State: branch `task/ldso-program-options-rest` (from origin/next 1d4d07280), role mei `~/w/s2` (Skylake-SP class).
Commits: 25a27d6ff (the change, tests, plants file), 260006817 (ldso_run expects "invalid ELF header"),
8c1a9ba1a (fold_preload without panic sites), 2b416b5a7 (test: --inhibit-cache opens no cache), then this note and the report. Two legs (leg 1 ended by REFRESH).
Brief steps done: 1 (normal start), 3 (`--list` of a missing / non-ELF file), 4 (in-tree dlopen test), 5 (no row
made obsolete: row 337 covers only the options of step 2). **Not done: step 2** (the five small options); it is the
next story (design notes below). Decisions in force: D-27.80, D-27.69, D-27.14, D-27.13; rulings #7513-#7516.

## What changed

- Exec'd start: `Preloads` has no `option` field (`names()` is the pre-LDSO-RUN-PROGRAM chain); `List` lost
  `preload_option`/`inhibit_cache`; `List::ctx` passes `cache` through. Program mode folds `--preload` into the
  LD_PRELOAD value (`entry::fold_preload`, cold; a private mapping kept for the process) and keeps the option's
  range (`state::keep_preload_option`/`in_preload_option`) for the diagnostic's "--preload" source.
  `read_cache` returns None after `--inhibit-cache` (`state::cache_inhibited`): program_file and dlopen rely on it.
- `--list PROGRAM` / running PROGRAM: `image::open_program` + `fail_program` (cold) give the reference's form,
  `<f>: error while loading shared libraries: <f>: <reason>`, 127, for missing / directory / short / non-ELF.
- Tests (`crates/ld/tests/ldso_program_options.rs`): `ldso_program_list_of_files_that_cannot_be_listed`,
  `ldso_program_dlopen_after_library_path_and_inhibit_cache` (program linked with libniji's crt1, libc.so.6 and
  ld.so), the non-ELF run case, LD_PRELOAD + --preload missing-name sources.

## Start-up counts (user instructions, perf, setarch -R, env -i, median of 51; base = origin/next 1d4d07280)

| class | program | base | head (8c1a9ba1a) | delta |
|---|---|---|---|---|
| Skylake-SP | empty main | 549,052 | 548,915 | -137 |
| Skylake-SP | three libraries | 798,034 | 797,837 | -197 |
| Zen 1 | empty main | 545,321 | 545,183 | -138 |
| Zen 1 | three libraries | 794,167 | 793,970 | -197 |

LDSO-RUN-PROGRAM cost +128 on the empty main, so the exec'd start is at or below its earlier level (not counted
against the pre-LDSO-RUN-PROGRAM revision itself). No start-up bound had moved, so none to tighten.
Why the first fix rose (+98/+38): a ptrace single-step profile per function (scratch `stepper.c`/`fnprof.py`,
step totals equal to perf's deltas) put it all in `bootstrap::relocate_self` (+234): `fold_preload`'s three
`split_at_mut` calls added three panic `Location`s, i.e. three RELR offsets (415 -> 418) relocated at every start.
The copy is now a zip into the zero-filled mapping. `link`/`run`/`niji_ld_start` save 137 / 197.

## Reference probe (Skylake-SP host, installed 2.39 ld.so)

- `--list` missing / text < 64 B / directory / non-ELF: 127, reasons `cannot open shared object file: No such file
  or directory`, `file too short`, `cannot read file data: Error 21`, `invalid ELF header`. Running a non-ELF: same.
- For step 2 (not implemented): `--list-tunables`: `name: value (min: x, max: y)` lines (sizes hex, counts
  decimal), 0, program ignored. `--list-diagnostics`: `key=value` lines (strings quoted, numbers hex), 0.
  `--inhibit-rpath LIST`: only an empty entry took effect (main program's RUNPATH ignored); names, paths, `x:libd.so`
  changed nothing; not probed: a dependency's full path, `':'`, `'a:'`. `--glibc-hwcaps-prepend`: named subdirs
  first, last copy wins, '' = none. `--glibc-hwcaps-mask`: built-in subdirs limited to exact names, '' = none,
  prepend not masked, last copy wins. Without a value: status 1, unrecognized-option text.
  Design note: hwcaps options as a `hwcaps` slice computed once per List (would also save the per-ctx
  `Native::hwcaps` call); inhibit-rpath's empty entry = the main program. Keep all of it cold (panic-free too).

## Jobs (final head) and next step

mei: `ldso-program-options-rest-check3` (clippy -D warnings, xtask clippy, static), `-gate3` (direct tier),
`-ldtests3` (preload, start_objects, ldso_search, search_cache, ldso_link), `-startup3` (`cargo xtask test`,
full-profile so, hot_text_products), `-plants3` (8 of 8 caught), `-count2`; keke `-zen-count2`. Results: report.
Next: a story for step 2 (the five options) with the probe notes above. Report: handoff/LDSO-PROGRAM-OPTIONS-REST-report.md.
