# PLANTS-PRODUCTS-ONCE (handoff, end of run 1, leg 3)

State: branch `task/plants-products-once` (from origin/next 45f916b8d). Role mei:~/w/s1. Code: `xtask/src/plants.rs`
(Need, needs, assign, simple_commands, RUNS_NO_PRODUCTS, cmd_reads_products, may_change_products, run_copy),
`xtask/src/affected.rs` (`unit_tests_may_read_products`, `product_unit_tests_at`, `filter_may_select`), `docs/plants.md`
("Which copies build the products"), decision record D-X.177 (reserved), plants file
`handoff/PLANTS-PRODUCTS-ONCE-plants.txt`. Report json: `handoff/reports/PLANTS-PRODUCTS-ONCE.json`.

## Decisions in force

D-X.177 (this story): a copy builds the products only when one of its plants reads them; reading is decided from what
the command runs (when in doubt: reads); the mutant rebuild keeps the tool's old rule (`plants_use_products` or a
traced layout) and `may_change_products`; readers are concentrated in as few building copies as possible.
Related: D-X.176 / D-X.142 (gate steps; the unit rule this reuses).

## Step 1 measurement (plants logs 11-24..26 on mei, keke, shiki)

| | value |
|---|---|
| plants jobs | 155 (keke 119, mei 24, shiki 12); wall sum 80.6 h |
| product builds on each job's critical path | 60.5 h (75 %) |
| jobs where no plant read the products | 32 (21 %): 6.8 h of their 9.5 h wall |
| mutant builds / restores | 775 (40.7 h) / 19 (0.8 h) |
| median wall / median build path | keke 30.7 / 23.2 min; mei 14.5 / 11.3; shiki 24.6 / 16.9 |

## Step 4 proof (mei, Zen 1, 48 CPUs; --copies 2; each before/after pair started together)

"before" = origin/next's tool in a copy of the base tree (`/mnt/zram/claude/plants-products-once/base`); "after" = ~/w/s1.
Verdicts compared per plant with `~/work/plants-products-once/verdicts.sh` on mei.

| file | before wall (CPU) | after wall (CPU) | products after | verdicts |
|---|---|---|---|---|
| AFFECTED-LAYOUT-ROWS (xtask only, 7) | alr-before 624.5 s (2786 s) | alr-after 204.4 s (895 s) | 0 of 7 read; built in 0 of 2 copies | identical, 7 CAUGHT |
| HEADERS-STAGE2-GAPS-D (header, 20) | hdr-before 1871.7 s (5880 s) | hdr-after 416.0 s (1686 s) | 0 of 20 read; 0 of 2 copies | identical, 20 CAUGHT |
| LIBM-PARTIAL-WRITE-CHECK (libm, 6) | libm-before 1012.6 s (3641 s) | libm-after 1050.5 s (2779 s) | 6 of 6 read, 1 mutant; 1 of 1 copies | identical, 6 CAUGHT |

Labels are `plants-products-once-<x>`. The leg-2 choice `libm-special-fast-tier-6` names the old `liella-*` crates
(its baselines would fail), so LIBM-PARTIAL-WRITE-CHECK (one `cargo xtask so` plant on `crates/math`, five xtask plants)
replaced it. libm row: +3.7 % wall, -24 % CPU. All six read the products by the unit rule
(`partial_write::static_checks_only_a_product_of_this_tree` joins `target/niji` under a temp root: the text rule errs
toward building), so all six went to one building copy and the second copy idled.

Rule plant (blocking): `ppo-mutant-shared` (a mutant plant handed products another plant's mutant built) and
`ppo-src-reuses` (a `src/` plant judged unable to change the products): CAUGHT in job `plants-products-once-plants`
(12 of 12 CAUGHT, no WARN, 315 s; the run itself built no products).

## Gate, lint

Direct tier: `plants-products-once-gate` rc 0, 7 suites, 718 passed, 0 failed, peak 3.6G. Workspace clippy
(`--all-features -D warnings`) and `cargo xtask static` clean on mei:~/w/s1. Unit tests: job plants-products-once-unit3
(158 passed, `plants:: affected::`).

## Open / next

- Cost: when every plant reads the products, building once and running all readers in one copy trades wall for
  CPU (libm row). Follow-up idea: build once, copy the built `target/niji` products into the other copies for plants
  that only read them (keeps the all-readers wall, saves the second build). Not started.
- Precision: the unit rule reads text, so tests that join `target/niji` under a temp root count as readers; a finer
  rule (only a join on the tree root) would let such plants skip the build. Not started.
- Things not to redo: measurement, design, unit tests, the plants file, the three proof pairs.
