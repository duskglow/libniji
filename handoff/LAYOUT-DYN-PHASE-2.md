# LAYOUT-DYN-PHASE-2: Zen 1 measurement of the size-independent dynamic start-up count (continues LAYOUT-DYN-PHASE)

State: branch `task/layout-dyn-phase-2` = run 1's `task/layout-dyn-phase` (92690d7ec) + merge of origin/next fb63eeec8
(ed4deec20) + the bound commit 321d2e74a; the report commits on top change only handoff/. Role mei:~/w/s1 (Skylake-SP).
Decision in force: **D-X.133** (updated this run); run 1's full report: `handoff/LAYOUT-DYN-PHASE.md`.

## Verdict: INCOMPLETE (the Zen 1 host class is still not measured)
The dispatcher admitted this run to mei:~/w/s1 (Skylake-SP), not to a Zen 1 role. The brief's job (fast and release
`so` + hot_text_products and the k = 0, 8, 16 series on a keke role) needs a keke role. A ship-the-source-to-keke run
without a role was refused for LAYOUT-LDSO-WINDOW (ops #6563/#6564), so it was not attempted. Next run: pin the story
to keke (`ops pin LAYOUT-DYN-PHASE-2 keke`).

## What this run did (Skylake-SP, mei)
1. Step 0 on the inherited head: `cargo clippy --workspace --all-targets --all-features -- -D warnings` rc 0,
   `cargo xtask static` rc 0.
2. Merged origin/next fb63eeec8 (52 commits; clean merge).
3. Job `layout-dyn-phase-2-sky` on ed4deec20 (merged head, bound still 80/76): records check, fast `so`, `-p niji-ld`
   and `-p xtask` tests all passed; fast hot_text_products **81 > 80** (the only failure, also the only failure of
   `cargo xtask test`: 6836 passed, 1 failed); release `so` + hot_text_products 76 (bound 76). Series (run 1's method,
   `series.sh` with k = 0, 8, 16, both libraries padded, k = 0 relinks byte-identical, every run identical):
   | | libc.so.6 padded k = 0 / 8 / 16 | ld.so padded k = 0 / 8 / 16 |
   |---|---|---|
   | fast, merged | 81 / 81 / 81 | 81 / 81 / 81 |
   | release, merged | 76 / 76 / 76 | 76 / 76 / 76 |
   | run 1 (pre-merge, k = 0..16) | fast 80, release 76 at every k | same |
   So the count is still size-independent; the fast +1 comes from next's code.
4. Attribution (`attr.py`, perf page-faults split by mapping, jobs `-attr-pre` and `-attr-merged`): pre-merge
   (run 1's products) vs merged fast products differ only in **ld.so r-xp 3 -> 4**. A new fault at image offset
   0x21000: `niji_ld::dlopen::loadable` moved 0x20ec0 -> 0x210c0 (ld.so text 0x47f7b -> 0x4a4fb, +0x2580 from next's
   loader code). It is the last entry of the traced start-up group (`target/product/fast/ldso-text.list`, 109
   functions), which now ends about 0x200 bytes past the group's second 64 KiB window (text at offset 0x1000 starts a
   window; D-X.125).
5. `DYN_TRUE_BOUND` [80, 76] -> **[81, 76]** (the merged head's Skylake-SP level, no margin; next's landed bound is
   [89, 86], so this is still a tightening); D-X.133 records the merged numbers and the cause.
6. Final gate `layout-dyn-phase-2-final` on 321d2e74a (same steps, no series): see the hand-back / report json.

## Open / next step
* **Zen 1 (run 3, on a keke role):** sync this branch, then run `m2.sh` (below; it does records, fast `so`, the loader
  and xtask tests, fast htp, `cargo xtask test`, release `so` + htp, and the k = 0, 8, 16 series per profile). Set
  `DYN_TRUE_BOUND` to max(Skylake-SP 81/76, Zen 1) per profile, no margin; add the Zen 1 numbers to D-X.133 and the
  test comment. If Zen 1 reads above 89/86 (next's bound): RULING NEEDED, stop. Plants: run 1's stay (no code changed).
* Scripts (never committed) on mei in `~/work/LAYOUT-DYN-PHASE-2/`: `m2.sh` (gate + series), `m3.sh` (gate only),
  `series.sh` (work dir `~/work/LAYOUT-DYN-PHASE-2`; edit `W=` when copying), `padprobe.py`, `cc-log.sh`, `attr.py`.
  On keke copy them to `~/work/LAYOUT-DYN-PHASE-3/` (scp from mei through the local host) and fix `W=`.
* Follow-up story idea: the ld.so start-up group overflows its second 64 KiB window by about 0x200 bytes; moving rare
  logic out of the group's functions (cold split) would get the fast fault back.

## Not to redo
Run 1's series, record, plants and the three build checks; the Skylake-SP merged series above.
