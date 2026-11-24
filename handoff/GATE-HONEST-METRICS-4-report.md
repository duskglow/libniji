# GATE-HONEST-METRICS-4 report (leg 4: train 975 red, owner rulings #8323 / #8322)

Branch `task/gate-honest-metrics-4`, role mei:~/w/s1 (Skylake-SP host). Code head d8f77f1a3.

## Commits

- d20f3ccc8 Trace the start-up layout in the locale-archive form only (#8323): removes the per-locale-directory
  trace from regen.sh, runner.sh and canon_host.sh, and trace_gdb.py's prefix redirection. The cause test
  `init_data_clusters_hold_at_every_text_phase` traces true and fork with the host's files, and locale and tool
  with the build's data files in their archive form. D-X.165 (not yet landed) is corrected to match: #8134 held,
  window budgets still gated (#8322).
- 0bf8eb08f Merge origin/next 9139a21d9 (clean; the brief asks for the check on the merged head; #8197 allowed
  this chain's merge of next).
- d8f77f1a3 Plants file. Later commits: handoff and report only.

## Train 975 cause (measured, release profile)

- Branch alone at e00b91c27 (base 7a0ba4cb2, both forms traced), job gate-honest-metrics-4-so-rel1: release tool
  2/2. Tool's sections were 115,133 B (lower bound 2 windows). Bin 2 (locale, strings, tool) was 64,641 of
  65,536 B (headroom 895): `{locale,tool}` 51,988 B, `{tool}` 4,557 B, `{strings}` 6,127 B. The per-directory
  trace added 5 functions to both modes (regen: "5 functions the archive trace did not run"), including
  `find_named<Files, Glue>` 6,189 B.
- Train 975's merged head (log lines 777-815): bin 2 was 61,355 B, the `{tool}` group no longer fit, and the
  planner put it in bin 1. That gave tool 3 windows.
- Nothing could move to make room. Bin 0 holds only 435 B of members that tool does not run. Bin 1 has no member
  whose signature is covered by another bin. The strings group (budget 0) must stay in a bin its warm-up runs in
  (bins 0 and 2).
- Option 1 of the first coordinator note (fit tool in 2 windows) had no zero-cost assignment. Note 2's ruling
  #8323 replaced it.

## Leg 4 results (merged head, archive form only)

- Release profile, job gate-honest-metrics-4-rel1 (`NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1
  cargo test -p niji --test hot_text_products`): WINDOW-BIN-BYTES ok; windows per mode (release, count/budget):
  float 1/1, fork 2/2, locale 2/2, localtime 2/2, localtime_mktime 2/2, mktime 0/0, rebind 0/0, strings 0/0,
  tool 2/2, true 1/1. hot_text_products: 36 passed.
- Release bins: bin 0 63,560 of 64,512 B; bin 1 64,828; bin 2 65,281 of 65,536 (headroom 255); bin 3 51,294.
- Direct gate, job gate-honest-metrics-4-gate1 (`cargo xtask affected --run --base origin/next`): 7 suites, 700
  passed, 0 failed.
- Plants, job gate-honest-metrics-4-plants1: 2 of 2 CAUGHT, each by its message. Dropping the setlocale keys is
  named by locale in the archive form. Dropping the textdomain keys is named by tool in the archive form.
- `cargo xtask test`, job gate-honest-metrics-4-xtest1: 29 suites, 12,700 passed, 0 failed; conformance 149 /
  0. Its fast-profile products step printed the non-blocking WINDOW-COUNT REPORT (D-X.145; windows-block is
  release): locale 3/2, tool 3/2. Next's fast count was not measured.
- Lint: gate-honest-metrics-4-lint0 (step 0, inherited head e00b91c27) green. gate-honest-metrics-4-lint1
  (merged head 0bf8eb08f): foreground-equivalent `cargo clippy --workspace --all-targets --all-features -- -D
  warnings`, `cargo xtask clippy` and `cargo xtask static`, green.

## INFORMATIONAL

- Release bin 2 has 255 B of headroom on next 9139a21d9 plus this branch. No member of the plan has a section
  name past the old 256-byte cut, so leg 3's whole-name listing adds none of those bytes. The rest of the branch
  changes tests and tools, not libc.a. A few hundred bytes of locale, tool or strings growth, such as the
  locale change in train 975, will make tool's own 4.5 KB group spill again (3 windows). This is next's state.
  Ruling #8322 (window budgets report-only once the fault-ratio gate blocks) is the route the owner chose.
- #8322 is not applied here. This branch's fault ratios report unless `NIJI_FAULT_BOUNDS=enforce`, so it lands
  no blocking fault-ratio gate, and WINDOW-BIN-BYTES stays as it is.
- The per-directory trace (legs 2 and 3) is in this branch's history before d20f3ccc8, ready for when #8323's
  measurement brings #8134 back.

## Refusals

None.
