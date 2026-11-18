# STARTUP-GOT-PAGE: start-up reads no GOT entry; RULING NEEDED on how the first RELRO window is paid

State: branch `task/startup-got-page-3` (continues `task/startup-got-page-2`, `task/startup-got-page`), cut from
origin/next 923df67e9. Code head b91d73666 (unchanged in run 3; run 3 adds only this note and its report). Role
keke:~/w/n1 (Zen 1 host). Decision `records/decisions/D-X/D-X.122.toml` (amends D-X.101, D-X.15). Plants:
`handoff/STARTUP-GOT-PAGE-2-plants.txt`. Run 3 scratch (keke): `~/work/startup-got-page-3/` (`proof3b.sh`,
`pfrelay.sh`, `c-got-align.patch`, clones `rel/` and `fast/`).

## What the branch does
* `xtask/src/copies.rs` (run 1, "relaxation"): 64-bit `test`/binop GOT operands are marked `R_X86_64_REX_GOTPCRELX`,
  so the static link turns them into immediates; the static probe has no `.got`.
* `crates/boundary/src/runtime.rs` (run 2, ruling #6541 option B): `protect_relro` reads RELRO's first byte before
  its `mprotect`, so start-up maps RELRO's first 64 KiB window.
* `tools/hottext/startup_data.py` (run 2): a `cmp`/`test` immediate is not a start-up data touch.

## Run 3: why gate3 was red (measured; nothing changed)
gate3 failed only the RELEASE-profile `init_short_programs_stay_within_their_page_fault_bounds`:
`startup-only program (whole process, release products): 11 page faults > 10`. Reproduced in a scratch clone.
Cause, from per-fault records of the relayed run (`perf record -e page-faults -d`, both processes): the `true`
probe takes its count (`getrusage`) BEFORE its own closing `fprintf`. In release, base's only RELRO read in `true`
is that `fprintf`'s `.got` read (`stdio::parts_first_use`), after the count. B adds a counted start-up RELRO read
(fault at 0x7c0000 by `runtime::start`). The relay parent shows the same thing: 12 -> 13 faults, and the one extra
is B's read. The exec'd child's list is the same length on base and B; only the RELRO fault's address moves. So the
extra page is B's deliberate read. It costs a page in every program that reads no RELRO. Following the coordinator
note, no bound was raised.

## Measurements (keke, static probe, ASLR off; relayed minflt 10 runs each, all identical; tests = page_fault bound tests)
| profile / state (base 923df67e9) | true | locale | tool | fork | first %.2f | DATA-WINDOWS rw | bound tests |
|---|---|---|---|---|---|---|---|
| release base | 10 | 42 | 44 | 5 | 3 | 8 | ok |
| release relaxation alone (0006344e5) | 10 | 42 | 44 | 5 | 3 | 7 | ok |
| release B (b91d73666) | **11** (bound 10) | 42 | 44 | 5 | 3 | 7 | FAIL (true) |
| release C (base + `.got` 4 KiB-aligned) | 10 | 42 | 44 | 5 | 3 | 8 | ok |
| fast base | 11 | 42 | 43 | 5 | 3 | 7 | ok |
| fast relaxation alone (run 2, next 18cf8293f) | 10 | - | 43 | 5 | **4** (bound 3) | 7 | FAIL (float) |
| fast B | 11 | 42 | 43 | 5 | 3 | 6 | ok |
| fast C | 11 | 42 | 43 | 5 | 3 | 7 | ok |
Whole-process test counts (release): tool 45, locale 43, localtime 36, localtime+mktime 36 in every state.
instructions:u (`perf stat -r 5`) true / tool / fork: fast base 239845 / 296519 / 240387, fast C 239893 / 296479 /
240394, fast B 239858 / 296313 / 240409, release C 239924 / 296405 / 240462, release relaxation 239908 / 296368 / 240452 (each state's spread is
about ±80, run 2). C prototype: `csys/src/machine/entry.S` adds an empty `.got` input section `.balign 4096` after
the `.data.rel.ro.local` one, so GNU ld starts the output `.got` on a page. RELRO grows 0x7000 -> 0x8000 (release)
and 0xc000 -> 0xd000 (fast).

Reading: B costs release `true` +1. Relaxation alone costs fast first `%.2f` +1: in fast, `render_simple` reads a
trait-object vtable in `.data.rel.ro`; in release that call reads no RELRO. C is at base everywhere in both profiles.
It keeps the `.got` reads and does not give the relaxation's one fewer written start-up page (release 8 -> 7,
fast 7 -> 6 on this base). Its float
protection is still placement luck: start-up's `.got` read (`locale::refresh_words`, fast) shares the 64 KiB window
with the vtable only while RELRO stays under 64 KiB. It avoids LSG-5's failure by construction (0xc8 bytes of
`.got` from a page start never span two pages), but that was not measured on LSG-5.

## RULING NEEDED (ruling #6541 follow-up)
B costs a page in short programs (release `true` 11 > 10). Options, all measured above except D:
* C (recommended, per #6541's own fallback): revert B and the relaxation, page-align `.got` in `entry.S`. All
  bounds at base in both profiles. Costs ~4 KiB of RELRO and keeps one written `.got` page.
* Relaxation alone, plus a follow-up story so the fast-profile first `%.2f` reads no RELRO (D, NOT measured): best
  counts (fast `true` 10, one fewer written start-up page), but it changes printf's fast-profile codegen.
* B as is: needs TRUE_BOUND release 10 -> 11 (a loosening; not done).

## Jobs (run 3, keke)
`startup-got-page-3-lint0` (clippy all-targets + static on the inherited head, rc 0); evidence, not gates:
`startup-got-page-3-proof-rel` (release B, base; its relaxation-only state is VOID: see below),
`startup-got-page-3-proof-relc` (release C), `startup-got-page-3-proof-relrelax` (release relaxation alone, clean
rerun), `startup-got-page-3-proof-fast` (fast base, C, B). VOID: `proof-relc` waited for `proof-rel` with an
in-job `~/bin/jobwait`, which returned 3 at its own timeout. Its `git checkout -f` then switched the shared clone
during `proof-rel`'s relaxation-only build, so that state's numbers were discarded and rerun (`proof-relrelax`).

## Not to redo
Options A/B/C are measured in both profiles; do not re-measure. Do not move the read into a `#[cold]` function (run 2).

## Exact next step
Apply the ruling. For C: remove the start-up read (`runtime.rs`), the relaxation (`copies.rs` step 3 and its unit
test and plants) and the cmp-immediate tool change if no longer needed. Add the `entry.S` `.got` alignment, rework
D-X.122 (and its plants for C), run release and fast `hot_text_products`, the gate and plants on the final head.
Plan rows: D-X.101, D-X.15, D-X.122.
