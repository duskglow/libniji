# UNWIND-EXIT-COST (handoff; leg 1 = profile and design, no code change yet)

State: branch `task/unwind-exit-cost` (from origin/next 0495c4684), role keke:~/w/n2 (Zen 1 host, 60 vCPU),
products built there by job `unwind-exit-cost-so0` (rc 0) on the unchanged head. Scratch (probes, scripts):
`~/w/.scratch/UNWIND-EXIT-COST/` locally, installed on keke in `~/work/unwind-exit-cost/`; probe builds and
outputs in keke `/dev/shm/claude/unwind-exit-cost/base0/`.

## Baseline (job `unwind-exit-cost-base0`, keke, user instructions per operation, N=401 minus N=1)
`measure.sh <role> <outdir> <tag>` prints: per_op, returning-thread cost, delta (= the exit's own cost).

| row | reference delta | libniji delta |
|---|---|---|
| pthread_exit depth 0 (plain C, no -fexceptions: no unwinder loaded) | 8,612 | 57,115 |
| pthread_exit depth 10 | 21,112 | 97,321 |
| pthread_exit depth 10, pthread_cleanup_push per frame | 79,011 | 124,468 |
| pthread_cancel depth 3 (in read) | 13,851 | 70,106 |
| C++ exit through 10 dtor frames + catch/rethrow (libgcc_s loaded) | 76,958 | 96,125 |
| C++ throw through 10 frames, caught (no thread) | 86,446 | 98,986 |
| static pthread_exit depth 3 (plain C) | 17,381 | 129 |
A returning thread itself costs 2,274 (reference) vs 44,091 (libniji): THREAD-CREATE-COST's area, not this story.

## Profile split (exact: gdb single-step of the 2nd thread's pthread_exit, `step.py`, c.niji mode 1 depth 0)
76,775 instructions in the exiting thread. `thread::finish` 19,539 (thread teardown, not the walker). The rest is
D-22.23's `backtrace::handler_on_stack` (libniji's own CFI walk, run because a plain C program has no
`_Unwind_ForcedUnwind`), ~53k for ~11 frames (~4.8k/frame):
`Object::fde_for` 10,787 (binary search of the eh_frame_hdr table, through generic `at()`+`pointer()` per row);
`Cursor::value` 8,355 (the table rows' pointer decode); `unwind::run` 7,518; `niji_machine_move_avx2` 7,268 +
`conv::internal_move` 2,219 + `hot::mem::internal_large` 1,440 + memmove 416 (the "6.3k move": copies, see below);
`step` 3,087; `Object::cie` 2,763; `handler_above` 2,293; `Fde::parse` 1,232; `OwnMemory::word` 1,072;
search closure 1,056; `Object::fde` 990; `unwind::object` 888; `row_at` 738; `handler_on_stack` 568;
`holds_pc` 504; dlsym (the failing `_Unwind_ForcedUnwind` lookup, ruling #6904) ~2.5k in all.
WARNING: perf sampling on this Zen 1 host attributes `instructions:u` samples by time, not count (it put 48% in
ld.so's fill/dlsym; the exact count there is ~2.5k). Use step.py or perf stat differences, never perf %.

## Findings for the brief's candidates
* (a) "per-object index, binary search": ALREADY how `Object::search` works (PT_GNU_EH_FRAME table, binary
  search; linear scan only without a table). The cost is constant factors per probe, not a linear scan.
* (b) "skip the handler search when no frame can have a handler": in the no-unwinder case the walk IS the
  handler search; skipping it needs to know no loaded object has a personality FDE (outside libc's own guards),
  i.e. a one-time scan of every object's .eh_frame (libc's is 260 KB, ~5k records) plus invalidation on
  dlopen/dlclose. Not measured yet; leg 2 should estimate it (count records) and report, likely rejected.
* The copies: `run()` builds `let mut saved = [*row; REMEMBER_DEPTH]` (4 x ~296 B Row) on EVERY call, two calls
  per frame (CIE + FDE instructions); `walk_with`'s `locate` returns `(Object, Fde)` by value (Object ~240 B,
  copied per frame, `Some((*o, f))`).

## Exact next step (leg 2)
Edit `crates/rt/src/unwind.rs` only:
1. `run`: lazy remember stack: `let mut saved: Option<[Row; REMEMBER_DEPTH]> = None;` filled on first 0x0a
   (`get_or_insert([*row; REMEMBER_DEPTH])`), 0x0b reads it (`saved.as_ref()?.get(depth)`).
2. `walk_with`: generic over `O: core::borrow::Borrow<Object<'a>>`; `walk` passes `&Object`; `handler_above`
   an enum (listed reference / found owned) so listed objects are not copied.
3. `Object::search`: fast path for `table_enc == 0x3b` (datarel|sdata4): take the table slice once
   (`self.bytes(table)`, check `count * 8` fits), binary search on i32 LE pairs; the generic path stays for
   other encodings. Bounds via slice `get` (loaded data is untrusted).
Then: `cargo xtask so` on keke (~10 min), `measure.sh` + `step.py` again (table before/after), tests
`cargo test -p niji-rt --test unwind` + `forced_unwind_products`, plants in `handoff/UNWIND-EXIT-COST-plants.txt`
(brief step 5: e.g. the search fast path returning the wrong row (stale/wrong range), the lazy stack restoring
the wrong row; a "shortcut taken with a cleanup frame present" plant only if (b) lands), clippy, static, gate,
precheck. Bounds: grep `validation/`/`benchmarks/` for a pthread_exit row; none was found in leg 1 (`grep -rln
pthread_exit benchmarks validation docs/benchmarks.md` hit only a bincompat exports list).

## Decisions / rulings in force
D-22.19, D-22.21, D-22.23 (the walk), ruling #6904 (lookup stays uncached). No new records yet.

## Jobs so far (keke)
unwind-exit-cost-so0 (build, rc 0), unwind-exit-cost-base0 (baseline table), unwind-exit-cost-prof0 (perf,
misleading: see WARNING).
