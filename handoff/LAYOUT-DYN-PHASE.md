# LAYOUT-DYN-PHASE: the dynamic start-up fault count must not depend on unrelated image sizes

State: branch `task/layout-dyn-phase` (cut from origin/next 36df714e7), tested code head 9ceeda03e (the report commit
on top changes only handoff/), pushed. Role mei:~/w/s1 (Skylake-SP, Linux 6.8). Decision in force: **D-X.133**
(`records/decisions/D-X/D-X.133.toml`: method, tables, causes, costs, what is not covered); read also D-X.123,
D-X.125, D-X.62. Verdict: see "Proof" (all named jobs on 9ceeda03e's content).

## What changed
1. Loader (`crates/ld/src/map.rs`): `Segment.mem_end` (p_vaddr + p_memsz); a segment is `partial` only when it has
   zero fill (p_memsz > p_filesz). Without zero fill its last page is mapped from the file: no read through the
   whole-file view, no anonymous copy, no store. `crates/ld/tests/map.rs` `check_model` pins that.
2. libc.so.6: `csys/src/machine/entry.S` `-DNIJI_LIBC_SO` = an empty, retained (`"aR"`) `.rodata1` aligned to
   `NIJI_RODATA1_ALIGN`; `xtask/src/so.rs` assembles it (`libc-so-anchor.o`) as the first input of both libc links
   and `check_rodata1_anchor` refuses a library whose `.rodata1` is off its alignment or lacks the build tag;
   `startup_ro::LOADER_READ = ["__niji_build_tag"]` joins the group.
3. ld.so: `ld-phase.ld` (INSERT BEFORE .rodata) starts `.rodata` at the text's 64 KiB phase;
   `check_interp_ro_phase` refuses an interpreter whose read-only data is off it.
4. `crates/core/tests/hot_text_products.rs`: `DYN_TRUE_BOUND` [89, 86] -> [80, 76] (ALWAYS FORWARD).

## Sensitivity (Skylake-SP; dynamic probe of hot_text_products; warm-up + 5 runs, every run identical)
| | libc.so.6 padded by k = 0..16 pages | ld.so padded |
|---|---|---|
| fast, next 36df714e7 | 86, 87 (k 1-5), 86 (6-9), 85 (10), 86 (11-16) | 86, but 87 at k = 6, 7 |
| release, next | 85, 84 (1-7), 83 (8-10), 84 (11), 85 (12-16) | 85 at every k |
| fast, this branch (leg-2 job layout-dyn-phase-l2b, code = 9ceeda03e's) | 80 at every k | 80 at every k |
| release, this branch (l2b) | 76 at every k | 76 at every k |
Zen 1 host class: NOT measured (no role there; see RULING NEEDED). The three red stories' heads were not re-measured
(their counts were 91 > 89, 91 > 89, 87 > 86 on the old bound; the coordinator reruns them on top of this).

## Layout and size (leg 3, merge base built in RAM: job layout-dyn-phase-base-bins, evidence only)
* `target/product/fast/layout-bins.tsv`: **identical** to the merge base's (developer-flow section 9): no start-up bin moved.
* Fast products, base -> branch: libc.so.6 file 6030312 -> 6063080 B (+32768), mapped span 0x518418 -> 0x520418
  (+0x8000, the `.rodata1` anchor's alignment gap); ld.so file 420520 -> 420448 B (-72), span 0x80000 both.

## Proof (jobs on mei, content of 9ceeda03e)
* `layout-dyn-phase-startup-2` (kind test; `~/work/LAYOUT-DYN-PHASE/l3gate.sh` under `~/bin/gate 16G`): records check,
  fast `cargo xtask so`, `cargo test -p niji-ld`, `-p xtask`, fast hot_text_products (28 passed), `cargo xtask test`,
  release `so` + hot_text_products (28 passed): rc 0, every step passed (`gate rc=0`).
* `layout-dyn-phase-plants-2` (`cargo xtask plants handoff/LAYOUT-DYN-PHASE-plants.txt`): rc 0, 4 of 4 CAUGHT
  (dyn-phase-tail by `map_model_objects_from_the_linker`; anchor, tag and ldro by the `cargo xtask so` checks, the
  anchor plant built traced), 0 SURVIVED, no WARN lines.
* Foreground on the role after the jobs: `cargo clippy --workspace --all-targets --all-features -- -D warnings` rc 0,
  `cargo xtask static` rc 0. Precheck and `ops handback-check` on the synced final head: in the hand-back.
* FULL: the train's (`cargo xtask affected` escalates: xtask/so.rs, xtask/startup_ro.rs).

## SUPERSEDED (all mei; killed by me unless said)
* `layout-dyn-phase-gate` (rc 124): the same tests as startup-2 but kind `gate`, which precheck reds without an
  `affected: ok` verdict (the FULL-escalation recipe runs no `affected`); restarted as kind test.
* `layout-dyn-phase-tests` (rc 124): its command line named no `w/<role>`, so jobrun kept no tested-content record.
* `layout-dyn-phase-startup` (rc 124): `atexit_owner` red, "build tag differs": the killed gate's half-finished
  `cargo xtask so` had left target/product/fast mixed, and l2gate.sh ran the loader tests before its own `so`
  (l3gate.sh builds first).
* `layout-dyn-phase-plants` (rc 1): dyn-phase-anchor SURVIVED. The plants tool builds ungrouped by default; there
  the group holds only the build tag, alignment 4096, which the unanchored group met. The plant is now
  `layout:traced` (alignment 65536); the other three were CAUGHT there too.
* `layout-dyn-phase-l2` (leg 2: the anchor check refused the build; fixed with "aR"). `layout-dyn-phase-l2b` (leg 2,
  on 6034b2ea9: the series above; evidence, not named).

## RULING NEEDED
* Zen 1 not measured: `DYN_TRUE_BOUND` [80, 76] is the Skylake-SP level, no margin; the test's rule is the higher
  of the two host classes. Recommend: run fast and release hot_text_products on a Zen 1 role on this head before
  landing (the HEAP-ADDRESS-SPACE-8 rerun's gate does it); a higher Zen 1 level becomes the bound, with numbers.

## INFORMATIONAL
* precheck reds a kind-`gate` job without `affected: ok`, while the brief's FULL-escalation gate ("precheck plus
  your changed crates' tests") runs no `affected`: the recipe could name the kind (`test`).
* `~/bin/kill-label --kill` ended only the slots/jobrun processes; the job's script, cargo, rustc and the hot-text
  regen subshells kept running, reparented to init, and kept building in the role (ended by PID / kill-mine).
* Commit 6034b2ea9 (the code) carries a `WIP:` subject from the leg-2 push; c43644b03's message describes the result.

## Not done / not to redo
Both series, the record, the bound, the plants. Not chosen (D-X.133): a k-padding gate test (34 relinks and probes
per profile); the three `cargo xtask so` checks catch each lever's removal at no test time. Not covered: ASLR-on
phase of ld.so's base; the header reads through the loader's whole-file view (about 4 faults per library).

## Next step
Coordinator: Zen 1 check above, then rerun ICONV-NAME-MATCH-4, STARTUP-GOT-PAGE-7, HEAP-ADDRESS-SPACE-8 on top.
Follow-up story: let the loader read each library's headers through the mapped image instead of a second
whole-file view (about 4 start-up faults per library).
