# STARTUP-GOT-PAGE: start-up reads no GOT entry, so `.got`'s place cannot move a written start-up page

State: branch `task/startup-got-page`, cut from origin/next 923df67e9; code 341f4f7d3, record + plants 9be5c2f79
(+ this note). Role keke:~/w/n1 (Zen 1 host). Decision `records/decisions/D-X/D-X.122.toml` (id reserved via ops;
amends D-X.101 and D-X.15). Plants: `handoff/STARTUP-GOT-PAGE-plants.txt` (4). Scratch scripts (keke):
`~/work/startup-got-page/{gotprobe,proof,gate}.sh`; local copies in the run's scratch dir.

## What the branch does
* `xtask/src/copies.rs`: D-X.15 step 3 (`mark_relaxable`) also marks the 64-bit `test` and binop GOT operands
  (`REX.W` + `85 03 0b 13 1b 23 2b 33 3b`, RIP-relative ModRM) `R_X86_64_REX_GOTPCRELX` (`operand_kind`). The static
  non-PIE link turns them into immediates (same length, text layout unchanged); PIE and `libc.so.6` keep the GOT
  read. Step 1 (memcpy redirection) unchanged. Unit test `test_and_binop_got_operands_become_relaxable_and_nothing_else`.
* `crates/core/tests/hot_text_products.rs`: `init_startup_data_pages_hold_at_every_data_phase` adds `relro-tail` pads
  (0x10-0x1000 bytes of `.data.rel.ro` behind the cluster, moving `.got`); no `.got` read in the hot text
  (`got_reads`) and the same written relro page count at every pad. Passed on head (try1) and on the merged tree.
* Chosen over aligning `.got` (up to 4095 B relro, `.got` still a written start-up page): `so` leaves 0 GOT uses
  unrecognised (160 before: 147 add, 13 cmp) and the static probe has no `.got` at all.

## Measurements (fast products, keke; perf user page faults / instructions, `setarch -R`)
| build | STARTUP-DATA-WINDOWS | true | tool | fork | first %.2f (bound 3) |
|---|---|---|---|---|---|
| origin/next 923df67e9 | 7 <= 7 | 82 / 230332 | 108 / 286446 | 98 / 230873 | not run |
| next + this branch | 7 <= 7 | 81 / 230331 | 108 / 286344 | 98 / 230867 | gate1 pending |
| task/locale-slots-growth-5 1ed232a5a | 8 > 7 BROKEN (`.got` reads on 0x86a000 + 0x86b000) | 82 / 239081 | 107 / 295966 | 98 / 239630 | 3 (pass) |
| LSG-5 + this branch (merge ec4ae5228) | 7 <= 7 ok | 82 / 239093 | 108 / 295976 | 97 / 239613 | **4 > 3 FAILED** |
Proof log: keke `~/logs/startup-got-page-proof1.log` (`grep ^proof:`), trees in /dev/shm/claude/startup-got-page/proof-1.

## OPEN: the merged tree's first %.2f 3 -> 4 (and tool 107 -> 108)
Hypothesis, NOT verified: the start-up `.got` read was a READ fault in relro's 64 KiB window, so fault-around mapped
the read-only relro pages near it (the non-start-up `.data.rel.ro` tail that first calls such as the float / locale
paths read). With no `.got` read, the first such read takes a fault of its own. If true, the change moves a fault
from start-up into the first call that reads relro tail data (true 82 -> 81 on next, but float +1 on the merged tree).
Also unexplained: STARTUP-DATA-WINDOWS stays 7 on next+branch although `.got`'s page is gone (102 start-up objects vs
99 on next: the trace-at-build keys differ; get the page list with the contract's budgets at 0 in a scratch clone,
as proof.sh's predecessor did).

## Jobs (keke)
* `startup-got-page-gate1` (head 9be5c2f79, `~/bin/gate 16G sh ~/work/startup-got-page/gate.sh`): clippy rc 0,
  static rc 0, xtask unit test 1 passed; then affected --run --base origin/next, cargo xtask test, release so +
  hot_text_products (logs `~/logs/startup-got-page-{affected,xtest,rel}.out`). Was running at the refresh.
* `startup-got-page-plants1` (dir /dev/shm/claude/startup-got-page/plants-1): was running at the refresh.
* `startup-got-page-proof1` rc 0 (above). Superseded: `startup-got-page-try1` (rc 2: only gotprobe.sh failing on a
  link without `.got`; its so and data-phase test passed), `startup-got-page-so0` (next's products, base pages).

## Exact next step
1. `~/bin/jobwait startup-got-page-gate1 startup-got-page-plants1` on keke; read the float/page_fault lines of
   `~/logs/startup-got-page-affected.out` / `-rel.out`: is first %.2f 3 or 4 on next+branch?
2. Test the hypothesis on the merged tree (/dev/shm/claude/startup-got-page/proof-1/tree, products built there):
   `perf record -e page-faults -d` on `merged.probe/pf float` vs `other.probe/pf float` and compare the faulting
   data addresses (is the new fault in `.data.rel.ro` beyond the cluster?).
3. If confirmed: the fix is a design choice (RULING NEEDED, or a measured fix such as putting the relro data the
   first float/locale calls read into the start-up relro cluster via `startup-data.keys`/rw keys); the change must
   not land while the float bound is red (ALWAYS FORWARD). Else (layout jitter of the merge): report with numbers.
4. Then the hand-back: report json, handback-check, JOBS line (gate1, plants1, proof1).
