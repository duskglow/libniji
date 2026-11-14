# DL-START-SEARCH: find DT_NEEDED libraries by soname at program start

**State (run 1, context checkpoint):** branch `task/dl-start-search`, code commit 14fc68682 (WIP), then this note.
Role mei:~/w/s2. Brief: ~/state/briefs/DL-START-SEARCH.md. Read first: handoff/DL-SEARCH-CACHE.md (the search, cache
and `--list` this builds on). Decisions in force: D-27.16, D-27.17, D-27.18, D-27.20, R14-3 (a), #4237, #5126.

## Done (code only, compiles, clippy clean; NOT VERIFIED by tests)
* `crates/ld/src/search.rs`: `Ctx.cache` is now `CacheRef` (`None` / `Read(&Cache)` / `Lazy(&dyn Fn)`), so program start
  reads the cache only when a search reaches the cache step, at most once (a `OnceCell` in `link`).
* `crates/ld/src/dlopen.rs`: `Refusal::skipped_at_start` and `Refusal::at_start` (the probed start-time rule, below).
* `crates/boundary/src/ld.rs`:
  - `read_cache()`: one cache reader for start, `--list` and `dlopen` (dlopen's `Env` uses it).
  - `open_candidate()` + `Unusable`: open, fstat (directory), size, map, `dlopen::check`, then the start rule.
  - `fail_loading()`: `<prog>: error while loading shared libraries: <what>: <text>`, status 127.
  - `List` gained `execfn` and `ctx()`; `run()` passes `&List` to `link()` with the program's origin (the path given to
    `ld.so`, else `AT_EXECFN`; threat model rule 2).
  - `link()`: bare names go through `search::search` with the requester chain (each object's first requester up to the
    program, `parent[]`), `$ORIGIN` = dir of the path an object was opened by (`paths[]`; found paths copied into a
    scratch store mapped at the first search hit, address space for OBJECTS_MAX x PATH_MAX). A bare name also matches a
    loaded object's DT_SONAME; a found object whose DT_SONAME is ld.so's is the start-time ld.so (#5126). Names with a
    slash keep `open_file` (no new work: no search context, no cache). Missing library, either kind: the reference text.
  - `list()`: same cache reader, same candidate rule, same message.

## Probe-settled reference behaviour (mei, black box; scripts in the run's scratch, re-creatable from this list)
* Missing bare or path-named DT_NEEDED: `<argv[0]>: error while loading shared libraries: <name>: cannot open shared
  object file: No such file or directory`, status 127 (`exec -a renamed` prints `renamed`; `ld.so ./prog` prints `./prog`).
* A candidate found first in a search: other machine (patched e_machine) or ELFCLASS32: skipped, search goes on.
  Unreadable (mode 000): skipped. Fatal, status 127, `<prog>: error while loading shared libraries: <path>: <text>`:
  200-byte non-ELF `invalid ELF header`; 14-byte file `file too short`; other OS ABI `ELF file OS ABI invalid`;
  big-endian `ELF file data encoding not little-endian`; ET_REL `only ET_DYN and ET_EXEC can be loaded`; directory
  `cannot read file data: Error 21`; PIE / ET_EXEC: subject is the NEEDED NAME, `cannot dynamically load
  (position-independent) executable`. `--list` and LD_TRACE_LOADED_OBJECTS end the same way (127), not a "not found"
  line. **This contradicts the brief's "non-ELF files are skipped as in list()"**: implemented as probed, and list()
  was changed to match (it used to skip anything Elf::parse rejected).
* A symlink in LD_LIBRARY_PATH to an already-found object: loaded once (dev, ino).

## Not done (exact next steps)
1. Wait for / read gate `dl-start-search-gate-1` (mei, direct tier, on 14fc68682 tree); fix what it finds.
2. Product tests (new file `crates/ld/tests/ldso_search.rs`, pattern from `ldso_run.rs` + `search_cache.rs::list_matches_reference`):
   a program needing a bare soname found via DT_RUNPATH, DT_RPATH, LD_LIBRARY_PATH, `$ORIGIN`, and a cache written by
   `liella_ld::cache::writer` in a test root (needs a co-install prefix build or a cache path the test controls: check
   how `search::prefixed` finds `etc/ld.so.cache` before designing; never the host's /etc/ld.so.cache); loaded objects
   == `ld.so --list` lines; secure child (set-group-ID, as ldso_run.rs) ignores LD_LIBRARY_PATH; missing message and
   127; one skip (other-machine) and one fatal (non-ELF) candidate. Name clause s27_1_c_1_1.
3. Plants file through `cargo xtask plants` (skip rule, lazy cache, chain parent, origin, soname match, message).
4. Counts: start-up instructions before/after for a dynamic hello named by path (no new work expected beyond the
   `parent`/`paths` array init and the `bare` test; if the ratchet sees it, move the arrays into the bare branch).
   Also the brief's start-up checks: `cargo xtask test` and `LIELLA_FULL_PROFILE=1` hot_text_products.
5. `cargo xtask static`; update docs/install-packages.md line 56 (still says the search is unavailable).
6. Try SL-FLAVOUR's case: Debian trixie dynamic hello in a system root under Liella's ld.so.

## Open
* Out of scope, unchanged: #5510 (libc.so.6 lacks DT_NEEDED on the interpreter), #5524 (start-time l_name = DT_SONAME).
* `dlopen`'s `Refusal::skipped(searched=true)` also skips Data and OsAbi; at start the reference does not. Not
  touched (dlopen's rule was probed separately); worth a look by a dlopen story.
