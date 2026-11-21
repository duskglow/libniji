# LDSO-PROGRAM-NO-CWD: `ld.so PROGRAM` without a slash never opens PROGRAM from the working directory

State: branch `task/ldso-program-no-cwd` (from origin/next 305065b07). Code head 203aca154 (three commits: the change,
dropping a dead check, then the test fix for the surviving trace plant, leg 2). Role keke `~/w/r1`, synced at that
head (tree 96cb0ea5).

## What changed (D-27.69, reserved this run)

- `crates/ld/src/program.rs`: `Found` and `find` (pure). A name with `/` is `AsGiven`; else the loader cache only
  (`Cache::lookup` with `Arch::CACHE_FLAGS` and the hwcaps list); a miss is `NotFound`. The cache reader already
  refuses a relative path (an inconsistent entry), so no extra check is needed.
- `crates/boundary/src/ld/image.rs`: `program_file` (`#[cold] #[inline(never)]`). It returns the name when it holds
  `/`. Otherwise it reads the cache, keeps the found path in a `PathStore` (NUL-terminated, never unmapped) and
  unmaps the cache. Not found: `NAME: error while loading shared libraries: NAME: cannot open shared object file`,
  127 (`fail_loading`). An empty name gets the probed text without the name.
- `crates/boundary/src/ld/entry.rs`: the `Action::Program` arm resolves the file first, then uses it for the trace
  list, `load` and `run` (so messages and `$ORIGIN` follow the file; `argv[0]` stays as given). `Action::List`
  resolves the same way. The kernel-start path is untouched.
- Records: D-27.69 in records/decisions/D-27/D-27.69.toml (the generated view plan/07-decisions/s26-29.md is the
  train's; leg 2 moved the row there after precheck's train-owned red); the probe record in spec/behaviour/dynlink.md (after
  "Loader exit status and messages"); spec/security/threat-model.md rule 2 (origin of a cache-found program) and
  rule 3 (no working-directory open). No differences row or compat flag: the behaviour now matches the reference.
- Tests: `crates/ld/tests/search_cache.rs` `program_find_cache_only` (pure) and `ldso_program_without_slash`
  (product; every probe case; the cache-hit cases run under `unshare -r -m` with a private cache bound over
  `<prefix>/etc/ld.so.cache`, and skip with the reason when that is unavailable). The working directory's
  `libprogx.so.1` is a non-object file, so a run, `--list` or trace of it instead of the cache's file differs; the
  traced cache hit exports `LD_TRACE_LOADED_OBJECTS` only for the loader (leg 2; reference probed: run, `--list`
  and trace all read the cache's file there, `statically linked`, 0).
- Plants: handoff/LDSO-PROGRAM-NO-CWD-plants.txt (6).

## Probe table (black box, Zen 1 host; probe scripts in the run's scratch, not in the tree)

| case (cwd holds `prog`) | reference | libniji before | libniji after (first check run, 58bcb3e96's parent) |
|---|---|---|---|
| `ld.so prog`, nothing else set | 127, `prog: error while loading shared libraries: prog: cannot open shared object file` | ran ./prog | as reference |
| LD_LIBRARY_PATH `.`, `:`, `lib:` | 127, same text | ran ./prog | as reference |
| name only in an LD_LIBRARY_PATH dir (absolute or relative) | 127, same text | 127 `ld.so: X: cannot open the program` | as reference |
| `--library-path DIR` | 127, same text | unrecognized option (unchanged; option not implemented) | unchanged |
| name only on PATH | 127, same text | 127, own text | as reference |
| absent | 127, same text | 127, own text | as reference |
| empty name | 127, `: error while loading shared libraries: cannot open shared object file` | 127, own text | as reference |
| name only in a default lib dir (not in the cache) | 127, same text | 127, own text | as reference (the test does not bind a default dir) |
| name in the cache (private cache via `ldconfig -C`, `unshare -r -m`) | runs the cache's file, argv[0] = name as given, also with cwd and LD_LIBRARY_PATH copies | ran the cwd copy | first run: FAILED only because the test cache also held a relative entry; fixed in 58bcb3e96 |
| `--list prog`, `LD_TRACE_LOADED_OBJECTS=1 ld.so prog` | 127, same text | `statically linked` from the cwd file | as reference |
| `./prog`, `lib/prog` | runs, argv[0] as given | same | same |

The first check job (ldso-program-no-cwd-check, superseded) passed clippy, static, `cargo xtask so`, and every
not-found case; only the two cache-hit asserts failed, for the test-cache reason above.

Not probed: the reference's `$ORIGIN` for a cache-found program (libniji: the directory of the cache's path).

## Jobs

On 58bcb3e96 (superseded by the test fix): `ldso-program-no-cwd-check2` green (clippy, static, so, search_cache
18 passed, ldso_run); `ldso-program-no-cwd-gate` green (direct tier, 5 of 307 targets, 180 passed, peak 1.6G);
`ldso-program-no-cwd-plants` stopped by me after trace-unresolved SURVIVED (find- and boundary-slash-free-as-given
CAUGHT). Older: `ldso-program-no-cwd-check` (test-cache bug).

On 203aca154 (keke): `ldso-program-no-cwd-check3` green (clippy, static, so, search_cache 11 passed with
--nocapture, no SKIP); `ldso-program-no-cwd-gate2` green (direct tier, 5 of 307 targets, 180 passed, 0 failed,
peak 1.5G); `ldso-program-no-cwd-plants2` 6 of 6 CAUGHT with their msg (WARN-DEV-SHM only). Started after the
doc-only head's sync: `ldso-program-no-cwd-startup` (stopped, superseded), then on 691f58853 (records-only change)
`ldso-program-no-cwd-startup2` (cargo xtask test, full-profile so, hot_text_products). precheck on 691f58853: no red,
WAIT on startup2 only.

Layout: `target/product/fast/layout-bins.tsv` lists only libc.so.6's hot group; the change is confined to ld.so's
command path (`Action::Program`/`List`, new code in one `#[cold] #[inline(never)]` function), so no bin moves;
no primed base product to diff on keke.

## Exact next step

1. On keke: `~/bin/jobwait ldso-program-no-cwd-startup2` (check3, gate2, plants2 already green)
   (a label not yet started is refused: wait on the started ones). Fix a red, then run only that suite again.
2. Plants: every plant CAUGHT with its msg; list any WARN lines (WARN-DEV-SHM seen).
3. `cargo xtask precheck --jobs <labels> --head <sha>` on the role, then handoff/reports/LDSO-PROGRAM-NO-CWD.json
   (with `jobs`), push, `ops handback-check`, hand back.

## Hand-back notes

- INFORMATIONAL: `--library-path` (and other ld.so options) are not implemented in libniji's ld.so; the probe shows
  the reference ignores the option's directory for a slash-free program name anyway.
- IMPROVEMENT: the brief's `ld.so prog with --library-path` case and the "library path" case were the same answer
  (the reference uses neither); a one-line probe of the reference before writing the brief would have stated the
  rule (cache only) up front.
