# THREAD-CREATE-COST: thread create + join cost

**State:** branch `task/thread-create-cost-2` (THREAD-CREATE-COST-2 continues `task/thread-create-cost`), role
mei:~/w/s2 (Skylake-SP). Code commits (first run, THREAD-CREATE-COST, on the Zen 1 host):
- 0fdd6889e `validation/percall/thread_cost.c`: per-call cases create + return + join, create + pthread_exit + join,
  pthread_exit five frames down, eight threads alive at once (per = 8).
- 47af88677 `validation/wl_trace.py`: a trace whose thread ends (entry, exit sequence) prints its split instead of dying.
- 4b4c55da5 key scan bound (D-22.25 part 1): `KeyTable::span` is an O(1) high-water mark, not a scan of 4096 numbers.
- 6c59ccf45 loader TLS area of a reused mapping left to the loader (D-22.25 part 2); released-ID dtv slot written 0.
Second run (THREAD-CREATE-COST-2):
- 14e08e0f4 merge of origin/next 9139a21d9: the CI conflict in `crates/boundary/src/ld/tls.rs` `fill_static`. next
  now empties the slot of an on-demand block (DL-19) as well as of a released ID, a superset of 6c59ccf45's store:
  next's code is kept and the comment adds this branch's reason (a reused mapping's area is not cleared).
- the test `a_reused_mapping_gets_its_own_block_of_a_module_on_a_reused_id` (crates/boundary/tests/dl_tls_threads.rs):
  the plant on that store SURVIVED every existing test (job thread-create-cost-2-plants). With 6c59ccf45, the store
  is what keeps the following correct. A static-surplus module's slot (`tp + offset`) survives its thread in the reused
  mapping. The module is closed and an on-demand module takes its ID (DL-17). A thread in that mapping then reads the
  old block (tp + the old offset) instead of a block made at first access. The test builds that case, checks the ID
  really is reused, and runs it on the reference and on libniji.
Decision in force: D-22.25 (records/decisions/D-22/D-22.25.toml). Related: D-22.9, D-22.19, D-27 DL-16d, DL-17,
DL-19, ruling #6904 (unwinder lookup stays uncached), ruling #7877 (unwinder walk: follow-up UNWIND-EXIT-COST).

## Measurements, Skylake-SP (mei), merged head 14e08e0f4 vs origin/next 9139a21d9, wl_percall release products
instructions:u per thread, 5 runs; glibc = the reference (dynamic). Job thread-create-cost-2-counts2 on ea4a96b5b
(dynamic, the script's default link, then WL_LINK=static); the same as the earlier counts on 14e08e0f4 but join8.

| case | link | glibc | next | this branch |
|---|---|---|---|---|
| create + return + join | static | 2,285 | 36,022 | **1,205** |
| create + return + join | dynamic | 2,285 | 36,109 | **1,345** |
| create + pthread_exit + join | static | 9,558 | 36,109 | **1,292** |
| create + pthread_exit + join | dynamic | 9,558 | 65,798 | **31,034** |
| pthread_exit 5 frames down | static | 15,194 | 36,132 | **1,315** |
| pthread_exit 5 frames down | dynamic | 15,194 | 75,748 | **40,984** |
| 8 alive at once (per thread) | static | 2,441 | 27,190 | **1,066** |
| 8 alive at once (per thread) | dynamic | 2,441 | 27,292 | **1,201** |

sd 0 in every libniji row (join8 was 1.6-2.7 in the earlier run: 1,083 static, 1,211 dynamic). Every row is cheaper than next; the dynamic pthread_exit rows stay
above the reference (the unwinder walk, ruling #7877, story UNWIND-EXIT-COST). Zen 1 numbers of the first run (same
code before the merge): static join 36,014 -> 1,197, dynamic join 39,973 -> 1,332, dynamic exit 93,824 -> 55,183.

## Jobs (mei)
- thread-create-cost-2-lint0: step 0, clippy + static on the inherited head 7cbd370b4: rc 0.
- thread-create-cost-2-gate (head 14e08e0f4): clippy, static, direct tier `affected --run --base origin/next`
  (17 suites ok, 11,032 passed, 0 failed, 1,387 ignored; conformance 50 passed), the TLS programs the direct tier
  does not select (s27_dlopen_tls_dynamic, s27_dlopen_tls_surplus, s27_dynlink_clauses), `cargo xtask test`,
  NIJI_FULL_PROFILE=1 `xtask so` + `hot_text_products`.
- thread-create-cost-2-plants (head 14e08e0f4): 3 CAUGHT, empty-slot SURVIVED (led to the new test). Superseded.
- thread-create-cost-2-reusetest (head 74923763f): `cargo test -p niji-boundary --test dl_tls_threads`, 3 passed.
- thread-create-cost-2-gate2 (head 74923763f): red at clippy (indexing_slicing in the new test), fixed in cdd01ce70.
  thread-create-cost-2-plants2 stopped (superseded head).
- thread-create-cost-2-gate3 (final code head cdd01ce70): clippy, static, direct tier: 17 suites ok, 11,034
  passed, 0 failed, 1,387 ignored; conformance 50 passed.
- thread-create-cost-2-plants3 (cdd01ce70): 4 CAUGHT (span-zero, span-short, reuse-skips-block, empty-slot),
  0 SURVIVED, 0 EQUIVALENT, 0 WARN.
- Counts and the start-up checks first ran on 14e08e0f4 (gate, counts, counts-static). precheck could not judge
  them on the final head: gate/gate3 ran from a script file, so jobrun saw no role and wrote no tested-content
  record, and the counts predate the test. So on ea4a96b5b: thread-create-cost-2-gate4 (clippy, static, direct
  tier, `cargo xtask test`, NIJI_FULL_PROFILE=1 `xtask so` + `hot_text_products`) and thread-create-cost-2-counts2
  (dynamic then WL_LINK=static).

## Not done / next
- dynamic pthread_exit: the unwinder walk, story UNWIND-EXIT-COST (ruling #7877).
- Syscall split: kernel instructions only (first run's table); no strace split.
