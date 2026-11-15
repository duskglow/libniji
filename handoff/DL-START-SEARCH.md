# DL-START-SEARCH: find DT_NEEDED libraries by soname at program start

**State (DL-START-SEARCH-2):** branch `task/dl-start-search-2` (stacks on `task/dl-start-search`: code commit
14fc68682, then tests c974c9443 and 6b653d0e9). Role mei:~/w/s1. Briefs: ~/state/briefs/DL-START-SEARCH.md ("The
change"), ~/state/briefs/DL-START-SEARCH-2.md. Read first: handoff/DL-SEARCH-CACHE.md. Decisions in force: D-27.16,
D-27.17, D-27.18, D-27.20, R14-3 (a), #4237, #5126. Full report: handoff/DL-START-SEARCH-2-report.md.

## What the branch does
* `crates/ld/src/search.rs`: `Ctx.cache` is `CacheRef` (`None` / `Read` / `Lazy`): program start reads the cache only
  when a search reaches the cache step, at most once (a `OnceCell` in `link`).
* `crates/ld/src/dlopen.rs`: `Refusal::skipped_at_start` / `Refusal::at_start` (the probed start-time candidate rule).
* `crates/boundary/src/ld.rs`: `link()` resolves names without `/` through `search::search` with the requester chain
  (`parent[]`), `$ORIGIN` of the program (path given to ld.so, else AT_EXECFN) and of each found object (`paths[]`, a
  scratch store mapped at the first hit); a bare name also matches a loaded DT_SONAME; `open_candidate()` +
  `fail_loading()` (reference text, status 127); `read_cache()` shared by start, `--list` and `dlopen`; `list()` uses
  the same candidate rule. Names with a slash keep `open_file` (no search context, no cache).
* `crates/ld/tests/ldso_search.rs` (clause s27_1_c_1_1): 11 cases (each step, RPATH before LLP, LLP before RUNPATH,
  requester chain, RUNPATH requester-only, `$ORIGIN` of program and of a found library, other-machine skip, non-object
  fatal, missing) against the reference loader (status, message, `--list` lines); set-group-ID start ignores
  LD_LIBRARY_PATH; strace: no cache open for path names, one open for two searches that reach it.
* docs/install-packages.md: the "dynamic programs stop at start" paragraph replaced.

## Probe-settled reference behaviour (black box, mei; now checked in the gate by ldso_search.rs)
* Missing DT_NEEDED: `<argv[0]>: error while loading shared libraries: <name>: cannot open shared object file: No such
  file or directory`, status 127.
* Candidates: other machine / ELFCLASS32 / unreadable skipped; non-ELF, short, other OS ABI, big-endian, ET_REL,
  directory, executable are fatal (127, `<path>: <text>`; executables name the NEEDED name). `--list` the same.
  This contradicts the original brief's "non-ELF files are skipped as in list()"; implemented as probed.

## Not to redo
* Product test for a cache written by the cache writer: a `/`-prefix product reads the host's /etc/ld.so.cache (compile
  time path), so the start-time cache step is shown on the host cache (libgcc_s) plus strace; the writer's cache is
  covered by cache_write.rs / search_cache.rs. A writer-cache product test needs a prefixed product build.
* A one-site plant on the secure rule is equivalent (LLP cleared from the environment, then dropped twice more).

## Open / next step
* SL-FLAVOUR's case (Debian trixie dynamic hello in a system root) was not run: next step for SL-FLAVOUR once this lands.
* Out of scope, unchanged: #5510 (libc.so.6 lacks DT_NEEDED on the interpreter), #5524 (start-time l_name = DT_SONAME).
* dlopen's `Refusal::skipped(searched=true)` also skips Data and OsAbi; at start the reference does not (dlopen's rule
  was probed separately); worth a look by a dlopen story.
