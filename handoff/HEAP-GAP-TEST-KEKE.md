# HEAP-GAP-TEST-KEKE: why the heap's gap test failed on the Zen 1 host

Branch `task/heap-gap-test-keke` (from origin/next `43987d4dc`); fix commit `d02fbab84`. Build role keke `~/w/n2`.

## Verdict

A test assumption, not a heap defect, and not the host, load or NUMA. `kernel_pages_map_aligned_past_a_gap_without_an_aligned_range`
fails **every time as a libniji process** (pass 2 of `cargo xtask test`, the `niji-deps` copies started by libniji's loader)
on both hosts, and passes every time as a host-C-library process (`cargo test`, pass 1).

Cause (measured with a diagnostic build in pass 2, keke): the re-mapping of the whole gap failed with **ENOMEM**, not EEXIST
(no page of `map`'s was left in the gap). The limit was `vm0 + SEG + SEG/4` (vm0 from `vm_size()` before the layout);
at the check the process had grown by 2 MiB + 3 pin pages, so `SEG + PAD` (18 MiB) did not fit (3 pages short:
limit - vm_now = 18862080 < 18874368). The 2 MiB: `vm_size()` used `read_to_string`, which reads a short file into a
stack probe buffer **before** it allocates; in a libniji process that allocation is the test thread's first, and the heap
maps 2 MiB of address space for it **after** statm was read. In a glibc process it maps none, so the margin held.

Fix: `vm_size()` reads statm into a stack buffer (never allocates); the gap test takes the figure after its layout
(pins included), right before `prlimit`. The assertion is unchanged; its failure message now carries the errno.

## Pass counts (20 each)

| tree | host | glibc process native | glibc process interposed | libniji process (pass 2) |
|---|---|---|---|---|
| next `43987d4dc` | Zen 1 (keke) | 20/20 | 20/20 | 0/20 unpinned, 0/20 cpu 0, 0/20 node 0, 0/20 node 1 |
| next (compat19 tree) | Skylake-SP (mei) | 60/60 (compat19-kpages) | - | FAIL in pass 2 of xtest4 and xtest5 (pass 1 PASS) |
| fix `d02fbab84` | Zen 1 (keke) | see report | see report | see report |

## Not done (budget)

* The 1509aeb3 run and the bisect: not needed to explain the failure (the mechanism is measured), but not measured:
  whether the heap's 2 MiB first-allocation mapping in a new thread is new since HEAP-10. HEAP-10's own `xtask test`
  stopped at the products step (WINDOW-BIN-BYTES) and its 20/20 runs and plants were `cargo test` (glibc processes),
  so pass 2 never ran this test on its branch.
* Pass 2 on the Skylake-SP host for the fix (no role there); the cause is host-independent.

## CI selection

`cargo xtask affected --run` runs pass 2 for niji-boundary targets only when the change touches mem, rt, kernel, abi or
boundary (`affected.rs`, `interpose`); a branch touching none of them (VALGRIND-LDSO-SYMBOLS-2, COMPAT19-LDSO-COST) never
runs this test in pass 2 in CI. See the report for HEAP-10's own CI landing.

## Next step

Land the fix. Then (coordinator's open item) heap_meta_products NUMA sensitivity: reported separately, not touched here.

Full report: handoff/HEAP-GAP-TEST-KEKE-report.md.
