# LAYOUT-LDSO-ORDER: ld.so gets a traced, ordered start-up layout like libc

State: branch `task/layout-ldso-order-3` (continues `task/layout-ldso-order-2` and `task/layout-ldso-order`), run 3
(second session). Base origin/next 627ef49ca. Code head a530f933b. Role mei:~/w/s2 (run 3 first session: mei:~/w/s3;
runs 1-2: keke:~/w/n1).
Brief: ~/state/briefs/LAYOUT-LDSO-ORDER-2.md ("## The change"), narrowed by ~/state/briefs/LAYOUT-LDSO-ORDER-3.md.
Decision: records/pending/LAYOUT-LDSO-ORDER-2-1.toml (D-NEW, series D-X; CI assigns the id).
Rulings in force (coordinator 11-18): #6504 the N=0 +347 instructions (0.004%) are an accepted measured cost against
-3 faults; #6505 the text ordering lands; the insensitivity property, ld.so data and the 64 KiB window phase move to
the new story LAYOUT-LDSO-WINDOW. **This story does not claim insensitivity to unrelated ld.so growth.**

## Done
* Run 1: tracer mode `HOT_TEXT_LDSO`, `tools/hottext/ldso_trace.sh` (`true`, `libs3`; slow and numa1 traces must
  equal), `group_ldso`/`ldso_lists` in xtask/src/hottext.rs, traced link in xtask/src/so.rs (details in the D-NEW body).
* Run 2 (counts on keke, Zen 1, fast products, 21 runs, paths of one length; logs ~/logs/layout-ldso-order-2-*.log):

  | N libs | faults ASLR off base -> new | ASLR on median base -> new | instructions:u median base -> new |
  |---|---|---|---|
  | 0 | 81 -> 78 | 82 -> 79 | 9037737 -> 9038084 (+347) |
  | 3 | 97 -> 95 | 97 -> 95 | 9317884 -> 9317882 |
  | 12 | 133 -> 131 | 134 -> 131 | 10120229 -> 10120228 |

  Page split (N=0): ld.so text faults 5 -> 2; data and anonymous pages unchanged. Determinism: Zen 1 and Skylake-SP
  builds give byte-identical lists, signatures, bins, ld.so and libc.so.6. A 46 KiB `#[cold]` pad still adds +4
  faults on base and branch (window phase, +1 RW page, +2 anonymous): handed to LAYOUT-LDSO-WINDOW.
* New test `init_dynamic_startup_only_program_stays_within_its_page_fault_bound` (hot_text_products).
* Run 3, first session: the +347 cause search (bounded, per ruling): the base and new interpreters have identical
  section addresses except `.text` (0x46b80 -> 0x46a9e, 226 bytes shorter grouped); libc.so.6 identical. So only
  code placement differs; the cause was not found, and the record states it as the measured cost. The plants file
  no longer runs `cargo xtask so` inside the plant commands, and its msg regexes use grep -E syntax. Its gate went
  red on `hottext::tests::trace_writes_the_names_at_one_address_in_byte_order`: the test pinned the tracer's
  output loop `for n in names.get(addr, [])`, which this story renamed to `out_names` (the rebased copy for ld.so).
  The session then ended on a login expiry.
* Run 3, second session: that test now finds the rebased copy (`OFFSETS = [(d, dict(d)) for d in (names, ...`),
  the choice `out_names = OFFSETS[0][1] if ldso else names` and the `out_names` loop, and checks nm < sort < copy <
  choice < write, so both modes still write each address's names in byte order (a530f933b).
  Jobs on mei, head a530f933b: `layout-ldso-order-3-lint1` (clippy -D warnings, static, records check) green;
  `layout-ldso-order-3-plants3` (handoff/LAYOUT-LDSO-ORDER-2-plants.txt, /mnt/zram/claude/layout-ldso-order-3/plants-3,
  --timeout 3600): 2 CAUGHT, 0 WARN (results: handoff/LAYOUT-LDSO-ORDER-3-plants.txt);
  `layout-ldso-order-3-gate3` (`cargo xtask affected --run --base origin/next`, which escalated itself to FULL
  because xtask changed; then `cargo xtask test`; then `NIJI_FULL_PROFILE=1` so + hot_text_products, peak 12G):
  affected 32 suites, 12123 passed, 0 failed; xtask test ok; release hot_text_products 26 passed, 1 failed:
  `init_dynamic_startup_only_program_stays_within_its_page_fault_bound` read 86 > 85 (warm-up and 5 runs all 86;
  ld.so grouped, 94 sections). The release bound (85) had been measured on the Zen 1 host only. Skylake-SP reads
  one more, as the file already records for other probes. Fix 1ccdc50f8: `DYN_TRUE_BOUND` = highest level of the two
  host classes, release 85 -> 86 (this story's own unlanded bound; fast stays 89). Failing suite rerun on
  1ccdc50f8, `layout-ldso-order-3-rerun4` (clippy, static, records check; release so + whole hot_text_products;
  fast so + whole hot_text_products; `--nocapture` for the counts): rc 0, peak 2.4G; release 27 passed (dynamic
  probe 86, bound 86), fast 27 passed (dynamic probe 87, bound 89; the ungrouped plant read 90 on this host, so the
  grouping saves 3 faults on Skylake-SP fast too). Fast on Skylake-SP (87) is under the Zen 1 level (89), inside the
  slack of 4; the bound stays at the higher class.
* Layout bins (§9): no libc source changes on this branch; run 3 measured libc.so.6 byte-identical to the base, so
  libc's start-up bins do not change.

## Next step
Land (train). Follow-up: LAYOUT-LDSO-WINDOW (window phase independent of image size, e.g. keep the interpreter's
mapped size a multiple of 64 KiB or align its base; ld.so data/bss pages; the insensitivity proof with the pad).

Do not redo: base/new/pad builds and counts (run 2), the +347 search, the plants (plants3 on a530f933b).
