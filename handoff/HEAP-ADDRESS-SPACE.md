# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-6` (continues -5 .. -1), head = this handoff's commit. Status: INCOMPLETE,
RULING NEEDED (the issue #21 RLIMIT_AS need depends on address-space placement). Code commits on -6: `8afd17494`
(window-bin planner: phase-only groups stay out of the head bin), `ce6328854` (issue #21 bounds at the measured
levels, ruling #6513; SEG_FIRST cost in the record, ruling #6514), `ad420da74` (stale mallocdbg difference dropped),
`a84abbe23` (segment placement under a limit keeps its growth room; partial fix, no plant). Run -5's code: `7c6a4078b`.
Build role keke `~/w/n3` (Zen 1); scripts in keke `~/work/HEAP-ADDRESS-SPACE-6/` (`asn*.sh`: need bisections and
strace of the issue #21 program under RLIMIT_AS; `m3.sh`: clippy, static, so, the two red suites, 40 bisections).
Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change"; rulings #6513, #6514.
Decisions in force: D-3.19, D-3.14, D-X.103, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3: growth with use), `-3-2.toml` (D-X: GOT alignment), `-6-1.toml` (D-X: amends D-X.103, head-bin rule).

## Done in run -6 (do not redo)

- Run -5's fullhot red fixed (planner put the rebind-only phase group in bin 0's tail; `head_may_take` keeps
  phase-only groups out of the head bin). `heap-address-space-6-fullhot1`: 26 passed, `pre 0x800: rebind 0`.
- Plants 3/3 CAUGHT (`heap-address-space-6-plants`, handoff/HEAP-ADDRESS-SPACE-6-plants.txt), on `ce6328854`.
- Gate `heap-address-space-6-gate1` on `ce6328854`: 6767 passed, 2 failed:
  1. `mallocdbg_equals_the_reference`: "recorded difference no longer seen" for "uordblks grew by at least
     200000". With SEG_FIRST 16 the second thread's blocks fit in already mapped slabs: libniji's line now equals the
     reference. Entry removed (`ad420da74`); the suite passes (`heap-address-space-6-m4`).
  2. `the_heap_keeps_address_space_near_its_use`: exit 12 (realloc step) under start + 42000 KiB. The need is
     not one number. Bisected to 16 KiB on keke: ASLR off (`setarch -R`) 41296 every time; ASLR on, old code
     41552..42192 (5 runs), 42000 passes 9/10. Cause (strace): under the limit the 32 MiB over-map fails, the
     fallback `map_aligned_exactly` placed the segment below the kernel's probe with a random gap above, the in-place
     grow (`MAP_FIXED_NOREPLACE`) hits EEXIST and a new 4.25 MiB segment is mapped. `a84abbe23` tries the highest
     aligned window under the gap's top first: 40 bisections 41296 (23x), 41312..41552 (9x), 41776..43088 (8x).
     The remaining tail: a start-up mapping (67104 bytes, not NORESERVE) sits inside a later segment's window. Also
     seen: when growth fails near the limit the heap retries a 4 KiB mapping plus the 32 MiB and 4.25 MiB maps on
     every allocation (strace t.8 in asn5).
- xtest1 was stopped (superseded head). No `cargo xtask precheck` exists on next. Rule slip in leg 1: one no-op
  heredoc call (empty body, ran nothing).

## RULING NEEDED

The issue #21 RLIMIT_AS check cannot be made non-flaky at 42000 KiB: the need depends on placement. Recommend:
the full fix in HEAP-SPILL-COLD (or a new story): keep the heap's other mappings out of a segment's growth window
under a limit (place segments at the bottom of a gap, or map small private regions from inside a segment), then
set AS_HEADROOM from a 40+ run bisection maximum and run the limited check several times. For landing now: run
the limited check under a fixed layout (personality ADDR_NO_RANDOMIZE in the oracle, need 41296) with the measured
spread written in the test comment, or drop the limited check until that story. Either is a coverage judgement.

## Exact next step

Apply the ruling: edit `crates/core/tests/heap_meta_products.rs` (AS_HEADROOM and how the limited run is made),
add a plant for `a84abbe23` if it stays (e.g. `lit:let fit = (room > len) ==> let fit = (room < len)`, caught
only if the test is deterministic), sync, clippy + static, gate (`cargo xtask affected --run`, ~64 min on keke),
fullhot (`NIJI_FULL_PROFILE=1 cargo xtask so` + hot_text_products), `cargo xtask test`, plants; then the report.

## Next steps for the story, in order

1. HEAP-SPILL-COLD: the spill as a cold extension of grow_segment; tighten refill/realloc/AS_HEADROOM to 33092 /
   35144 / 35152, recheck a smaller SEG_FIRST, and the placement-independent need above.
2. Still open: relro by-section comparison (ruling #6325 step 1); cbench_malloc / cbench_startup counters;
   returning empty segments' address space; findutils `find` under the issue's `ulimit -v` formula.
