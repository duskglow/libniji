# LAYOUT-BYTE-BUDGETS-3 handoff (owner ruling R2 for written start-up data: STARTUP-DATA-WINDOWS becomes a release metric)

**State:** branch `task/layout-byte-budgets-3` (base origin/next 50abc44ed); head and jobs in
`handoff/reports/LAYOUT-BYTE-BUDGETS-3.json`. Role mei:~/w/s3 (Skylake-SP). Decision **D-X.138** (reserved; record
`records/decisions/D-X/D-X.138.toml`, amends D-X.62 and D-X.136). Plants: `handoff/LAYOUT-BYTE-BUDGETS-3-plants.txt`.

## What is in force

* `WINDOW-BIN-BYTES` (`xtask/src/binbudget.rs::data_bins`, `data_verdict`; wired in `xtask/src/layout.rs::check_window_bins`)
  now also gates **start-up data bins**: the objects STARTUP-DATA-WINDOWS counts (`keys=startup-data.keys` plus the
  RIP-relative operands of the `text-keys=` functions, shared helper `layout.rs::startup_objects`), grouped by output
  section; bytes from the page (written) or window (read-only) boundary below the first object to the end of the last,
  within `data-bins-<profile>` units. Rules:
  * sections whose start-up data shares a page are joined in one bin (`.data1+.bss`): otherwise `.data1` could grow
    within its own bytes, push `.bss`'s start-up words onto a page of their own and stay green;
  * objects an untouched unit apart in one bin are separate runs, each run but the last charged whole units
    (release `.lbss`: two large buffers 0x4f000 apart = 2 pages, not an 80-page span);
  * a section holding start-up data with no budget, a budget no start-up data lies in, a bin joining written and
    read-only sections: red. So the budgets add up to the count: green bytes => count <= sum of units.
* `STARTUP-DATA-WINDOWS` lists every `ro window` / `rw page` with its address on every build; over a budget it prints
  a non-blocking `PAGE-COUNT REPORT` and blocks only with `NIJI_FAULT_BOUNDS=enforce` (the release ratchet switch
  -2 introduced for hot_text_products). Budgets unchanged (ro 1/1, rw 7 fast / 8 release). The verdict cache keys on
  every `NIJI_*` variable, so enforce never gets a cached pass.

## Levels (mei, base 50abc44ed)

| bin | fast | release |
|---|---|---|
| `.data.rel.ro` (written) | 6,728 of 2 pages | 4,153 of 2 pages |
| `.got` | 457 of 1 page (starts 272 into its page) | 3,713 of 1 page (starts 3,696 in: **383 headroom**, the next edge) |
| `.data1+.bss` | 11,784 of 3 pages (headroom 504) | 11,784 of 3 pages |
| `.lbss` | 4,096 of 1 page (heap reserve head) | 8,192 of 2 pages (two buffers) |
| `.rodata1` (read-only) | 37,888 of 1 window | 34,398 of 1 window |
| sum | 7 pages = rw-budget-fast 7 | 8 pages = rw-budget-release 8 |

## Step 3: the 8th page on THREAD-FORCED-UNWIND-2

Real, and now predicted in bytes. Its merged products (job thread-forced-unwind-2-so1, mei role s1, libc.a 06:24;
copied read-only to RAM scratch with a `.profile` = fast, checked with `NIJI_LAYOUT_CACHE=0 cargo xtask layout-check
<copy>` from this branch): `.data.rel.ro` is 0xafc8 bytes (next: 0xb110, 0x148 longer), so `.got` (200 bytes) starts
at 0x86afc8, 4,040 bytes into its page, and straddles 0x86b000: `refresh_words`' slot at 0x86aff8 sits alone on page
0x86a000. Verdict: `WINDOW-BIN-BYTES BROKEN: data bin .got (written): 4225 bytes from its 4096-byte boundary > 4096 (1
pages; headroom -129): its start-up objects touch 2 pages (0x86a000 0x86b000); sections .got 0x86afc8-0x86b090 (4040
bytes into its unit)`; STARTUP-DATA-WINDOWS prints 8 > 7 as a non-blocking report. So TFU-2 stays red, now naming the
bin and bytes: `.got`'s page position is set by the size of `.data.rel.ro` mod 4,096, which no story controls on purpose.
A layout fix (not this story): give `.got` a page of its own or put it ahead of `.data.rel.ro` (RELRO ordering), so
its 200 bytes stop moving with `.data.rel.ro`; until then any story changing `.data.rel.ro` by the wrong amount meets
this red (release has 383 bytes of headroom).

## Not verified / open

* The data bins name the largest objects, not objects new or grown against a base (`NIJI_LAYOUT_BASE_BINS` holds
  text members only).
* The train must set `NIJI_FAULT_BOUNDS=enforce` at release to keep the counts ratcheted (tooling outside the repo, as
  for -2).

## Jobs

See the report JSON. Superseded: `-unit1/-unit2` (before the record existed / earlier code), `-plants1` (head
a956dd655, 4 of 4 CAUGHT before the run-split change), `-relhot1` (release red on `.lbss` span: fixed in 1f8e05c7b),
`-lint2`, `-xtask2` (started on a stale role tree after a BUSY sync; stopped by me).
