# DL-START-SEARCH: find DT_NEEDED libraries by soname at program start

**State (DL-START-SEARCH-4):** branch `task/dl-start-search-4` (on -3), role keke:~/w/n3. Report:
handoff/DL-START-SEARCH-4-report.md. Loader code = the -3 head's (a cold out-of-line move cut instructions but added
one page fault per start: reverted). New: ldso_search case "$ORIGIN of a library named by path" + its plant.
**Blocked on RULING:** the CI red in sonames_products comes from R14-3 (a)/D-27.16 (the pinned `libc.so.6` is never
found through RUNPATH). Those tests used to skip on NO_SEARCH, so their dynamic runs never executed before; the
recommended fix is for that test to set LD_LIBRARY_PATH (on next). localtime 41 > 40 is systemic (#1307,
LAYOUT-WINDOW-BINS-4). **Next step:** after the ruling and LAYOUT-WINDOW-BINS-4, re-run CI; the p_path surplus
(+540 instructions) needs a layout-aware design for ld.so text (report, "The cold move").

**State (DL-START-SEARCH-3):** branch `task/dl-start-search-3` (on -2), head 71db28f7f, role keke:~/w/n3. Report:
handoff/DL-START-SEARCH-3-report.md. The -2 red was the counts script (base cannot start p_soname); fixed. The counts
then showed the -2 code above base for programs that need no search (p_none +304, p_path +830 instructions); this run
removed the two costs every start paid (closure vtable relocations in ld.so, eager AT_EXECFN read). **Next step:**
read dl-start-search-3-counts-3; if p_path is still above base, the rest is in `link` (a 512-byte array copy and the
out-of-line `parse` of the loaded object, report "Where the -2 surplus went"): move the bare-name search state and
its code out of `link` into a `#[inline(never)]` function so `link`'s frame and codegen for path names return to base.

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
