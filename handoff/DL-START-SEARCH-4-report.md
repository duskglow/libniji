# DL-START-SEARCH-4 report

Branch `task/dl-start-search-4` (on -3, head 958b4681c). Role keke:~/w/n3. Brief ~/state/briefs/DL-START-SEARCH-4.md.
CI red on 958b4681c (mei, FULL, snapshot with DL-TLS-DTOR-PIN-3) had three findings; outcome of each below.

## (1) sonames_products: `libc.so.6: cannot open shared object file` (RULING NEEDED)
* Reproduced on keke (job dl-start-search-4-so, script ~/work/DL-START-SEARCH-4/repro.sh): the test's program
  (`-nostdlib`, PT_INTERP = target/liella's loader, `-Wl,-rpath,<target/liella>`, so DT_RUNPATH, DT_NEEDED
  `libc.so.6`) exits 127. strace: the only open is `/lib/libc.so.6` (ENOENT); RUNPATH and the cache are never tried.
* Cause: `libc.so.6` is a pinned soname (`search::PINNED`). By R14-3 (a) and D-27.16 a pinned name resolves to
  `<prefix>/<libdir>` only, after `LD_LIBRARY_PATH` outside secure mode, and "never through RPATH, RUNPATH, the cache
  or another directory". target/liella is an unprefixed build (prefix `/`), so the pinned path is `/lib/libc.so.6`.
  The search behaves as decided; `--list` and `dlopen` already did so before this story.
* "Before your change those programs started" does not hold: with the base loader they hit the test's own
  `NO_SEARCH` branch ("the library search is not available in this build") and were skipped, which counts as a
  pass. This story removes that refusal, so the runs execute for the first time and fail on the pinning rule.
* The reference loader does not refuse this: it finds `libc.so.6` through DT_RUNPATH. So the brief's exception
  ("a test relied on something the reference also refuses") does not apply. Making the search find it means
  amending D-27.16 / R14-3 (a), a trust-boundary decision, so I did not change it.
* Options: (a) keep D-27.16; the SL-SONAMES test runs its dynamic programs with `LD_LIBRARY_PATH=<target/liella>`
  (R14-3's documented route for testing a libc build) and drops the NO_SEARCH skip. That test exists only on next,
  so this needs a merge of origin/next (not allowed by my brief) or a one-line story on next. The change is harmless
  before this branch lands, since the skip still fires. (b) Amend D-27.16 so a pinned name also follows the
  requester's RPATH/RUNPATH outside secure mode. (c) Resolve pinned names to the running loader's directory.
  Recommend (a).

## (2) localtime_r 41 > 40 faults (systemic; not chased)
* keke (Zen 1), this branch's fast products, `hot_text_products init_first_localtime_r` (job dl-start-search-4-ht1):
  40 and 40 page faults (bound 40), both tests pass. The probes are static `-nostdlib` programs: the loader's
  `link()` never runs in them. The branch adds no static data (diff checked) and no code on a static program's
  start-up path; the search and dlopen text is linked in (no section GC) but never runs.
* Coordinator note (#1307): next sits at the bound and two unrelated stories hit the same 41 > 40;
  LAYOUT-WINDOW-BINS-4 is the structural fix. Not chased, per that note.

## (3) Counts (keke, release products, perf mean of 30 runs, ASLR off; instructions:u / page-faults:u)
| program | base 2d5625a3e | -3 head 71db28f7f | cold-move commit 41343de52 (reverted) |
|---|---|---|---|
| p_none | 29,985 / 16 | 30,026 / 16 (+15 vs its base run) | 30,001 / 17 |
| p_path | 131,411 / 40 | 131,977 / 40 (+540) | 131,736 / 41 |
| p_soname | cannot start | 149,106 / 43 | 148,981 / 45 |
Base and cold-move: dl-start-search-4-counts (base products reused from dl-start-search-3-counts-2, same revision).
-3 head: dl-start-search-3-counts-3. The final head's loader code equals the -3 head's (71db28f7f/958b4681c), so its
counts are the -3 column; not re-measured on the final head.

## The cold move, tried and reverted (negative result)
`find_needed` (`#[cold]`, `#[inline(never)]`) took the bare-name branch out of `link`, and a scratch-store lookup
replaced the per-object `paths` array (a path-named object's path is its own name). Instructions fell (p_path 540 ->
325 above base), but faults rose by one on p_none and p_path, and by two on p_soname. One page fault costs more than
~200 instructions, so the commit was reverted (5e428c14a). The likely cause is the ld.so text layout: a new function
placed among the start path's functions moves one across a page boundary. ld.so has no traced hot group, unlike
libc. Next time, check where `find_needed` landed (`nm -n` on the loader) and pin the start path first. Not
recorded as a saturation row (budget); this paragraph is the record.

## Tests and plants
* crates/ld/tests/ldso_search.rs: new case "$ORIGIN of a library named by path" (`b/libmidp.so`, no DT_SONAME,
  RUNPATH `$ORIGIN/../a/inner`, named by its absolute path): Liella and the reference both exit 121. Its `--list` is
  not compared (see INFORMATIONAL).
* Plants: handoff/DL-START-SEARCH-4-plants.txt (the -3 six plus `path-named-object-origin-lost`).

## INFORMATIONAL
* `ld.so --list` prints an object needed by its path as `PATH => PATH`; the reference prints the path alone
  (`\tPATH (0x...)`). This is in `list()` (DL-SEARCH-CACHE), which this branch does not change; a list story should
  settle the line form, then enable the `--list` comparison for that case in ldso_search.
* An unprefixed build's pinned path is `/lib/libc.so.6`. On a host where that file is the host's C library, an
  unprefixed Liella loader would load that file as `libc.so.6`. Development builds should use LD_LIBRARY_PATH or a
  prefixed build.

## Jobs (keke)
* dl-start-search-4-lint0 (inherited head 958b4681c): clippy + static, rc 0.
* dl-start-search-4-so: products + repro of (1); dl-start-search-4-ht1: localtime fault tests, rc 0 (evidence).
* dl-start-search-4-lint1 (compile error, fixed), dl-start-search-4-lint2 rc 0, dl-start-search-4-quick (found the
  `--list` difference) and dl-start-search-4-counts: all on the reverted cold-move code.
* dl-start-search-4-gate, dl-start-search-4-plants: stopped by me (head reverted).
* Final head: dl-start-search-4-gate2 (clippy all targets, static, affected direct tier), dl-start-search-4-plants2.
