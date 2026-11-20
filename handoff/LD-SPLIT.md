# LD-SPLIT / LD-SPLIT-2: the loader's ld.rs split into modules by concern (run 2, leg 2: done)

**State.** Branch `task/ld-split-2`, cut from origin/next `c1413a3c4` (contains DL-TRACE-INTERP-2). Code head
`a9371bfd5`; the commits after it touch handoff/ only. Role keke `~/w/n1`. Run 1's branch `task/ld-split` is
superseded (nothing from it was merged). Full report: handoff/LD-SPLIT-2-report.md (module map, commits, every
proof, the start.rs / so.rs proposal). JSON: handoff/reports/LD-SPLIT-2.json.

**Decisions in force.** Ruling #6193: regenerate on a next that contains DL-TRACE-INTERP. Done. Its literal test
("only DL-TRACE-INTERP's lines differ") cannot hold because four loader stories changed ld.rs. The proof used
instead: every item of next's ld.rs appears exactly once in the split, byte-identical (127/127).

**Proved on a9371bfd5 (keke):** step0 clippy+static; every move commit compiles; moved-only diffs per commit;
unsafe audit markers equal before and after; merge-tree demo clean; direct-tier gate `ld-split-2-gate` (9 suites,
630 passed); plants `ld-split-2-plants` (5/5 CAUGHT); `ld-split-2-hottext` (30 passed); `ld-split-2-compare-5`
(17/17 shared products have identical exported symbols; dynamic start-up instructions equal on 3 probes, within
run-to-run variation); `ld-split-2-lint` (clippy -D warnings + static). Swap root: head and base both
same=63 differ=2 in the same two ldd cases (`ld-split-2-swap-2`, `-swap-base`; both rc 1 by design, so not in
JOBS).

**Not run:** FULL `cargo xtask test` (owner 11-19: the CI train is the FULL gate).

**Do not redo:** the split, the proofs above, the base products build (keke `~/work/ld-split-2/base`, release).

**Next step:** land it. A later story may split crates/rt/src/start.rs and xtask/src/so.rs as the report
proposes, and may move `_r_debug`/`_dl_debug_state` into ld/debug.rs.

Scratch: `~/w/.scratch/LD-SPLIT-2/work/` (generator copies, moved_only.py, verify_moves.py, compare.sh, swap.sh).
