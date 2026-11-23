# BUILD-REPRODUCIBLE-PATHS: handoff (run 1, leg 3; final)

Branch `task/build-reproducible-paths`, base origin/next 9d12e7ee3. Code commits: f9d03aedc (path fix), 565671f9e
(count fix), a3511ce6b (D-X.159 + docs/benchmarks.md), 9a476134f (check_dir test reads unlisted files; plants file),
d5551955c (`--counters-only` runs the COUNT-SPREAD check), 79e80e1c6 (layout tests read the relative marker). Role mei:~/w/s1. Decision in force: **D-X.159**.
Scratch: mei and keke `~/work/build-reproducible-paths/` (build-scan.sh, gdbprobe/, repcount.sh, spread.py,
before-/after-/fix-*.tsv, manifest-s1, manifest-treeb, manifest-kkrel, lint3.sh).

## Item 1: products byte-identical whatever path builds them (DONE, verified)

* Cause: the C compiler records the logical `$PWD` (role dirs are reached through a symlink) as `DW_AT_comp_dir`;
  19 utilities and small libraries and `.hot-lists` carried it. Fix: every spelling of the root (given, canonical,
  logical `$PWD` ancestor) gets `-ffile-prefix-map=<s>=/niji` (`align::root_spellings`, `so::debug_prefix_flags`);
  `.hot-lists` relative; `remap::check_dir` scans every top-level product file.
* Proof (release profile, head 96f0f1b2a: product inputs unchanged since; later commits touch xtask bench/remap tests,
  bench/ and docs only): three builds, three manifests, **one sha256 b82220ab...86e9, 1339 lines**:
  mei ~/w/s1 (symlinked role dir; job `build-reproducible-paths-so2`, release via the bench build), mei
  /mnt/zram/.../second/tree-b (plain dir; job `-treeb`, `NIJI_FULL_PROFILE=1`), keke /dev/shm/.../kk/tree-c (Zen 1;
  job `-kk-rel`). Each: scan of every product file for $HOME, both root spellings, /mnt/zram: 0 hits; layout
  `PRODUCT-PATH-REMAP ok: 47 products`. (Leg 2's keke manifest differed only because it was a `fast` build.)
* gdb (job `-gdb-treeb`, tree-b products): `bt` from a qsort comparator: 15 libniji frames with file:line
  (`boundary/src/posix/s14_search.rs:119`, `core/src/search.rs:355`, `legacy/src/search_c.rs:79`, ...,
  `qsort () at boundary/src/posix/s14_search.rs:80`), "Compilation directory is /niji/crates"; after
  `set substitute-path /niji <tree>`, `list` prints the source and "Located in <tree>/crates/boundary/...".

## Item 2: bench counts repeat (DONE; repeat-run result below)

* Causes: (a) event 4 (user instructions) outside the pinned group, multiplexed (one row 0.001 to 110 per call over
  20 runs); (b) ASLR: strstr_hit_64 / strstr_miss_64 (gated) 71 or 135 user instr/call, strcasestr_hit_64 3.7 %
  (gated); exact under `setarch -R`. Fix: event pinned; the counter header sets ADDR_NO_RANDOMIZE and re-execs when
  CBENCH_COUNTERS is set; `--count-runs N`; `check_count_spread` (COUNT-SPREAD: a limited row moving more than
  max(2, 2 %) fails), now also in `--counters-only` (d5551955c: before it, a counters-only run never checked).
* After the fix (leg 2, 20 memref + 6 opt runs): user column exact on every memref row; gated rows beyond tolerance 0.
  Left, ungated: memcpy_64k 7.3 %, memset_64k 2.7 % (kernel share), gettext_language_steady 14.6 %,
  gettext_language_alternating 4.2 %, ngettext_found 0.5 %, re_mt4_* (threads).

## Jobs on the final code head 79e80e1c6 (mei)

79e80e1c6 fixes what the first gate found: `.hot-lists` names its directory relative to target/niji (f9d03aedc), and
`hot_text_products` joined it against the test's own directory (4 tests red in `-gate`); the tests now resolve a
relative line against target/niji (an absolute one from an older marker still works).

* `build-reproducible-paths-lint5`: clippy -D warnings ok, static ok, records check ok, 11 branch unit tests ok.
* `build-reproducible-paths-gate2`: `cargo xtask affected --run --base origin/next` (tier Direct; the train runs
  FULL for the xtask changes): 7 suites ok, 664 passed, 0 failed; peak 2.2G.
* `build-reproducible-paths-plants3`: 3 plants, 3 CAUGHT with their messages, 0 WARN (`handoff/BUILD-REPRODUCIBLE-PATHS-plants.txt`).
* `build-reproducible-paths-repeat2` (PENDING at hand-back): `cargo xtask bench --counters-only --count-runs 5 --only
  cbench_memref,cbench_hot,cbench_opt,cbench_locale,cbench_malloc,cbench_stdio,cbench_time`; green = rc 0 and the
  line `bench: COUNT-SPREAD ok: N rows with an instruction limit repeated ...`; red names each row as COUNT-SPREAD.

SUPERSEDED: `-lint3`, `-lint4`, `-plants`, `-plants2` (3/3 CAUGHT on 9a476134f and d5551955c), `-gate` (d5551955c,
4 hot_text_products tests red: the relative marker, fixed in 79e80e1c6), `-repeat` (10 runs on 9a476134f; killed in
cbench_hot: a counters-only run did not check the spread then, the gap d5551955c fixes).

## Open / next

* Not planted: the `--counters-only` wiring (d5551955c) and the counter header's re-exec (bench C code); a plant needs
  a full bench run and the ASLR path shows only in some runs. Covered by the repeat run.
* Start-up checks (§9, `cargo xtask test` and the NIJI_FULL_PROFILE hot_text run) not run: no start-up code
  changed; the change is build flags (path prefix maps) and the marker's spelling. The gate's layout-check and
  hot_text_products/cold_bss/heap_meta product tests passed.
* Next step: collect `build-reproducible-paths-repeat2` (`~/bin/jobwait build-reproducible-paths-repeat2` on mei);
  green: land via the train. A COUNT-SPREAD row: read its range in the log and find its cause before any limit change.
