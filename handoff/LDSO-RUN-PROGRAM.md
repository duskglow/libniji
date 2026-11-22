# LDSO-RUN-PROGRAM: ld.so run as a program takes ld.so(8)'s options and runs static programs

State: branch `task/ldso-run-program` (from origin/next 90390071c). Commits: d9c103ffb (the change, tests, records,
plants), 44ccf4e9a (empty `--argv0` fix and a test-harness fix), then this note and the report. Role keke `~/w/r1`.

## What changed (pending record records/pending/LDSO-RUN-PROGRAM-1.toml, D-NEW in D-27)

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
- Plan rows: D-27.69, D-27.14, D-27.13; the pending record above.
