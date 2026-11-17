# SEM-NAMED-GROWTH: no limit on named semaphores open in a process (audit HL-7)

## State

- Branch `task/sem-named-growth`, cut from `origin/next` at `59c31ec19`. Code head `4efeda703`; the hand-back
  commit with this note and `handoff/reports/SEM-NAMED-GROWTH.json` sits on top.
- Done: the 256-entry `NAMED` table in `crates/boundary/src/sync.rs` (`sem_open` returned `EMFILE` for the 257th
  name although no descriptor limit was reached) now grows with use.

## Design (owner ruling 2026-11-17, PH14-OWNER-RULINGS "Hardcoded-limits audit table"; ruling #5594)

- A 16-entry static area (`NAMED`, 3 words per entry: (dev, ino) key, address, open count; 384 bytes instead of
  8 KiB) serves the common case.
- When every entry is in use, `named_grow` takes a chunk with twice the entries of the one before it (the static
  area counts as the first: 32, 64, 128, ...) from fallible allocation (`niji::mem::TryVec::resize`, so the global
  allocator, `ENOMEM` on failure), links it from `NAMED_MORE` (newest first) and leaks it: never moved, never
  freed. Closed entries are reused, so the table stays at the most names ever open at once (24 bytes each).
- Every read and write is under `NAMED_LOCK` (as before); chunk words are reached through `tb::words64` (the
  library's-own-memory contract). No new `unsafe`.
- `EMFILE` is no longer produced by the table; the only new failure is `ENOMEM` when a chunk cannot be allocated.
- A failed growth leaves the file `O_CREAT` made (as any later failure in `sem_open` did before).

## Tests

- `conformance/c/s22_sem_named_many.c` (clause s22_7_c_5, run by `s22_7_c_5` in
  `crates/core/tests/s22_threads_clauses.rs`): 2000 names open at once, each usable with its own value and
  address, the same address when opened again; all closed, all 2000 opened again at once by name with their
  values, closed, unlinked, then `ENOENT`. Reference and libniji both run it.
- `crates/boundary/tests/sem_named_nomem.rs` (s22_7_c_5): fault injection fails the growth allocation: the first
  open that needs a chunk fails with `ENOMEM` after at least the 16 static entries; earlier names keep their
  semaphores; the refused name opens once memory is back; 1000 open/close cycles with every allocation failing
  all succeed (closed entries are reused).

## Results (mei, role s4)

- Before (base `59c31ec19`, products built in a RAM copy, job `sem-named-growth-before`, evidence only, rc 1 by
  design): reference run passes 2000; libniji `s22_sem_named_many: step 1 failed at 256 (errno 24)` (EMFILE).
- After (head): gate `sem-named-growth-gate1` rc 0: 6 suites ok, 1424 tests passed, 0 failed, 100 ignored;
  19 conformance programs passed, `s22_sem_named_many` included (reference and libniji, 2000 at once).
- Plants `sem-named-growth-plants1` (`cargo xtask plants`): 5 of 5 CAUGHT, 0 WARN: cap restored (conformance and
  nomem test), growth error as EMFILE, chunk chain lost, closed entries not reused.
- Start-up counts (`cbench_startup`, `--compare --counters`): see the report json `impact` and the hand-back
  (jobs `sem-named-growth-counts2` head, `sem-named-growth-counts2-base` base).
- clippy and static clean on the code head (`sem-named-growth-check2` clippy part, `sem-named-growth-check3`).

## Not done / open

- No decision record written: the design is the owner's ruling applied as the audit described; the chunk sizes
  (16 static, doubling) are a local detail recorded here. A record can be added if the coordinator wants one.
- The start-up checks of `cargo xtask test` / `hot_text_products` were not run: no start-up code changed; only
  `.bss` shrinks (8 KiB to 384 bytes) and the table is not touched before the first `sem_open`.
- Not to redo: the fixed cap is gone; no larger fixed cap anywhere.

## Next step

None for HL-7. Other audit rows (plan/hardcoded-limits.md) are their own stories.
