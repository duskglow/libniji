# STARTUP-GOT-PAGE: start-up reads no GOT entry, so `.got`'s place cannot move a written start-up page

State: branch `task/startup-got-page`, cut from origin/next 923df67e9; code 341f4f7d3, record + plants 9be5c2f79,
leg-2 record note + this handoff on top. Role keke:~/w/n1 (Zen 1 host). Decision `records/decisions/D-X/D-X.122.toml`
(amends D-X.101 and D-X.15; a pending-ruling paragraph added in leg 2). Plants: `handoff/STARTUP-GOT-PAGE-plants.txt`
(4). Scratch scripts (keke): `~/work/startup-got-page/{gotprobe,proof,gate,pfaddr}.sh`, `pfaddr.py`.

## Verdict: INCOMPLETE, RULING NEEDED (FLOAT_BOUND)
The change does what the brief asks (STARTUP-DATA-WINDOWS 7 on next and on the LOCALE-SLOTS-GROWTH-5 merged head,
which reads 8 > 7 without it), but `init_first_float_conversion_stays_within_its_page_fault_bound` reads
**4 > 3** with it, on origin/next + branch (relay float 4,4,4) and on the merged head. Cause (verified in leg 2, see
below): a fault moves from start-up into the first call that reads relro. Raising the bound is loosening it (ALWAYS
FORWARD), so it needs a maintainer's ruling.

## What the branch does
* `xtask/src/copies.rs`: D-X.15 step 3 (`mark_relaxable`) also marks 64-bit `test`/binop GOT operands
  `R_X86_64_REX_GOTPCRELX`; the static non-PIE link turns them into immediates (same length). PIE / `libc.so.6` keep
  the GOT read. Unit test `test_and_binop_got_operands_become_relaxable_and_nothing_else`.
* `hot_text_products.rs`: `init_startup_data_pages_hold_at_every_data_phase` adds `relro-tail` pads 0x10-0x1000 that
  move `.got`; no `.got` read in the hot text and the same written relro page count at every pad.

## Leg 2 diagnosis (keke, `pfaddr.sh <probe dir> <mode> <out>`: `perf record -e page-faults:u -c 1 -d`, ASLR off)
* LSG-5 head alone (`other`): the only relro access during start-up is the `.got` read in `locale::refresh_words`
  (0x86afb8, page 0x86a); its read fault maps the 64 KiB relro window 0x860000-0x86ffff (fault-around), so the
  later `printf::render_simple` read of a trait-object vtable in `.data.rel.ro` (0x860ca0) takes no fault.
* Merged / next+branch: no `.got` read, start-up touches no relro, and that vtable read (merged 0x860ca0, next+branch
  0x8611b8) faults inside the first `%.2f`: call-only 3 -> 4. In `tool` the moved fault lands in
  `environ::getenv_many` (relro data too).
* Whole process unchanged where relro is read: `float` 17 = 17, `tool` 42 = 42 (3 runs each, identical); `true`
  82 -> 81 on next (never reads relro). The earlier `tool` 107 -> 108 (perf stat mean) is noise: the base run's
  minflt alternates 134/135, per-fault recording gives 42 = 42.

## Measurements (fast products, keke; perf user page faults / instructions, `setarch -R`)
| build | STARTUP-DATA-WINDOWS | true | tool | fork | first %.2f (bound 3) |
|---|---|---|---|---|---|
| origin/next 923df67e9 | 7 <= 7 | 82 / 230332 | 108 / 286446 | 98 / 230873 | not measured here (bound 3 holds on next's gates) |
| next + this branch | 7 <= 7 | 81 / 230331 | 108 / 286344 | 98 / 230867 | **4** |
| task/locale-slots-growth-5 1ed232a5a | 8 > 7 BROKEN | 82 / 239081 | 107 / 295966 | 98 / 239630 | 3 |
| LSG-5 + this branch (merge ec4ae5228) | 7 <= 7 ok | 82 / 239093 | 108 / 295976 | 97 / 239613 | **4** |

## RULING NEEDED (choose one)
* A (recommended): land as is and set `FLOAT_BOUND` 3 -> 4 with the reason above: no program has more faults (whole
  process equal or one lower), start-up gets one cheaper, and the bound measured a fault start-up paid by layout luck.
* B: keep the relaxation and add a deliberate read of the relro window's first page at start-up (relro starts on a
  64 KiB boundary, D-X.101 / STARTUP-DATA-PAGE-ALIGN-2): float back to 3 and `true` back to 82, all at base level,
  at the cost of one load on the start-up path (a hot-function change) and the `true` saving.
* C: align `.got` to a page instead (brief option 1): keeps the base fault pattern, up to 4095 B relro padding, and
  `.got` stays in relro's first 64 KiB window only while relro is shorter than 64 KiB.

## Jobs (keke; all on head 9be5c2f79; later commits change records/handoff only)
* `startup-got-page-gate1`: escalated to FULL (xtask changed). clippy, static, xtask unit test ok; expected red on
  the float bound (xtask test, release hot_text_products). `startup-got-page-plants1`: dir
  /dev/shm/claude/startup-got-page/plants-1. `startup-got-page-proof1` rc 0 (the table above). Superseded:
  `startup-got-page-try1`, `startup-got-page-so0`.
* Not yet run on the final head: `cargo xtask static` (the role was busy with gate1; record edit only).

## Exact next step
After the ruling: A = change `FLOAT_BOUND` (doc comment with the numbers above), rerun the direct gate, `cargo xtask
test`, `NIJI_FULL_PROFILE=1` so + hot_text_products, and static; B / C = implement, plant, then the same gates.
