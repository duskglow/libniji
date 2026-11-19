# LAYOUT-DYN-PHASE: the dynamic start-up fault count must not depend on unrelated image sizes

State: branch `task/layout-dyn-phase` (cut from origin/next 36df714e7), HEAD c43644b03 (code 6034b2ea9 + bound,
record, plants), pushed. Role mei:~/w/s1 (Skylake-SP, Linux 6.8). Record: **D-X.133** (reserved, written:
`records/decisions/D-X/D-X.133.toml`). Read also D-X.123, D-X.125. Leg 1 measured/found causes, leg 2 implemented
and measured flat; leg 3 = finish (verify jobs, lint, precheck, report, hand back).

## What changed (leg 2)
1. `crates/ld/src/map.rs`: `Segment.mem_end` (exact p_vaddr + p_memsz); `partial` only when mem_end > file_end. A
   segment without zero fill maps its last page from the file (no view read, no anonymous copy, no store).
   `crates/ld/tests/map.rs` `check_model`: such a segment's pages are all `Backing::File` with nothing stored.
2. libc.so.6: `csys/src/machine/entry.S` `-DNIJI_LIBC_SO` branch = only `.rodata1` "aR" (SHF_GNU_RETAIN: without
   it `--gc-sections` dropped the empty anchor; my check caught that on the first build) `.balign NIJI_RODATA1_ALIGN`;
   `xtask/src/so.rs` assembles it (`target/product/libc-so-anchor.o`) and passes it first in `link_libc`;
   `check_rodata1_anchor` / `rodata1_anchor_errors` (+ unit test `libc_so_rodata1_group_is_anchored_and_holds_the_build_tag`).
   `xtask/src/startup_ro.rs` `LOADER_READ = ["__niji_build_tag"]` joins `ro_keys`.
3. ld.so: `ld-phase.ld` INSERT BEFORE .rodata (`ldso_phase_script`, `LDSO_RO_PHASE` 0x10000): `.rodata` starts at
   ADDR(.text)'s 64 KiB phase; `check_interp_ro_phase` / `interp_ro_phase_errors` after the span check.
4. `crates/core/tests/hot_text_products.rs` DYN_TRUE_BOUND [89, 86] -> [80, 76].

## Sensitivity tables (Skylake-SP; method in D-X.133; leg-1 scripts mei ~/work/LAYOUT-DYN-PHASE/)
| | k = 0..16 libc padded | ld.so padded |
|---|---|---|
| fast, next 36df714e7 | 86, 87 (1-5), 86 (6-9), 85 (10), 86 (11-16) | 86, 87 at k = 6, 7 |
| release, next | 85, 84 (1-7), 83 (8-10), 84 (11), 85 (12-16) | 85 all |
| fast, this branch (job layout-dyn-phase-l2b) | 80 all | 80 all |
| release, this branch (l2b) | 76 all | 76 all |
Every run identical (warm-up + 5). l2b also: clippy ok (on 6034b2ea9), xtask unit test ok, `niji-ld --test map` ok
both profiles, hot_text_products both profiles: only the dynamic test failed, on the slack (80 < 89-4, 76 < 86-4),
which the new bound fixes. Zen 1 host class NOT measured (no role there): RULING NEEDED / coordinator to run
hot_text_products on a Zen 1 role before landing (bound = Skylake-SP level, no margin).
SUPERSEDED: layout-dyn-phase-l2 (build refused by the new anchor check: gc dropped the anchor; fixed with "aR").

## Running (started at the end of leg 2 on c43644b03)
* `layout-dyn-phase-gate` (mei; `~/work/LAYOUT-DYN-PHASE/l2gate.sh` under `~/bin/gate 16G`): records check,
  `cargo test -p niji-ld`, `-p xtask`, fast `cargo xtask so` + hot_text_products, `cargo xtask test`, release so +
  hot_text_products; prints `== step` / `FAILED: step` / `gate rc=N`. (`cargo xtask affected` would escalate to
  FULL: xtask/ and csys/ changed, so no affected run; hand-back says `FULL: the train's (affected escalated: xtask/,
  csys/)`.)
* `layout-dyn-phase-plants` (mei, `cargo xtask plants handoff/LAYOUT-DYN-PHASE-plants.txt --dir
  /mnt/zram/claude/LAYOUT-DYN-PHASE/plants-1`): 4 plants (tail page, anchor, tag, ld phase). List any WARN lines.

## Exact next steps (leg 3)
1. On mei: `~/bin/jobwait layout-dyn-phase-gate layout-dyn-phase-plants` (grep `FAILED|gate rc` and the plants
   summary/verdicts, not the whole log). Fix anything red, rerun only that.
2. After the gate (same role, no build beside it): foreground `cargo clippy --workspace --all-targets
   --all-features -- -D warnings` and `cargo xtask static` on ~/w/s1 (static has NOT been run yet).
3. `cargo xtask precheck --jobs layout-dyn-phase-gate,layout-dyn-phase-plants --head <sha>` on the synced head.
4. Report: handoff/reports/LAYOUT-DYN-PHASE.json (impact: faults fast 86->80 / release 85->76 at k=0, flat over
   k, measured; cost: <= 60 KiB zero file bytes + address space per library); rewrite this note as the report;
   `ops handback-check LAYOUT-DYN-PHASE --head <sha> --jobs "<JOBS>"`; hand back (JOBS: gate, plants; l2b as
   evidence of the series).

## Do not redo
Both series (before/after), the record, the bound, the plants file. Not chosen (in D-X.133): a k-pad gate test
(34 relinks + probes per profile) — the three `cargo xtask so` checks catch each lever's removal at no test time.
IMPROVEMENT (for the report): the loader reads each library's headers through a whole-file view and again through
the mapped image (about 4 faults per library that could be shared): a follow-up story.
