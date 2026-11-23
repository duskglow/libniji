# PRINTF-NO-HEAP-3 (handoff; run 1, leg 4)

State: branch `task/printf-no-heap-3` (continues task/printf-no-heap-2 at 629dd2c71; merge base with origin/next
28843dea1, not merged). Code head `eb197a9be` (leg 4 changed tests only; printf code as at `32b6f8490`). Role
mei:~/w/s2 (Skylake-SP). Decisions in force: D-12.32, D-12.33; ruling #7808. Plants: handoff/PRINTF-NO-HEAP-3.plants,
results handoff/PRINTF-NO-HEAP-3-plants.txt. Predecessors: handoff/PRINTF-NO-HEAP-2.md (its cbench "before" is the
land bar). Report: handoff/reports/PRINTF-NO-HEAP-3.json.

## Commits (this story)
- `d61ef591a` dispatcher split (`format_routed` inline, out-of-line `format_planned`); `918114f6c` `vsnprintf`
  `#[inline(always)]`, `render_kept` removed; `98bd3673b` conformance s12_registry: a handler registered for a
  built-in letter writes it (probe-settled, both libraries); `32b6f8490` `vsnprintf` calls `format_routed` directly.
- `eb197a9be` (leg 4, coordinator 08:34Z): printf_no_heap.rs folded into crates/boundary/tests/s12_stdio_symbols.rs
  as module `no_heap` (one test binary fewer; test names, clause traces, probe record kept; failinject's allocator
  is now that binary's global allocator, per-thread, so its other tests allocate as before). New
  `no_heap::registered_conversion_is_written_by_its_handler` (s12_11_c_1; in a child): handlers for the built-in
  `d` and `s` write through `vsnprintf` ("[%d]", "%d", bare "%s") and through `vfprintf` on a stream whose plan
  cache kept "[%d]" before the registration. D-12.32/D-12.33 `tests` name the new file. Plant
  nh3-planned-skips-hooks now runs that Rust test (before: only conformance s12_registry caught it).

## Jobs on eb197a9be (mei)
- `printf-no-heap-3-lint-4`: clippy --workspace --all-targets --all-features -D warnings, then `cargo xtask static`: rc 0.
- `printf-no-heap-3-plants-4` (/mnt/zram/claude/printf-no-heap-3/plants-4): 3 CAUGHT, no WARN; nh3-planned-skips-hooks
  caught by `no_heap::registered_conversion_is_written_by_its_handler` (baselines of both commands green).
- `printf-no-heap-3-gate-4` (direct tier, 08:48-09:21Z, peak 12G): rc 1, 11863 passed, 12 failed = the six #7846
  tests below in both passes (catalogs_equal_the_reference, find_cached_answers_every_category_as_find,
  locale_name_found_before_its_alias, locale_behaviour_equals_the_reference, s7_2_f_0,
  printf_corpus_equals_the_reference); s12_stdio_symbols (with `no_heap`) green; conformance 137/137.
- `printf-no-heap-3-hot-text-4` (chain4.sh): rc 1, 32 passed, 1 failed: init_data_clusters_hold_at_every_text_phase,
  tool 59/60/59/59 at cold 0x800/0x2000/0x8000/0xF000, the same as leg 3's branch numbers (#7846, below).
- `printf-no-heap-3-xtask-test-4` (chain4.sh, mei ~/work/printf-no-heap-3/): still running at hand-back (09:33Z).
  Expected red only through host change #7846
  (leg 3's gate-3: locale_data (2), s07_locale_clauses s7_2_f_0, locale_products, printf_corpus_products,
  catalogs_products; hot_text `init_data_clusters_hold_at_every_text_phase`, tool 60 at cold 0x2000). Any other red
  is this story's.
- cbench `printf-no-heap-3-cbench-3` (leg 3, printf code unchanged since), instr/call base / now: snprintf_d
  489.6/487.4, ld_large 549.7/546.5, x08 530.8/526.6, s_path 229.2/201.2, s_d_colon 961.5/940.1, ls_line
  2422.9/2378.4, chk_ls_nlink 761.5/698.6, mixed 1998.7/1923.7, f2 1111.8/1059.4, g 1287.7/1237.3, e 1583.9/1531.4,
  f_large 18478.1/17813.1, fprintf_ls_line 1878.1/1874.2, du_line 842.7/839.3, fprintf_d 633.7/630.5. Every printf
  row at or below base.

## HOST CHANGE on mei, 2026-11-23 06:32Z (#7846; my flag #7860 answered "noted")
`locales-all` replaced /usr/lib/locale/locale-archive with 509 locale directories. Leg 3 traced every red above to it
(scripts mei ~/work/printf-no-heap-3/l3/, outputs /mnt/zram/claude/printf-no-heap-3/diag-l3): with no archive,
setlocale reaches the directory search (`niji::locale::name::candidates`, untraced, cold `.text` behind the hot
group) and at pad cold 0x2000 the hot group straddles a 64 KiB boundary (base 59/59/59/59, branch 59/60/59/59 at
0x800/0x2000/0x8000/0xF000); with an archive-only /usr/lib/locale (`unshare -r -m`) base and branch are 45 at all
four pads. No printf section faults differently; no printf code change is indicated (#4347).

## Leftover found in leg 4
Leg 3's `printf-no-heap-3-xtask-test-3` ended rc 124 (kill-label, 08:36:27Z), but its inner
`sh -c "cd ~/w/s2 && cargo xtask test"` (pid 79498, parent 1) kept running a nextest of the old head; leg 4 ended it
with `kill-mine s2 --kill` (08:46Z) before syncing.

## Exact next step
Collect plants-4, gate-4, hot-text-4 and xtask-test-4 on mei (`~/bin/jobwait <label>`); a red outside the #7846
list is this story's to fix. Then precheck and handback-check on the final head.

## RULING NEEDED
- Hosts: the hot_text and locale tests assume a locale archive; mei has none since 06:32Z. Recommend restoring it
  (or an explicit ruling on the build hosts' locale set); until then every gate on mei that selects them is red.
- Layout tracer: the directory-search instance of `candidates` runs in `tool`/`locale` on an archive-less host but
  is never traced, so its placement is chance. Recommend a layout story.
- LOCPATH: when the locale is absent from LOCPATH, the reference loads it from the default directory and libniji
  does not. Recommend a story to settle the spec and match.
- From leg 1, still open: layout planner head-bin tail (xtask/src/bins.rs `head_takes`).

## Follow-ups still open (from PRINTF-NO-HEAP-2)
#7764 (a) grouped/`I` floats past 128 bytes allocate; (b) registered conversions allocate. Not started.

## Not to redo
The +28 profile and its fixes; the leg-1 pre-0x2000 strings cause (planner); the leg-3 hot_text diagnosis; the
leg-4 fold.

## Refused
None.
