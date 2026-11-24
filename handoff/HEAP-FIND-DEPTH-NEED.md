# HEAP-FIND-DEPTH-NEED: handoff (leg 5: HEAP-FIND-DEPTH-NEED-3)

State: branch `task/heap-find-depth-need-3` (continues `task/heap-find-depth-need-2`), origin/next `de86ab08b`
merged in (`ee4067c64`, clean). Code fix from leg 1 unchanged: `EXACT_BLOCKERS` 9 (crates/boundary/src/heap.rs).
This leg changes only the test (`find_over_200000_entries_bounds_its_address_space`, crates/core/tests/heap_meta_products.rs),
a new oracle `crates/core/tests/oracle/vm_size_at_exit.c` and D-3.25 (`records/decisions/D-3/D-3.25.toml`).
Role shiki `~/w/sh1` (Haswell-EP). Report: handoff/reports/HEAP-FIND-DEPTH-NEED-3.json. Plants: handoff/HEAP-FIND-DEPTH-NEED-3-plants.txt.

## Why the last leg went red, and the rule that replaces the constant

The bound was a KiB constant (33576) on need(200000 entries) - need(3 entries). The 3-entry baseline cancels start-up,
not where the heap's 16 MiB-aligned segments land; every train moved it (33344, then 33808 with
INTERNAL-ALLOC-ROUTING-3). Coordinator (systemic #3049): gate on what the test protects.

Rule now, per layout phase (8 phases 2 MiB apart, as before): need(200000) under `ulimit -v` (page-exact) <= the VmSize the
same walk leaves mapped at exit WITHOUT a limit + 548 KiB. Same run, layout, phase; start-up, the library's own
allocations and the walk's real use are in both and cancel. The VmSize comes from a freestanding object (raw syscalls,
no libc) preloaded into every run (`--preload`); it writes to fd 3 because find closes stderr at exit before
destructors run. 548 = 2 x 256 KiB segment metadata (a slot whose growth another mapping blocks: one more segment) +
9 blocker pages. Issue #21's formula (need over 3 entries <= 35000) is asserted too.

| Haswell-EP, page-exact | correct (this head) | 8 blockers (the cause) |
|---|---|---|
| need - unlimited VmSize, 8 phases | 0,0,0,0,256,264,256,256 | 1188 x4, 1444 x4 |
| need over 3 entries | 33232..33752 | 34420..34932 (passes 35000 alone) |

Margin: 284 KiB above the correct maximum, 640 below the mutant minimum. The unlimited footprint is the same for the
mutant (49160/49416): the bug only shows under a limit, which is the property.

INTERNAL-ALLOC-ROUTING-3 (diagnostic composition, merged locally, not pushed; same harness with preload): need(200000)
identical at all 8 phases, need(3) 4 KiB lower (15924 vs 15928), so +4 over 3 entries. No real address-space cost on
this walk; train 973's 33808 is a layout-phase reading, not something IAR-3 adds.

## Results (shiki, Haswell-EP)

- lint0 (inherited head 429cd4795): clippy + static green.
- meas1 / meas-iar3 / meas-mut8: diagnostic harness (`~/work/heap-find-depth-need-3/measure.sh`), numbers above.
- lint1: clippy + static green; the test then failed (no VmSize line: stderr closed by find); fixed in 1c8ecd047.
- test2 (1c8ecd047): the find test passes, 27 s, numbers above.
- plants (dda23e4eb): 2 of 2 CAUGHT by the rule's assertion, no WARN. blockers-8-find: 1184 KiB over at phase 0
  (needs 50348, maps 49164); never-aligned-find (no aligned segment once the over-mapping is refused): 24472..28416.
- gate (direct tier, `affected --run`, dda23e4eb) and lint2 (clippy + static, final code head): see the report json.
- NOT measured this leg: Zen 1 and Skylake-SP (no role there). CI on the Skylake-SP host prints the per-phase lines.

## Open / next

- INFORMATIONAL (heap, not this story; from leg 4): HEAP-SEGMENT-GROWTH-ROOM, placing a later segment where it can
  grow would remove the 256..264 KiB under-limit excess at half the phases.
- If the rule goes red: read the per-phase lines. "maps" moving with "needs" is real use (not this test's concern);
  a gap above 548 is a placement failure under the limit. If the heap ever unmaps segments before exit, the
  unlimited VmSize is no longer the walk's footprint and the baseline must move (the message says so via the gap).
- Next step: the coordinator's landing.
