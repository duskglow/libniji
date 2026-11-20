# LAYOUT-DYN-PHASE-4: Zen 1 measurement with DL-23-2 merged (continues LAYOUT-DYN-PHASE, -2, -3)

State: branch `task/layout-dyn-phase-4` = run 3's head 2c4d5546b + merge of origin/next 05636bf0b (66b439ab8;
clean; it brings DL-23-2) + the bound commit bd52a0a39; the report commits on top change only handoff/. Role
keke:~/w/n4 (Zen 1 host, 60 vCPU). Decision in force: **D-X.133** (updated this run). Earlier reports:
`handoff/LAYOUT-DYN-PHASE.md` (run 1, design and plants), `handoff/LAYOUT-DYN-PHASE-2.md` (Skylake-SP merged level).
Run 3 left no handoff (killed at the wall-time limit inside a full `cargo xtask test`); its one result was release
76 on Zen 1 before DL-23-2.

## What this run did
1. Merged origin/next 05636bf0b (DL-23-2 landed there; the brief requires it before measuring). Clean merge; next's
   bound is still 89 / 86.
2. Step 0 on the merged head (job `layout-dyn-phase-4-lint0`): clippy `--workspace --all-targets --all-features
   -D warnings` and `cargo xtask static`, rc 0.
3. Measurement, Zen 1 host, run 1's method (`series.sh`: products built once with the link-logging wrapper, each
   library relinked with k pages of cold text, k = 0 relinks byte-identical, warm-up + 5 runs each), no
   `cargo xtask test` (brief):
   | profile (job) | libc.so.6 padded k = 0 / 8 / 16 | ld.so padded k = 0 / 8 / 16 | hot_text_products |
   |---|---|---|---|
   | fast (`layout-dyn-phase-4-fast`) | 82 / 82 / 82 | 82 / 82 / 82 | 82 (> old bound 81) |
   | release (`layout-dyn-phase-4-release`) | 77 / 77 / 77 | 77 / 77 / 77 | 77 (> old bound 76) |
   Every run identical. Size-independent on Zen 1; both = Skylake-SP pre-DL-23-2 level (81 / 76) + DL-23-2's one
   accepted page (ruling #6756). Both jobs exit 1 only because the old bound failed; that is what the bound commit fixes.
4. `DYN_TRUE_BOUND` [81, 76] -> **[82, 77]** (higher host class per profile, no margin; still tighter than next's
   [89, 86]). D-X.133 and the test comment record the numbers.
5. Final job `layout-dyn-phase-4-final` on bd52a0a39: clippy, static, records check rc 0; fast `so` +
   hot_text_products **28 passed** (82, bound 82). Its release step is void: `NIJI_FULL_PROFILE=1 step cargo ...`
   (an assignment before a shell function) did not reach cargo under dash, so the prefixed-interpreter trace test
   built a fast interpreter and compared it with the release products' trace (77 vs 97 names); the bound test there
   read 77 (bound 77). The job therefore exits 1; it is listed as SUPERSEDED for its release half.
6. Job `layout-dyn-phase-4-rel` on bd52a0a39: release `so` + hot_text_products with the variable exported:
   **28 passed**, 77 (bound 77), rc 0.
7. Job `layout-dyn-phase-4-fastfinal` on bd52a0a39 (starts when `-rel` ends): clippy, static, records check, fast
   `so` + hot_text_products, so the fast half has a clean job of its own. PENDING at hand-back (wall-time rule);
   the dispatcher verifies it. Expected as in step 5's fast half (28 passed, 82).
8. Process slip, for the record: one empty heredoc was typed in a local no-op shell command (nothing written or run).

## Not verified / open
* Skylake-SP with DL-23-2 merged was not measured (this run is pinned to the Zen 1 host). Expected 82 / 77 (its
  pre-DL-23-2 level + the one page); the train's gate runs on Skylake-SP and would show a difference.
* No plants this run: no code changed, only the test bound and records (run 1's plants cover the three levers).
* `cargo xtask test` and the NIJI_FULL_PROFILE checks beyond hot_text_products not run (brief: code unchanged since
  run 2's tests; the train is the FULL gate).

## Next step
Land via the train. Follow-up idea (from run 2): the ld.so start-up group overflows its second 64 KiB text window by
about 0x200 bytes; a cold split of rare logic in the group's functions would get the fast fault back (82 -> 81).

## Scripts (never committed), keke `~/work/LAYOUT-DYN-PHASE-4/`
`count.sh <role> <fast|release>` (series + htp), `final.sh <role>`, `series.sh`, `padprobe.py`, `cc-log.sh`, `attr.py`.
