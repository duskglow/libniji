# LDSO-RUN-PROGRAM: ld.so run as a program takes ld.so(8)'s options and runs static programs

State: branch `task/ldso-run-program-2` (continues `task/ldso-run-program`, from origin/next 90390071c). Commits:
d9c103ffb (the change, tests, records, plants), 44ccf4e9a (empty `--argv0` fix and a test-harness fix), the first
run's note and report; run 2: ebd5de49d (the map test's expected refusal form), then this note and the report.
Roles: run 1 keke `~/w/r1`, run 2 mei `~/w/s3`.

## Run 2 (LDSO-RUN-PROGRAM-2): the start-up check's red

- Red: `niji-ld::map map_malformed_corpus_refused_by_ldso` (start-up job, `cargo xtask test`): it expected program
  mode's refusal of a malformed file to start `ld.so: `; run 1 had moved program mode's `image.rs` `load` refusals to
  `PATH: error while loading shared libraries: PATH: <reason>`. Reproduced on mei (deterministic, the only failure
  of run 1's 7047-test run).
- Probe (mei, the test's own PIE and corpus rebuilt from it by a scratch script; reference = the installed 2.39
  ld.so run as a program): on the entries it refuses (congruence-broken, huge, huge-align, not-loadable, the
  fixed-address program at 0) the reference prints exactly that one-line form, status 127. Program mode did not
  regress: status 127 as before, one line, no signal, and the plan refuses before the first mapping call
  (`map_malformed_corpus_refused`, unchanged and green). So the test encoded libniji's old text: it now checks the
  reference's form, a non-empty reason and one line for every corpus entry (ebd5de49d).
- Reasons: libniji's program mode keeps its own reason texts (`MapError::message`: "malformed loadable segments",
  "not a valid ELF file for this machine" for `ET_REL`, ...); the reference's differ, and libniji's dependency path
  already gives the reference's texts for some of them ("only ET_DYN and ET_EXEC can be loaded", "ELF load command
  address/offset not page-aligned"). Not changed here (pre-existing, not a regression); next step below.
- Plants (handoff/LDSO-RUN-PROGRAM-2-plants.txt): the old `ld.so:` form, an empty reason: 2 CAUGHT.
- Jobs (mei `~/w/s3`, tree of ebd5de49d): `ldso-run-program-2-check0` (clippy all-targets all-features and
  `cargo xtask static` on the inherited head: clean), `ldso-run-program-2-plants` (2 CAUGHT, no WARN),
  `ldso-run-program-2-gate` (direct tier, `--base origin/next`: 13 suites, 816 passed, 0 failed, 0 SKIPPED, peak
  3.8G), `ldso-run-program-2-startup` (`cargo xtask test`: 7047 passed, 0 failed; full-profile `so` ok;
  `hot_text_products` 33 passed), then clippy, static and precheck on the final head (hand-back names them).
  The +128-instruction start-up cost is unchanged by run 2 (no loader code changed); its fix-forward is
  LDSO-PROGRAM-OPTIONS-REST (#7513).

## What changed (D-27.80, records/decisions/D-27/D-27.80.toml, reserved this run)

- `crates/ld/src/program.rs` (pure): `Action::Program { mode, name, at, options }`, `Mode` (Run, List, Verify),
  `Options` (library_path, preload, argv0, audit, inhibit_cache), `UNSUPPORTED`, `USAGE` (1), `verify_status`,
  `audit_names`, `audit_ignored`; `refusal` takes `argv[0]`; new `--help` text.
- `crates/boundary/src/ld/entry.rs`: one `#[cold] #[inline(never)]` `program_mode` replaces the old Program/List
  arms. It applies the options, keeps `--library-path`/`--inhibit-cache` for `dlopen` (`state.rs`
  `keep_program_options`/`program_options`; `dlopen.rs` `Env::read_with` reads them), lists or verifies, reports
  each `--audit` object, maps the program (`load`), rewrites auxv, moves the stack top past the options, sets
  `--argv0`, then: no `PT_INTERP` -> entered as the kernel enters it (static, static PIE; `e_entry` 0 refused),
  else the one start path `run`. `List` gained `preload_option`, `inhibit_cache` (ctx() gives no cache when set).
- `link.rs` `Preloads.option`: names are LD_PRELOAD's, then `--preload`'s, then the file's; the diagnostic says
  `from --preload` for the option's names. `image.rs` `load`: the reference's message form and reasons for a
  missing file, a directory, a short file; `program_file` skips the cache under `--inhibit-cache`.
- Records: spec/behaviour/dynlink.md "Program mode options" (the probe); docs/differences-from-glibc.md row (the
  six options not implemented: `--audit` reported, five refused; no compat option, reason in the row).
- Tests: `crates/ld/tests/ldso_program_options.rs` (5: parsing, library path + list + inhibit-cache, preload +
  argv0, verify + audit + refusals + usage errors, non-dynamic files); `ldso_link.rs` updated (status 1).

## Probe table (Zen 1 host; reference = 2.39 ld.so; libniji after = this branch; scripts in the run's scratch)

| case | reference | libniji before | libniji after |
|---|---|---|---|
| `--library-path lib ./prog a b` (PIE and non-PIE) | runs, dep from lib | 127 unrecognized option | as reference |
| `LD_LIBRARY_PATH=lib2` + `--library-path lib` | lib wins, env keeps lib2 | refused | as reference |
| `--library-path` twice / `nolib:lib` / `''` | last wins / both / not found | refused | as reference |
| `--library-path=lib`, `--`, option without value, unknown, none, `--list` alone | 1, `<argv0>: ...` + hint | 127, `ld.so: ...` | as reference |
| `--preload` one, `a:b`, `'b a'`, twice, with LD_PRELOAD, missing, bare name | as ld.so(8): env first, last copy wins, diagnostic `from --preload` | refused | as reference |
| `--argv0 NAME`, `--argv0 ''` | sets argv[0] | refused | as reference (empty: fixed in 44ccf4e9a) |
| `--inhibit-cache` (start, `--list`, dlopen of a cache-only name) | cache not used | refused | as reference (dlopen: probe only) |
| dlopen after `--library-path` | option replaces LD_LIBRARY_PATH | refused | implemented (probe only, no in-tree test) |
| `--verify` prog / lib / static / static-pie / text / missing | 0 / 2 / 1 / 2 / 1 / 1, silent | 127 unknown option | as reference |
| `--audit ./libaud.so` | la_version runs | refused | diagnostic, program runs (differences row) |
| `--inhibit-rpath`, `--glibc-hwcaps-mask/-prepend`, `--list-tunables`, `--list-diagnostics` | accepted / print | 127 unknown | refused, 1 (differences row) |
| missing / text / script / directory file | 127 `<f>: error while loading shared libraries: <f>: <reason>` | 127 own text | as reference |
| static program, static PIE | run (own status) | SIGSEGV | as reference |
| shared library without entry | see the hand-back | 127 relocation text | 127, "no entry point" |
| set-user-ID program file | runs, caller's identity | refused (option) | runs |
| options after the program name | program's arguments | n/a | as reference |

Layout-only differences in the libniji runs (not program mode): `libc.so.6` resolves from `<prefix>/lib` (the
probe copied it into lib/ and lib2/), so the RUNPATH-only case cannot find libc; `--list` names libniji's paths.

## Jobs (keke)

- `ldso-run-program-check3` (44ccf4e9a): clippy workspace all-targets all-features clean, `cargo xtask static` ok,
  `cargo xtask so` ok; ldso_program_options 5/5, ldso_link 9/9, ldso_run 7/7, search_cache 12/12, preload 4/4,
  no SKIP; probe cases and the dlopen probe (six cases, all as the reference) against the new loader; counts.
- `ldso-run-program-plants` (44ccf4e9a, handoff/LDSO-RUN-PROGRAM-plants.txt): 9 CAUGHT, 0 SURVIVED, each with its
  msg; WARN-DEV-SHM only. Includes the brief's two: an option silently ignored (libpath-, preload-, argv0-ignored,
  audit-silent) and program mode skipping a dependency's relocation (no-dependency-relocation).
- `ldso-run-program-gate` (direct tier, `--base origin/next`) and then `ldso-run-program-startup` (`cargo xtask
  test`, full-profile `so`, `hot_text_products`), sequenced by `~/work/ldso-run-program/src/seq.sh`: running at
  hand-back. Re-attach: `~/bin/jobwait ldso-run-program-gate ldso-run-program-startup`.
- Superseded: `ldso-run-program-check` (clippy: `-fd` arithmetic), `ldso-run-program-check2` (test harness passed
  LD_PRELOAD to timeout(1); counts run with LD_LIBRARY_PATH reaching perf). `ldso-run-program-baseso`: the base
  product for the counts and the "before" probe.

Start-up counts (Zen 1 host, user instructions, `perf stat -r 51` under `setarch -R`, empty `main` linked with each
loader, `env LD_LIBRARY_PATH=<products>` in front of both): base 1,032,374 / 1,032,373; branch 1,032,501 / 1,032,501
(+128, +0.012%). Program mode `ld.so ./e`: 1,045,859 -> 1,046,719 (+860, the cold path). The +128 comes from the
exec'd start's `--preload` chain in `Preloads::names`, the `--inhibit-cache` test in `List::ctx` and two `List`
fields. Fix-forward (not done: it would have invalidated the plants and gate): fold `--preload` into the
LD_PRELOAD value inside `program_mode` so `names()` stays byte-identical to the base. RULING NEEDED in the hand-back.
Layout: no libc.so.6 code changed; ld.so's changes are in its command path plus the lines above.

## Open / next

- Not implemented (differences row): `--audit` (needs an rtld-audit interface; crates/ld/src/audit.rs is a stub),
  `--inhibit-rpath`, the two hwcaps options, `--list-tunables`, `--list-diagnostics`. Each is a follow-up story.
- `--list` of a missing file and of a non-ELF file keep libniji's earlier texts (pre-existing; reference differs).
- `--verify` of a name without `/` gives 1 without a search (not probed).
- Release notes: the snapshot notes' known limitation for program mode can shrink to the six options above.
- Program mode's reasons for a file it cannot map (`image.rs` `load`, `MapError::message`) could take the
  dependency path's reference texts (`niji_ld::dlopen::Refusal::text`) where the two agree on the cause; suggested
  for LDSO-PROGRAM-OPTIONS-REST (it already holds the `--list` texts and the `--preload` fix-forward, #7513/#7514).
- Plan rows: D-27.69, D-27.14, D-27.13; D-27.80.
