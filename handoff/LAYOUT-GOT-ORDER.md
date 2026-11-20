# LAYOUT-GOT-ORDER handoff (D-X.140): static programs have no `.got`

**State:** branch `task/layout-got-order` (base origin/next e5447b54f), role mei:~/w/s1 (Skylake-SP). Decision
**D-X.140** (`records/decisions/D-X/D-X.140.toml`, amends D-X.138). Plants: `handoff/LAYOUT-GOT-ORDER-plants.txt`.
Coordinator ruling #6935 (premise challenge accepted): mark the binop GOT relocations instead of reordering `.got`.
Status: COMPLETE pending the hand-back checks (see "Jobs"); nothing left to code.

## What was found (link map of the static probe, base e5447b54f fast)

* `cc -static` uses GNU ld's default script: `.got` always follows `.data.rel.ro`; libniji cannot reorder it.
* `.got` = 0xc8 bytes (25 slots) at 0x86b110, 272 bytes into its page; start-up read 5 slots (refresh_words, ...).
* All 25 slots come from 160 plain `R_X86_64_GOTPCREL` on `add`/`cmp` operands (LLVM 23 emits plain GOTPCREL for
  binops even with `-Cllvm-args=-x86-relax-relocations=true`; checked with a two-function crate). GNU ld relaxes only
  `*GOTPCRELX`. A REX_GOTPCRELX binop to a target above 2 GiB makes ld 2.42 refuse the link (probe), not miscompile.

## What is in force

* `xtask/src/gotrelax.rs` (called in `so.rs::finish_archive` on the published archive after the weakening pass):
  byte-checked REX.W adc/add/and/cmp/or/sbb/sub/xor/test RIP-relative -> `R_X86_64_REX_GOTPCRELX`; build fails if
  0 marked. Result: 160 marked in 1 member, **0 plain GOTPCREL left**.
* `layout/contracts.tsv`: `.got:1` data bin removed; rw-budget fast 7 -> 6, release 8 -> 7 (both verified).
* `tools/hottext/startup_data.py`: a `cmp` with an absolute address declares no object (the relaxed `fp == stdin`
  compare had put `_IO_2_1_stdin_` alone on page 0x88b000: .data1+.bss 4 pages > 3).
* `hot_text_products::init_static_probe_has_no_got_at_any_relro_size` (relro-tail pads 0/64/512/4000).

## Measured (mei, Skylake-SP)

| | base e5447b54f | this branch (code 0f3963634) |
|---|---|---|
| static probe `.got` (fast) | 0xc8 at 0x86b110 | none |
| written start-up pages fast / release | 7 / 8 | 6 / 7 (6 of 6, 7 of 7 budgeted) |
| `.data.rel.ro` bin fast / release | 6728 / 4153 of 2 pages | same |
| `.data1+.bss` bin | 11784 of 3 pages | same |
| step-2 test relro-tail 0/64/512/4000 | - | no `.got` at any pad |

Paired execcount (ASLR off, median of 101, fast products; job `layout-got-order-counts1`), instructions / faults:

| program | base | new |
|---|---|---|
| dynamic empty main (pie0) | 832958 / 64 | 832917 / 64 |
| dynamic 3 libs (pie3) | 1095655 / 80 | 1095563 / 80 |
| dynamic clock_gettime (time) | 868038 / 68 | 867997 / 68 |
| static empty main (st0) | 2095 / 8 | 2094 / **7** |
| static clock_gettime (sttime) | 7508 / 12 | 7508 / 12 |

`libc.so.6` differs only outside its relocations (identical `readelf -rW` md5). A static PIE keeps its `.got`
(0xe0 both; a PIC link cannot turn a binop into an immediate), unchanged.

## Brief step 4 (not done; sized)

Data bins naming objects new or grown against next: not small. `layout-bins.tsv` (the base) is the text planner's
plan, written before the link, so data objects need a new link-time artifact (e.g. `layout-data.tsv` from
`layout.rs`), a parser and a `fmt_grown` for `DataBin` in `binbudget.rs`, two tests, plants, and the train exporting
a second base variable: about 120 lines plus an ops change. Recommended as its own story.

## Jobs (mei)

`-counts1` (counts, base vs new), `-plants2` (6 plants: 5 CAUGHT, `got-after-relro-again` WARN-NO-MSG: the build's
layout gate went red first, `data bin .got (written) ... no budget` and 7 > rw-budget-fast 6), `-plants3` (that plant
again with `msg:` naming the gate), `-startup1` (release so + release hot_text_products + `cargo xtask test`: green),
`-tfu2` (step 5: scratch merge of this branch + next 3f2a84520 + origin/task/thread-forced-unwind-2 260e4d491, not
pushed: `cargo xtask so`), `-lint3` (clippy -D warnings, `cargo xtask static`, `records check` on d2b4a1f59: green).
Superseded: `-so1`/`-so2`/`-so3` (earlier heads), `-plants1` (plants file refused at parse: ` | ` inside a `lit:`
pattern), `-lint1` (unused `mut` in a gotrelax test helper), `-lint2` (D-X.140 body had paragraph breaks). The
counts, plants and startup jobs ran on code that differs from the final head only by that test-helper `mut`.

Step 5 result (`-tfu2`, merged tree cebfb1a6 content, 12:45Z): `cargo xtask so` green; STARTUP-DATA-WINDOWS 6 of 6
written pages; `.data.rel.ro` 0x860000-0x86b060 (0xb0 shorter than this branch), no `.got` bin.

Precheck (`-precheck1`) was red on jobs: the plants ran before the test-helper `mut` fix, and `sh ~/work/...` jobs
name no `w/s1`, so jobrun kept no tested-content record. Final-head reruns, started at hand-back (PENDING):
`-plants4` (full plants file), `-startup2` (release so + hot_text_products + `cargo xtask test`), `-lint5`.

## Exact next step

Wait for `layout-got-order-plants4`, `-startup2` and `-lint5` (`~/bin/jobwait` on mei), then run `cargo xtask precheck
--jobs <those> --head <head>` on s1. Follow-up story: the step-4 data-bin diff (above).
