# STARTUP-GOT-PAGE: `.got` starts on a page (D-X.122, ruling #6606 option C)

State: branch `task/startup-got-page-5` (run 5, hand-back only; continues `task/startup-got-page-4`, `task/startup-got-page-3`, `-2`, `task/startup-got-page`), cut
from origin/next 923df67e9. Role keke:~/w/n1 (Zen 1 host). Decision `records/decisions/D-X/D-X.122.toml` (amends
D-X.101). Plants: `handoff/STARTUP-GOT-PAGE-4-plants.txt`.

## What the branch does (run 4, option C applied)
* `csys/src/machine/entry.S`: an empty `.got` input section `.balign 4096` after the start file's
  `.data.rel.ro.local`, so GNU ld starts the output `.got` on a page; its 0xc8 bytes (static probe) never span two
  pages whatever length of relro precedes them. Cost: up to 4 KiB of RELRO (0x7000 -> 0x8000 release, 0xc000 ->
  0xd000 fast).
* `crates/core/tests/hot_text_products.rs` `init_startup_data_pages_hold_at_every_data_phase`: relro pads of
  0x10-0x1000 bytes behind the cluster (`relro-tail`) move `.got`; the test checks `.got` starts on a page, the hot
  text's `.got` reads lie in one page and the written relro pages (cluster plus `.got` reads) hold in number.
* Removed (runs 1-2): B's start-up RELRO read (`runtime.rs`), the test/binop GOT relaxation (`copies.rs` step 3, its
  unit test, `so.rs` message), the cmp/test-immediate rule in `tools/hottext/startup_data.py` (only the relaxation
  produced such immediates). The old plants files went with them.

## Measurements (run 3, not redone; static probe, Zen 1 host, base 923df67e9)
| state | true | locale | tool | fork | first %.2f | DATA-WINDOWS |
|---|---|---|---|---|---|---|
| release base / C | 10 / 10 | 42 / 42 | 44 / 44 | 5 / 5 | 3 / 3 | 8 / 8 |
| fast base / C | 11 / 11 | 42 / 42 | 43 / 43 | 5 / 5 | 3 / 3 | 7 / 7 |
A (relaxation) cost fast first `%.2f` 3 -> 4; B (relaxation + start-up read) cost release `true` 10 -> 11.

## Run 4 jobs and results
All on code head 1e555af95, keke: `startup-got-page-4-lint` (clippy all-targets + static, rc 0);
`startup-got-page-4-gate` (escalated to FULL by itself: entry.S defines `_start`; workspace clippy and
`cargo xtask test`, rc 0); `startup-got-page-4-plants` (1 plant, CAUGHT by
`init_startup_data_pages_hold_at_every_data_phase`); `startup-got-page-4-htp-release` (NIJI_FULL_PROFILE=1 so +
hot_text_products, 26 passed); `startup-got-page-4-htp-fast` (26 passed; every whole-process count at its bound:
`true` 11, setlocale+printf 43, coreutils-shaped 44, localtime 36 / 36; STARTUP-DATA-WINDOWS 1 ro window, 7 rw
pages; at every relro pad the cluster is 2 pages and written relro pages 3). Not done: a `layout-bins.tsv` diff
against the merge base (no text changed: runtime.rs is back to base byte for byte). Note: run 4's edit script used
a heredoc once (scratch only), against the no-heredoc rule.

## Run 5 (hand-back only, branch `task/startup-got-page-5`, role mei:~/w/s3, Zen 1 host)
Merged head dd57bcca4 (run 4's merge of origin/next 3743713c; next had not moved since). `cargo xtask precheck
--jobs <run 4's labels> --head dd57bcca4`: GREEN (31 checks; the keke jobs left to handback-check).
`startup-got-page-5-lint` (clippy workspace all-targets all-features -D warnings, then static): rc 0.
`startup-got-page-5-layout` (`cargo xtask so`, fast, on the merged head and on a copy of 3743713c): rc 0; all 12
layout contracts hold on both (STARTUP-DATA-WINDOWS 1 ro window, 7 rw pages on the head); the fast
`layout-bins.tsv` is identical (543 rows, empty diff), so no start-up bin grew. Nothing redone from run 4.
`ops handback-check` is red: run 4's keke jobs tested pre-merge content (the merge of next changed crates/boundary
ld.rs, bootstrap.rs, conformance dyr_* among others), so they do not prove the merged head. Started on mei on the
merged head and still running at hand-back: `startup-got-page-5-plants` (dir /mnt/zram/claude/startup-got-page-5/
plants-1), `startup-got-page-5-htp-release` then `startup-got-page-5-htp-fast` (sequenced by
~/work/startup-got-page-5/htp.sh). Not started: the gate. It escalates to FULL (entry.S), and the coordinator said
not to start a FULL gate by hand (RULING NEEDED).

## Open / informational
* HEAP-ADDRESS-SPACE-8 adds its own 256-aligned `.got` in `entry.S`; when it merges, keep one alignment (this one,
  4096) per the coordinator's note.
* Option D (relaxation plus making the fast first `%.2f` read no RELRO) was not measured; it would save one written
  start-up page. A possible follow-up story, not needed for this one.
* The `xtask plants` `.S` rule assumes every site is in a function: this plant anchors on the preceding
  `niji_machine_ld_enter:` label (the `.got` text is unique in the file).

## Not to redo
A/B/C/D measurements (run 3). Do not move a read into a `#[cold]` function (run 2).

## Exact next step
Collect mei startup-got-page-5-plants, -htp-release, -htp-fast (`~/bin/jobwait <label>`). Then the gate on the
merged head (FULL by escalation, `cargo xtask test`) per the coordinator's ruling; then land. Plan rows: D-X.101, D-X.122.
