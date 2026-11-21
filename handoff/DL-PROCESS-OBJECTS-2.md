# DL-PROCESS-OBJECTS-2: part 2 of 3, the loader's object tables grow (audit HL-3 without TLS)

**State.** Branch `task/dl-process-objects-2` from next `ef7bc6fd3`; role mei:~/w/s3. Code head `9601fcc70`
(gated and planted tree); later commits change only handoff/ and the pending record's text. Part 2 is written,
gated and planted; the hand-back is INCOMPLETE only because `cargo xtask test` has one native-pass red whose cause
is not established (below).
Part 3 (TLS, `MODULES_MAX`) is NOT started and stays for a later story. Design: handoff/DL-PROCESS-OBJECTS.md
sections 3-7 (not to redo). Decision: `records/pending/DL-PROCESS-OBJECTS-2-1.toml` (D-NEW, series D-27).

**What part 2 does.** Link-map nodes, object and symbol records, the readers' table, `Finis` entries and the
lock-held lists grow by never-moved generations (`ld/table.rs` `Gens`/`Grown`, part 1's `Arena` arithmetic);
generation 0 is the old 512-entry array in place. Per-call scratch past 512 is reserved by the `dlopen` that grows
the count (`Loader::grow_to`, cold), so `dlclose` and the exit never allocate. u32 indices. "cannot load more
objects" is gone; a failed reservation is `<name>: cannot allocate memory` before anything is written.

**Proof (all on mei, Skylake-SP).**
- Loader tests: `dlopen_process_objects_past_the_old_object_tables` (2000 objects; its expected lines are the
  reference's own output for the same program, probe `~/work/DL-PROCESS-OBJECTS-2/ref_many.py`, job
  `dl-process-objects-2-ref2`), dlclose room test (600 pairs), init_order. Job `dl-process-objects-2-t6` (workspace
  clippy, static, records check, `xtask so`, counts, the three loader test binaries) green on `9601fcc70`'s tree.
- OOM test (brief item 3): the sweep stops after two completed runs in a row (11 limits, not 48); part 1's gate
  474 s for that test alone, now 20.9 s for the whole `dlopen_process_objects` binary (3 tests, t6).
- Counts (`~/work/DL-PROCESS-OBJECTS-2/count.sh`, user instructions per call, min of 5x1000; next built in
  `/mnt/zram/claude/DL-PROCESS-OBJECTS-2/base-src`). N=100 libraries (104 objects), next -> branch (glibc):
  dladdr own 1002 -> 928 (238), dladdr last 2441 -> 2239 (777), `_dl_find_object` own 607 -> 605 (45), last
  18464 -> 18462 (167), `dl_iterate_phdr` walk 16021 -> 16024 (3895; +3 per walk = the final generation compare).
  N=600 (604 objects; next refuses at libc510): dladdr last 23934, find last 111267, phdr walk 97058.
- How the within-512 residue was closed (leg 3): each lock-free walk runs generation 0 in its own loop and continues
  in one cold function (`View::find_past`, `readers::iterate_past`, `readers::find_past`); `dladdr`'s chain walk
  masks the checked index with `CAP - 1` because LLVM folds `k >= CAP` into a byte-offset test and otherwise keeps
  `node_of`'s `i < CAP` in every step (seen in our own ld.so's disassembly).
- Layout: `target/product/fast/layout-bins.tsv` equal to the merge base's except one LLVM hash suffix in a symbol
  name (same section, same bytes).
- Jobs on the final code tree: `dl-process-objects-2-plants2` (18 plants), `dl-process-objects-2-gate` (direct
  tier, 6 targets), `dl-process-objects-2-xtest` (`cargo xtask test`), `dl-process-objects-2-fullprof`
  (`NIJI_FULL_PROFILE=1` so + hot_text_products); chained by `~/work/DL-PROCESS-OBJECTS-2/chain.sh`.
  Results: plants 18/18 CAUGHT (view-node-addr by the 600 s timeout: a wrong decode loops the chain, the walk's
  guard is 2^32 steps); gate 747 passed, 0 failed, peak 2.9G; `cargo xtask test` 6980/6980 then 4853/4854, the
  failure is `niji-boundary::s03_memory_symbols kernel_pages_map_aligned_past_a_gap_without_an_aligned_range`
  in the native pass only (s03_memory_symbols.rs:1008: after `KernelPages::map`, the pinned gap cannot be mapped
  again: `back` 0). Heap code is untouched, but the native pass runs the test under libniji's ld.so, which this
  branch changes, so the cause is NOT ESTABLISHED. Branch: run plainly 0 of 100 fail; through the role's
  native-pass runner 100 of 100 fail (job `dl-process-objects-2-counts3`, `rep.sh`, products stable). One
  run under `strace -f -e mmap,munmap,prlimit64` PASSED; its trace from the limit to the check shows only
  `KernelPages::map`'s own calls (aligned tries, PROT_NONE page pins, one inside the pinned gap) and no loader
  mapping: a hint that the loader is not involved, NOT a proof. A next comparison was attempted (`rep2.sh`) but is INVALID: next's
  tree had no native copies, so its runner refused every run; withdrawn. fullprof green (hot_text_products 32 passed).

**Superseded.** `dl-process-objects-2-plants` (stopped: reader code changed), `-counts2`, `-t3`, `-t4`, `-t5`
(earlier trees); `-t6` (final tree, green, but no tested-content record; its counts were rerun as `-counts3`,
same numbers). Diagnostics, not proof: `-rep`, `-rep2` (rep2's next half invalid), `-rep3next` (did nothing).

**RULING NEEDED.** Freed slots are reused only from the top (`load_set` puts a new set at `l.n`; `reclaim` lowers
the top past freed slots only), so a process that closes objects out of order (FIFO plugin churn) leaves holes.
Before part 2 such a process failed at 512 slots; now the tables grow by one node and its records per hole and every
`dlclose`/`live_set` walks up to the top. Recommend a follow-up story: reuse a run of freed slots below the top
(first fit for the set's size). The 2000-object test asserts only `reused >= 2`.

**INFORMATIONAL.** (1) A program linked with `-Wl,-rpath,<products>` naming `libc.so.6` by soname did not find it
at start ("cannot open shared object file"); the tests and count.sh name libc by path with its DT_SONAME made
inert, so it was not chased. (2) libniji's `_dl_find_object` is linear in the objects (18462 instructions for the
last of 104; the reference 167) and `dladdr` about 3x the reference; separate performance stories.

**Next step.** Settle the `cargo xtask test` red: run that one test in the native pass on next (a tree where
`cargo xtask test` or a gate made the native copies, e.g. a role synced to next) N times and compare with this
branch's rate. Same rate on next: not this story (a heap/test defect for its own story), hand back COMPLETE. Only
on this branch: find what in the loader change maps into the pinned gap (e.g. the loader's state mapping made on a
first reader or `dlopen` call) and fix it. Then precheck, handback-check. Part 3 (TLS) is a new story.
