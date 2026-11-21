# DL-START-SEARCH-2 report

Branch `task/dl-start-search-2` (stacks on `task/dl-start-search`). Role mei:~/w/s1.

## Commits (this run)
* c974c9443 Test start-time soname search against the reference loader (crates/ld/tests/ldso_search.rs, docs).
* 6b653d0e9 Name the test case type and fix the plants file separator.
* (this note, the plants results and the report json.)
Inherited and unchanged: 14fc68682 (the code), 98939ef46 (run 1's handoff).

## Step 0 (inherited head)
`cargo xtask clippy` + `cargo xtask static` on 98939ef46: green (job dl-start-search-2-lint0). The predecessor's gate
dl-start-search-gate-1 was rc 0; nothing to fix.

## Jobs (mei)
* dl-start-search-2-lint1: clippy + static on the final tree, rc 0.
* dl-start-search-2-gate-2: `cargo xtask affected --run` (direct tier), rc 0: 5 suites ok, 131 tests passed, 0 failed
  (liella-ld::ldso_search selected), peak 2.0G.
* dl-start-search-2-plants-3: `cargo xtask plants handoff/DL-START-SEARCH-2-plants.txt`, rc 0: 6 plants, 6 CAUGHT,
  0 SURVIVED, no WARN lines. The secure rule has no non-equivalent one-site plant (see the plants file).
* dl-start-search-2-startup: the brief's start-up checks (`cargo xtask test`, then `LIELLA_FULL_PROFILE=1 cargo xtask
  so` and `hot_text_products`); `cargo xtask test` runs the whole suite (6589 tests), so it is left running for the
  dispatcher.
* dl-start-search-2-counts: start-up counts, below.
* Superseded: dl-start-search-2-t1 (first run of the new tests: a library link dropped by the toolchain's
  --as-needed default; fixed), dl-start-search-2-gate (clippy type_complexity on the new test; fixed),
  dl-start-search-2-plants (refused: a literal containing ` | `), dl-start-search-2-plants-2 (stopped by me: tree
  before the clippy fix).

## Start-up counts (brief item 4)
See the COUNT lines in ~/logs/dl-start-search-2-counts.log on mei and the numbers in the hand-back / report json:
`-nostdlib` program with no DT_NEEDED (p_none), one DT_NEEDED by path (p_path), one bare soname via DT_RUNPATH
(p_soname); release-profile products of origin/next (2d5625a3e) and of this head; instructions:u and page faults,
perf mean of 30 runs, address randomisation off. Before this branch p_soname did not start (refused).

## Tests added (crates/ld/tests/ldso_search.rs, clause s27_1_c_1_1)
* ldso_search_steps_match_reference: 11 cases, Liella and the reference loader on the same files: status, the
  127 message text, and `--list` `=>` lines equal.
* ldso_search_secure_ignores_library_path: set-group-ID start (AT_SECURE probed by a second program) runs the
  DT_RUNPATH library, not the LD_LIBRARY_PATH one.
* ldso_search_cache_opened_once_when_reached: strace: 0 cache opens for a path-named DT_NEEDED, exactly 1 for a
  cached system library followed by a missing one (127 message checked).

## Limits, informational
* The cache step is shown on the host's cache (a `/`-prefix product's cache path is compile time); a product test on a
  cache written by `liella_ld::cache::writer` would need a prefixed product build (not done; cost of a second build).
* SL-FLAVOUR's Debian trixie dynamic hello was not run (no Debian system root prepared in this run). The counts job
  runs the host's dynamic `/bin/echo` under the head's ld.so with LD_LIBRARY_PATH at Liella's libraries as a smoke.
* dlopen's `Refusal::skipped(searched=true)` skips Data and OsAbi candidates; at start the reference refuses them.
  Unchanged here (dlopen's rule was probed separately).
* Process: `cargo xtask test` (the brief's start-up check) is the full 6589-test suite; on a loaded Zen 1 host it runs
  for hours, far beyond the other jobs. A products-only entry (the layout contracts) would give the same start-up check
  in minutes.
