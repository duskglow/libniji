# LOCALE-ADDRESS-SPACE: handoff (run 1, leg 2: fix, test and plants done)

State: branch `task/locale-address-space` (base origin/next `fb12f4d3e`), build role keke `~/w/r1` (Zen 1, 60 vCPU).
Brief: `~/state/briefs/LOCALE-ADDRESS-SPACE.md`. Decision: pending record
`records/pending/LOCALE-ADDRESS-SPACE-1.toml` (D-NEW, series D-7; CI assigns the id). Plants file:
`handoff/LOCALE-ADDRESS-SPACE-plants.txt`. Scratch scripts: keke `~/work/locale-address-space/` (`measure.sh`,
`smaps_sum.py`, `trace.sh`, `refill.sh`; outputs `m0` = base, `m1` = after).

## What was done

* `niji::mem::Permanent` (default: the heap leak; `OnHeap`), `LocaleView::new_in`, `global_locale::Objects:
  Permanent` (view, items, `Tables` and name go through it), and in `boundary/src/locale.rs` the products' arena
  (`perm_place`: private page chunks, 16 KiB first, doubling to 1 MiB, own `RawMutex` taken only inside the interning
  lock; with a replacement malloc the default heap path, D-X.156). The unsafe placement helpers are in `tb.rs`
  (`uninit_slice_at`, `uninit_at`, `filled`; category `slice` is only permitted there).
* Test `heap_meta_products::a_utf8_locale_costs_no_more_address_space_than_the_references` (same run: products vs the
  system loader, en_US.UTF-8 minus C, allowance = reference's + products' C.UTF-8 cost + 64 KiB).

## Measured (keke, find 4.9.0, 3 files on tmpfs; VmSize at exit KiB, fixed layout)

| | C | en_US.UTF-8 | C.UTF-8 |
|---|---|---|---|
| libniji before (m0) | 12896 | 18512 (+5616) | 13472 (+576) |
| libniji after (m1) | 12896 | 16480 (+3584) | 13488 (+592) |
| reference | 3736 | 6724 (+2988) | 7152 (+3416) |

The least `ulimit -v` (3 ASLR runs and fixed, spread 0) barely moves: libniji C 8716, en_US 12416 -> 12408, C.UTF-8
9304 -> 9272; the reference 3736 for all (under the limit its setlocale fails and it runs in C). de_DE is not
installed on keke (only en_US.utf8, C.utf8). Left over: +596 KiB anon = dlopen loader state (528 KiB table map +
44 + 4 KiB image views) + 16 KiB arena chunk: see the record (loader change, its own story).

## Jobs so far (keke)

* `locale-address-space-plants` (head 1f3dddf4b, code unchanged since): 2 plants, 2 CAUGHT (mutants +5632 KiB).
* `locale-address-space-clippy-final` rc 0; `-static3` rc 0 (static, records check; affected = direct, 264 targets).
* `locale-address-space-gate` (head 3b97f3f2b): two reds, both from this change:
  1. `hot_text_products::init_startup_data_accesses_are_in_the_data1_cluster`: PERM_LOCK/NEXT/END touched at
     start-up outside `.data1`. FIXED after the gate: declared in `crates/libc-so/startup-state.list` (with
     PERM_CHUNK); not yet re-run.
  2. `heap_meta_products::find_over_200000_entries_bounds_its_address_space` (heap_meta_products.rs:403, the
     `FIND_PLACEMENT_SLACK` = 548 check), phase 4096 only shown: needs 45616 KiB, maps 44840 = 776 over. Base
     (release-snapshot-1124c gate on keke, today, log `~/logs/release-snapshot-1124c-heapnode-1.log`) at phase 4096:
     needs 46844, maps 46836. So the need FELL 1228 KiB, but the at-exit mapping fell 1996 (the en_US heap growth
     step is gone), and under the limit the walk now places a later segment where it cannot grow (3 x 256 + 8).
     Not investigated yet. Do NOT loosen the slack (ALWAYS FORWARD): find the cause (heap segment placement under a
     limit with the arena chunk / without the early growth step; `trace.sh` with a limit at phase 4096), or bring
     RULING NEEDED with all 8 phases' numbers, base vs branch.
* The chain's `-xtest` and `-fullprofile` were never started (chain stopped).
* At the refresh (17:25 UTC) the gate was still running (only those two reds so far, `test -p niji` and its
  native run); `~/bin/jobwait locale-address-space-gate`, then read the rest of its FAIL lines.

## Next step

1. Sync (role was busy with the gate), run only the failing tests on the new head:
   `cargo test -p niji --test heap_meta_products find_over_200000 -- --nocapture` (prints all 8 phase rows) and
   `NIJI_LAYOUT_TRACE` products + `hot_text_products init_startup_data_accesses` (via `cargo xtask affected --only`
   or the gate) to confirm fix 1.
2. Investigate red 2 as above (start: is the 16 KiB arena chunk the mapping that sits in the aligned slot? try
   the run with the chunk mmap'd with a hint or the first chunk's size changed, compare phase rows).
3. Then gate, `cargo xtask test`, full-profile hot_text_products, precheck, report json
   (`handoff/reports/LOCALE-ADDRESS-SPACE.json`, draft placeholders HEAD_SHA, GATE_RESULT, IMPROVEMENT_LINE, jobs),
   handback-check.
