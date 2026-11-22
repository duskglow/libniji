# LDSO-PROGRAM-OPTIONS-REST: program mode's cost off the normal start, the small options

State: branch `task/ldso-program-options-rest` (from origin/next 1d4d07280), role mei `~/w/s2` (Skylake-SP class).
Commits: 25a27d6ff (the change, tests, plants file), 260006817 (ldso_run expects "invalid ELF header").
Leg 1 ended by REFRESH at ~150k context; scratch on mei `~/work/ldso-program-options-rest/` (probe.sh, counts.sh,
trace.sh, ref.txt = the reference probe output, base/ and head1/ = product copies), counts dir
`/mnt/zram/claude/ldso-program-options-rest/c1` (e, t, product dirs).

## Done (25a27d6ff)

- Step 1 code: `Preloads` has no `option` field again (`names()` = LD_PRELOAD's then the file's); `List` lost
  `preload_option`/`inhibit_cache`; `List::ctx` passes `cache` through. Program mode folds `--preload` into the
  LD_PRELOAD value (`entry::fold_preload`, cold, private mmap kept for the process) and keeps the option's range
  (`state::keep_preload_option`/`in_preload_option`) for the "from --preload" diagnostic. `read_cache` returns
  None after `--inhibit-cache` (`state::cache_inhibited`); program_file and dlopen's Env::read_with rely on it.
- Step 3: `image::open_program` + `fail_program` (cold) hold run mode's file checks; `load` uses them and refuses
  a non-ELF file with "invalid ELF header"; `trace::list` with strict (`--list`) runs the same checks first.
- Step 4: tests in `crates/ld/tests/ldso_program_options.rs`: `ldso_program_list_of_files_that_cannot_be_listed`,
  `ldso_program_dlopen_after_library_path_and_inhibit_cache` (program linked with libniji crt1/libc.so.6/ld.so,
  `--library-path lib:<niji>`), notelf_long run case, LD_PRELOAD+--preload missing-name sources.
- Plants file: handoff/LDSO-PROGRAM-OPTIONS-REST-plants.txt (8 plants; NOT RUN yet). `inhibit-ignored` will
  probably SURVIVE: no test host has a libniji loader cache, so --inhibit-cache has no in-tree observable.

## Reference probe (Skylake-SP host, installed 2.39 ld.so; ref.txt on mei)

- `--list-tunables` [PROGRAM]: one line per tunable `name: value (min: x, max: y)` (sizes hex, counts decimal),
  status 0, program ignored. `--list-diagnostics` [PROGRAM]: `key=value` lines (strings quoted, numbers hex), 0.
- `--inhibit-rpath LIST`: only an empty entry (`''`) took effect (the main program's RUNPATH then ignored, dep not
  found, 127); names `prog_runpath`, `./prog_runpath`, absolute path, `libd.so`, `x:libd.so` changed nothing.
  Not probed yet: the full path of a dependency (`$W/dlib/libd.so`), `':'`, `'a:'`.
- `--glibc-hwcaps-prepend LIST`: named subdirs searched first; last copy wins; '' = none. `--glibc-hwcaps-mask
  LIST`: built-in subdirs limited to exact names (`x86-64-*` matched none), '' = no subdir; prepend not masked;
  last copy wins. All four options without a value: status 1, unrecognized-option text.
- `--list` missing / text<64 B / directory / non-ELF: `<f>: error while loading shared libraries: <f>: <reason>`
  127, reasons `cannot open shared object file: No such file or directory`, `file too short`, `cannot read file
  data: Error 21`, `invalid ELF header`. Running a non-ELF file: same, "invalid ELF header" (probed).

## OPEN: the counts went UP, not down (must be explained before anything lands)

Skylake-SP, counts.sh (median of 51, min=max within 2): base (origin/next 1d4d07280) empty main 549,052, three
libraries 798,034; head1 (25a27d6ff's code) 549,150 (+98), 798,072 (+38). Relocations and dynsym identical;
ld.so +352 bytes; start-path function sizes: run -112 B, link -268 B, niji_ld_start -16 B, read_cache +16 B.
Mixed sets (one ld.so with the other build's libc.so.6) exit 127: invalid split. Intel PT gives no data on mei (VM).
Next: exact per-function counts with valgrind callgrind (`--trace-children=yes`, `env -i LD_LIBRARY_PATH=...`),
or `perf stat` of read_cache alone; suspects: link()'s codegen after the Preloads change, the read_cache check.
Fix so the exec'd start is <= base (the brief: equal to before LDSO-RUN-PROGRAM, i.e. about -128 from base),
then count Zen 1 (copy products to keke ~/work, run counts.sh there; no role needed for counting).

## Next steps (in order)

1. Explain and fix the +98/+38; recount both classes (base dirs already built on mei).
2. Jobs on the final head: plants (`cargo xtask plants handoff/LDSO-PROGRAM-OPTIONS-REST-plants.txt --dir
   /mnt/zram/claude/ldso-program-options-rest/plants-1`), direct gate (`cargo xtask affected --run`), start-up
   checks (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && ... --test hot_text_products`), clippy,
   static, precheck. check1 (25a27d6ff): clippy+static clean, ldso_program_options 7/7, ldso_link 9/9, ldso_run
   6/7 (fixed by 260006817), preload/search_cache not reached.
3. Step 2 (five options) not started: if no budget, hand back with it as the next story (design: hwcaps options
   via a `hwcaps` slice computed once per List would also save the per-ctx `Native::hwcaps` call; inhibit-rpath =
   empty entry matches the main program). Differences row (docs/differences-from-glibc.md:337) unchanged so far.
4. handoff/reports/LDSO-PROGRAM-OPTIONS-REST.json, handback-check.

Plan rows: D-27.80 (LDSO-RUN-PROGRAM), D-27.69, D-27.14, D-27.13. Rulings #7513-#7516.
