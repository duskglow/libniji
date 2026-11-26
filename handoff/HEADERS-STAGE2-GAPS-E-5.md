# HEADERS-STAGE2-GAPS-E-5 (merge-only)

State: branch task/headers-stage2-gaps-e-5; origin/next merged into E-4's work, no new work.

Merge: 13 conflicted files. headers/reserved.tsv, spec/abi/visibility.tsv, s07.py, s10.py, s12.py, header_probe.py keep
both sides (next's `__WALL`/`__RES`/array-parameter/locale-struct/ELF names beside E's `__u_*`/`__fd_mask`/ftw/ns names;
`stage2e_names` made a complete function beside next's `stage2_gaps_b_names`; both register calls kept). The seven
generated libc-include headers were taken from the merge and regenerated with `cargo xtask headers-gen`.

Results on the final head: clippy and `cargo xtask static` clean; direct-tier gate green; plants (handoff/HEADERS-STAGE2-GAPS-E-plants.txt):
12 CAUGHT, 0 SURVIVED, 0 WARN.

Not to redo: the table merges. Next step: land.
