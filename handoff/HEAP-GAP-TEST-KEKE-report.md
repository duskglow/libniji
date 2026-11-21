# HEAP-GAP-TEST-KEKE report

Branch `task/heap-gap-test-keke`, base origin/next `43987d4dc`. Build role keke `~/w/n2` (Zen 1, 60 vCPU, 2 NUMA nodes).

## Commits

- `d02fbab84` Take the gap test's address-space figure without allocating (vm_size() reads statm into a stack buffer;
  the gap test takes the figure after its layout; the assertion carries the errno).
- `e60a274b9` Name the processor class, not a host, in the gap test's comment (static lab-names).
- `332e67479` Hold the gap test's limit to exactly the gap it re-maps (limit = vm at the pin + SEG + PAD).
- `111418a7e` Match the gap plants on the failing test line affected prints. This report and the JSON follow.

## Cause (measured)

Diagnostic build (pass 2, keke, `affected --only niji-boundary::s03_memory_symbols --with-exports --run`):
`back=Err(ENOMEM)`, limit 38617088, vm at the check 19755008, limit - vm = 18862080 < SEG + PAD = 18874368 (3 pages
short). vm grew 2109440 B after the figure: 3 pin pages + exactly 2 MiB. No mapping of `map`'s was in the gap (maps:
only the two pins at the gap's ends). The 2 MiB: `vm_size()` used `read_to_string`; std reads a short file into a stack
probe buffer before it allocates, so statm was read first and the `String` allocated after. In a libniji process that
allocation is the test thread's first, and libniji's heap maps 2 MiB of address space for it; in a host-C-library
process none is added, so the 2 MiB - 3 pages of slack held. Not a heap placement defect. Not load, NUMA or the host.

The heap side: a thread's first allocation adding 2 MiB of address space (MAP_NORESERVE) is ordinary heap growth, not a
defect; which structure it is (thread cache chunk or segment growth) was not identified (INFORMATIONAL below).
How a program could meet it: only a program that sets RLIMIT_AS from its own statm with no headroom and then allocates
on a new thread; the allocation would then fail with ENOMEM as under any limit. No libniji defect.

## Pass counts (20 each unless noted)

| tree | host | glibc process native | glibc process interposed | libniji process (pass 2 runner) |
|---|---|---|---|---|
| next `43987d4dc` | Zen 1 | 20/20 | 20/20 | 0/20 unpinned, 0/20 cpu 0, 0/20 node 0, 0/20 node 1 (VALGRIND-LDSO-SYMBOLS-2's copy, ENOMEM at line 1008) |
| COMPAT19 tree (next-based) | Skylake-SP | 60/60 (compat19-ldso-cost-kpages, cargo test) | - | FAIL in pass 2 of xtest4 and xtest5, PASS in pass 1 |
| fix `332e67479` | Zen 1 | 20/20 | 20/20 | pending: repeat5 (20 affected runs unpinned + 20 on cpu 0, the pass-2 runner) |
| fix | Zen 1 gate | pass 1 PASS | interposed PASS | pass 2 (native) PASS |

Not run: the fix in pass 2 on the Skylake-SP host (no role there; the cause is host-independent) and the 1509aeb3 tree
and bisect (the mechanism is measured, so the bisect would only date when the heap's first-allocation mapping in a new
thread started exceeding the slack; skipped for budget).

## Why CI let it land, and coverage

`affected --run` runs pass 2 for niji-boundary targets only when the change touches mem, rt, kernel, abi or boundary
(`affected.rs`, `interpose`). HEAP-ADDRESS-SPACE-10 touched boundary, so its CI run should have run this test in pass 2;
whether it did and passed at 71597d2a4 was not checked (CI logs not read). Its own runs never did: its 20/20 and
plants were `cargo test` (glibc processes) and its `xtask test` stopped at the products step. Branches that touch none
of those crates (VALGRIND-LDSO-SYMBOLS-2, COMPAT19-LDSO-COST) never run it in pass 2 in CI; only a full `xtask test`
does. Recommendation: plants on heap or address-space code run through `affected --run` (pass 2), never `cargo test`;
and the RLIMIT_AS / gap tests in s03 belong in the train's pass 2 for any change under crates/rt heap or boundary heap.

## Jobs (keke)

| label | what | rc |
|---|---|---|
| heap-gap-test-keke-lint3 | clippy --workspace --all-targets --all-features -D warnings; xtask static | 0 |
| heap-gap-test-keke-gate3 | `cargo xtask affected --run` (direct tier): 3 suites, 195 passed, peak 1.7G | 0 |
| heap-gap-test-keke-plants4 | `cargo xtask plants` (2 plants, through pass 2) | pending (plants3 on the same plants saw both fail the target test; only its msg did not match) |
| heap-gap-test-keke-verify4 | see -repeat5 | superseded by -repeat5 (harness picked the wrong binary and root) |

Superseded: -rep-next, -rep2-old, -build2, -diag1, -diag2 (diagnostics); -lint (static lab-names red), -gate and -plants
(stopped for that fix); -lint2/-gate2 green and -plants2 SURVIVED (limit slack equalled the 2 MiB; fixed by 332e67479);
-plants3 2 WARN-NO-MSG (msg did not match affected's output; fixed by 111418a7e); -verify3's libniji rows invalid (my
directly-run copy used a stale prefixed loader: `NIJI_PRIVATE build tag differs`); its glibc rows are the 20/20 above.

| heap-gap-test-keke-repeat5 | 20x `affected --only niji-boundary::s03_memory_symbols --with-exports --run` unpinned and on cpu 0; tallies the gap test per pass | pending |

## INFORMATIONAL

- NUMA lead (coordinator): retracted for this test (fails pinned to one cpu; passes nowhere as a libniji process). The
  heap_meta_products node-migration sensitivity was not examined here.
- The libniji heap maps 2 MiB of address space on a new thread's first small allocation; structure not identified.
- Plant 1 failed in the exports-interposed pass of the plants run; plant 2 in all three passes.
