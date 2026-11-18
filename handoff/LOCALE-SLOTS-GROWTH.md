# LOCALE-SLOTS-GROWTH (audit HL-10): handoff

**State:** branch `task/locale-slots-growth-5` (run 5; stacks on `-4`, `-3`, `-2`, `-1`). Merged origin/next
`923df67e9` (`9cfdcb888`, with TRACE-TIME-PIN), then `63f550a8b` (charmap lookup through a per-call `Files`, plus the
run-5 plants file), then this note. INCOMPLETE: the product build (`cargo xtask so`) fails one layout contract,
STARTUP-DATA-WINDOWS: 8 written start-up pages, budget 7 (base: 7). Every trace criterion from the step-back holds
(below). RULING NEEDED: whether to fix the cause in layout first (recommended) or count it against this story.
Report: `handoff/LOCALE-SLOTS-GROWTH-report.md`, `handoff/reports/LOCALE-SLOTS-GROWTH-5.json`.

## Decisions in force
`records/pending/LOCALE-SLOTS-GROWTH-1.toml` (D-NEW, series D-7; amends D-7.7, D-7.19, D-X.13): no cap on locales,
names or files. There is a static table (1024 entries) plus spill chunks of `64 << k` that never move. Past 64
entries, lookups use chunked hash indexes that are never rehashed. ENOMEM is reported from `newlocale` and
`setlocale`. Paths that start-up never runs are cold and out of line. D-X.101 (start-up data pages) and D-X.103
(window bins) are not touched.

## Run 5: measured on keke n1 (Zen 1, fast products), branch `63f550a8b` vs base `923df67e9` (saved in `~/work/LOCALE-SLOTS-GROWTH-5/{base,br}/`)
- clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static`: green
  (`locale-slots-growth-5-lint`). The same job's `cargo xtask so` is rc 1, only on STARTUP-DATA-WINDOWS.
- The step-back criteria, read from the products that the failed `so` had already written
  (`check.sh`, output `out/check.txt`):
  - `hot-text.signatures` differs from the base only in locale-tier rows: `Files::file_or_error` became
    `Source::file`, `intern_locked` closure renamed, `by_handle` and `query_cached` with `-`. One exception: a new
    row `TryBox<time_compat::Entry>::new` (mktime modes). It is a newly outlined function, not a changed signature.
  - The strings group (strlen, strncmp, niji_machine_strncmp, niji_machine_compare) is in bin 2, as in the base.
    Locale-tier growth in bin 2: intern_name_locked +293, intern_locked +139, each_block(publish_global) +133,
    setlocale_locked +81, setlocale_env +58, replace +42, refresh_words (bin 0) +22. Bin 2's total is 63,061 B,
    against 63,098 B in the base.
  - pf.py (run 4's copy): tool mode 44 faults on both branch and base (bound 44); strings mode 0 on both.
- **Cause of the red** (`rwpages.sh`, `relro.py`, `got.py`, `locs.py`, `fnptrs.py` in the work dir):
  - In the page_faults probe link, `.data.rel.ro` is 45,328 B in the base and 44,928 B on the branch (-400).
    `.got` (25 -> 26 slots; this story's `INDEX` adds one) follows it, so it starts at 0x86b110 in the base
    (one page) but at 0x86af80 on the branch, and straddles 0x86b000.
  - Start-up reads six `.got` slots: `locale::refresh_words` (C_CLASS, C_TOUPPER, global_locale::SLOTS) and
    `stdio::parts_first_use` (the three std streams). They are `add`/`cmp` instructions with a GOT memory operand,
    which carry plain R_X86_64_GOTPCREL (libc.a holds 160), and ld cannot relax that.
  - So the slots now sit on two pages, 0x86a000 and 0x86b000. In the base they sit on one.
  - The -400 B comes from LTO reshuffling anonymous relro in libc's single codegen unit. It is not one object.
    Panic locations account for -96 B (all in net_compat.rs, which this story does not touch); code pointers in
    relro for -40 B. The remaining -264 B is unattributed anonymous data.
  - D-X.101 pins the start-up relro cluster, `.data1`, RESERVE_MEM and SLOTS to fixed pages, but not `.got`.
    STARTUP-DATA-PAGE-ALIGN's handoff already names "place .got beside the start-up relro cluster" as the open
    step, and TB-FOREIGN-WORDS-LOW-11 reported the same `.got` page in next.
  - The measured tool fault count does not move (44 = 44); file-backed fault-around probably maps both pages at
    once (not verified). The contract's static page count is the only thing that fails.

## Open
- RULING NEEDED: the `.got` page split. Recommendation: a layout story that makes the start-up `.got` reads take a
  fixed page whatever precedes them (the D-X.101 follow-up, or start-up code without GOT-operand forms); then rerun
  this story's gate, start-up checks and plants with no product change. Padding relro or moving sections to dodge
  the boundary is not a fix; I did not do it.
- (c) Archive-form locales share one file entry: from reading the code, not measured.

## Next step (exact)
After the ruling or the layout fix: sync, run `cargo xtask so` (it must pass STARTUP-DATA-WINDOWS), then the direct
gate, the start-up checks (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products`) and the
plants (`handoff/LOCALE-SLOTS-GROWTH-5-plants.txt`, 21 plants) on the head, all under jobrun. Then rerun `check.sh`
against the new merge base.

## Do not redo
- Do not shrink locale code further (step-back decision); the trace criteria hold.
- No fixed larger cap, no preallocation, no rehashing index; the static table stays.
- Runs 1-4 jobs are superseded. The run-2 counts stay valid unless lookup code changes.
