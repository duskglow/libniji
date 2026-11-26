# PLANTS-PRODUCTS-ONCE (handoff, end of leg 2 of run 1)

State: branch `task/plants-products-once` (from origin/next 45f916b8d). Role mei:~/w/s1. Code: `xtask/src/plants.rs`
(Need, needs, assign, simple_commands, RUNS_NO_PRODUCTS, cmd_reads_products, may_change_products, run_copy),
`xtask/src/affected.rs` (`unit_tests_may_read_products`, `product_unit_tests_at`, `filter_may_select`), `docs/plants.md`,
plants file `handoff/PLANTS-PRODUCTS-ONCE-plants.txt`. Compiles; `cargo clippy -p xtask --all-targets -D warnings`
clean; `cargo test -p xtask --bin xtask -- plants:: affected::` 158 passed (job plants-products-once-unit3).

## Done (do not redo)

- Leg 1: measurement (table below), design.
- Leg 2: compiled, fixed (doc placement in affected.rs; a plant that reads nothing no longer forces a restore build;
  `distribute` replaced by `assign`); unit tests: assign (in-turn dealing, fewest building copies), needs,
  may_change_products, simple_commands, header chain, unit-test rule (plants.rs), unit_tests_may_read_products
  (affected.rs); the two-copy toy run asserts the copy without readers builds nothing. Found and fixed a soundness gap:
  libtest filters match the whole test path, the unit rule compared bare names (`plants::` never matched); now
  `filter_may_select` over-approximates over the file's module names (commit 70f1ca22f).
- Plants: mei job `plants-products-once-plants` on e32aa39ad: 12 of 12 CAUGHT, no WARN (incl. ppo-mutant-shared:
  a mutant plant reusing products another plant's mutant built; ppo-src-reuses: may_change_products false for src).
  The run itself built no products (0 of 12 read them), 315 s wall.

## Step 1 measurement (leg 1; plants logs 11-24..26 on mei, keke, shiki)

| | value |
|---|---|
| plants jobs | 155 (keke 119, mei 24, shiki 12); wall sum 80.6 h |
| product builds on each job's critical path | 60.5 h (75 %) |
| jobs where no plant read the products | 32 (21 %): 6.8 h of their 9.5 h wall |
| mutant builds / restores | 775 (40.7 h) / 19 (0.8 h) |
| median wall / median build path | keke 30.7 / 23.2 min; mei 14.5 / 11.3; shiki 24.6 / 16.9 |

## Step 4 proof (in progress; mei, --copies 2, each pair before/after started together)

"before" = origin/next's tool in a copy of the base tree: `/mnt/zram/claude/plants-products-once/base` (xtask prebuilt),
plants file passed by path from ~/w/s1/handoff. "after" = ~/w/s1. Dirs `/mnt/zram/claude/plants-products-once/<x>-1`.

| file | before job (wall) | after job (wall) | verdicts same? |
|---|---|---|---|
| AFFECTED-LAYOUT-ROWS-plants.txt (xtask-only) | plants-products-once-alr-before 624.5 s | plants-products-once-alr-after 204.4 s, 0 of 2 copies built | identical: 7 CAUGHT each |
| HEADERS-STAGE2-GAPS-D-plants.txt (header) | plants-products-once-hdr-before (started 06:13Z, RUNNING at refresh) | plants-products-once-hdr-after (started 06:13Z, RUNNING; "0 of 20 plants read them, built in 0 of 2 copies") | |
| libm-special-fast-tier-6-plants.txt (libm) | plants-products-once-libm-before | plants-products-once-libm-after | |

Compare verdicts on mei: `sh ~/work/plants-products-once/verdicts.sh <before-label> <after-label>` (installed; prints
"verdicts: identical" or the diff, and the summary/products lines). Wall: column 6 of the jobwait / jobs.tsv line.

## Exact next step

1. `~/bin/jobwait plants-products-once-hdr-before plants-products-once-hdr-after` on mei (one call, 600 s, repeat on
   exit 3), then verdicts.sh on them; then start the libm pair; start the next wave only when
   the previous pair ended (header, then libm), same command shape as alr (see `~/logs/plants-products-once-alr-*.log`
   first line or jobs.tsv): before `cd /mnt/zram/claude/plants-products-once/base && cargo xtask plants
   ~/w/s1/handoff/<file> --dir /mnt/zram/claude/plants-products-once/<x>-before-1 --copies 2 --timeout 600`, after the
   same from ~/w/s1 with `<x>-after-1`. If a file is REFUSED (plant no longer applies), pick another header/libm file.
2. Fill the table, diff verdicts. Then `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
   `cargo xtask static` on mei:~/w/s1 (foreground), a direct-tier gate (`cargo xtask affected --run` under jobrun gate),
   precheck, report json (`handoff/reports/PLANTS-PRODUCTS-ONCE.json`), handback-check, hand back.
3. D-row: none reserved yet; reserve with `ops reserve --story PLANTS-PRODUCTS-ONCE D-X` if a decision row is wanted
   (the rule is local tooling; a one-line row is enough).

Open: an assignment that puts all read-only plants in one copy can leave the other copy idle (load balance traded
for one fewer product build); note in the report. Slip to report: one `sed -i` on a scratch file (not the repo) in leg 1.
