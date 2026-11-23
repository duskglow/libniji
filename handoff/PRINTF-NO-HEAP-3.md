# PRINTF-NO-HEAP-3 (handoff; run 1, leg 3)

State: branch `task/printf-no-heap-3` (continues task/printf-no-heap-2 at 629dd2c71; merge base with origin/next
28843dea1, not merged). Code head `32b6f8490` (leg 3 changed no code). Role mei:~/w/s2 (Skylake-SP). Decisions in
force: D-12.32, D-12.33; ruling #7808. Plants: handoff/PRINTF-NO-HEAP-3.plants, results
handoff/PRINTF-NO-HEAP-3-plants.txt. Predecessors: handoff/PRINTF-NO-HEAP-2.md (its cbench "before" is the land bar).

## Commits (this story)
- `d61ef591a` dispatcher split (`format_routed` inline, out-of-line `format_planned`); `918114f6c` `vsnprintf`
  `#[inline(always)]`, `render_kept` removed; `98bd3673b` conformance s12_registry: a handler registered for a
  built-in letter writes it (probe-settled, both libraries); `32b6f8490` `vsnprintf` calls `format_routed` directly.

## Final head 32b6f8490
- clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static` green (leg 2).
- cbench `printf-no-heap-3-cbench-3` (instr/call, base / now): snprintf_d 489.6/487.4, ld_large 549.7/546.5, x08
  530.8/526.6, s_path 229.2/201.2, s_d_colon 961.5/940.1, ls_line 2422.9/2378.4, chk_ls_nlink 761.5/698.6, mixed
  1998.7/1923.7, f2 1111.8/1059.4, g 1287.7/1237.3, e 1583.9/1531.4, f_large 18478.1/17813.1, fprintf_ls_line
  1878.1/1874.2, du_line 842.7/839.3, fprintf_d 633.7/630.5. Every printf row at or below base.
- plants `printf-no-heap-3-plants-3`: 3 CAUGHT (nh3-kept-count, nh3-planned-skips-hooks, nh3-bare-s-count), no WARN.
- gate `printf-no-heap-3-gate-3` (direct tier): RED only on locale-dependent tests: locale_data (2),
  s07_locale_clauses s7_2_f_0, locale_products, printf_corpus_products, catalogs_products (see HOST CHANGE).
- hot_text `printf-no-heap-3-hot-text-3`: RED, 1 test (tool 60 at cold 0x2000, 59 elsewhere): explained below.
- `printf-no-heap-3-xtask-test-3` (`cargo xtask test`): see the hand-back (queued behind the gate in chain3.sh).

## HOST CHANGE on mei, 2026-11-23 06:32Z (flagged #7860)
`locales-all` installed: /usr/lib/locale/locale-archive is gone, 509 locale directories instead. Base products
(/mnt/zram/claude/printf-no-heap-2/base) were built 06:15 (archive), the branch's after. Effects seen:
1. locale_data (2 tests) assert "the gate machines have a locale archive".
2. s7_2_f_0 expects `zh_HK.big5hkscs` to fail; both libraries now load it (codeset BIG5-HKSCS).
3. printf corpus lines 6701-6704 (en_US.utf8): the harness runs both libraries with LOCPATH=target/test-locales,
   which lacks en_US.utf8. Probe (mei ~/work/printf-no-heap-3/loc.c): with LOCPATH naming a directory without the
   locale, the reference loads /usr/lib/locale/<name>/; libniji returns NULL, errno ENOENT. Without LOCPATH both load
   it. CI's role on mei shows the same three reds (07:29Z). locale_products, catalogs_products: locale-dependent,
   cause not verified.
4. hot_text, `init_data_clusters_hold_at_every_text_phase`, mode `tool`. With no archive, setlocale(LC_ALL, "")
   reaches the directory search, `niji::locale::name::candidates::<Data, find_named's in_dir>`
   (crates/core/src/locale/mod.rs `find_named`). hot-text.signatures lists no `candidates` instance (base or branch),
   so it sits in cold `.text` right behind the hot group. The branch's hot group changed size (printf bins), so at
   pad cold 0x2000 it straddles the 64 KiB boundary 0x580000 (fault map: `.text` 7 against 6, ip 0x57ffff). No
   printf section faults differently. Measured on mei now (min of 5 runs): base 59/59/59/59 at cold
   0x800/0x2000/0x8000/0xF000, branch 59/60/59/59. Inside `unshare -r -m` with /usr/lib/locale bound to an
   archive-only directory (en_US.UTF-8, system localedef), the state the test was written for: base and branch
   45 at all four pads. Scripts: mei ~/work/printf-no-heap-3/l3/ (diag3.sh, inner4.sh, run.py); outputs
   /mnt/zram/claude/printf-no-heap-3/diag-l3 (fault maps `*.fm`).

## Exact next step
Coordinator: restore an archive on mei (or rule the hosts' locale set), then rerun on 32b6f8490: hot_text
(`NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`), the gate
and `cargo xtask test`. No code change is indicated by this story's evidence; moving printf sections to dodge the
boundary would be chasing jitter (#4347).

## RULING NEEDED
- Hosts: the hot_text and locale tests assume a locale archive; mei has none since 06:32Z. Recommend restoring it
  (or an explicit ruling on the build hosts' locale set); until then every gate on mei that selects them is red.
- Layout tracer: the directory-search instance of `candidates` runs in `tool`/`locale` on an archive-less host
  but is never traced, so its placement is chance. Recommend a layout story (trace with the measured modes'
  locale source, or key start-up generic instances).
- LOCPATH: when the locale is absent from LOCPATH, the reference loads it from the default directory and libniji
  does not. Recommend a story to settle the spec and match.
- From leg 1, still open: layout planner head-bin tail (xtask/src/bins.rs `head_takes`).

## Follow-ups still open (from PRINTF-NO-HEAP-2)
#7764 (a) grouped/`I` floats past 128 bytes allocate; (b) registered conversions allocate. Not started.

## Not to redo
The +28 profile and its fixes; the leg-1 pre-0x2000 strings cause (planner); the leg-3 hot_text diagnosis above.

## Refused
None.
