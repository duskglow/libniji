# DL-FIND-OBJECT-FAST: _dl_find_object and dladdr through an address index

**State.** Branch `task/dl-find-object-fast` from next `d5764940d`; role keke:~/w/n4. Code head `f92f73c40`
(record `b4cba586a`, then handoff/report commits). Decision: `records/pending/DL-FIND-OBJECT-FAST-1.toml` (D-NEW,
series D-27; id D-27.75 reserved in leg 3 and named in `index.rs`'s module doc, a comment-only commit after the jobs).

**Design (in force).** `crates/boundary/src/ld/index.rs` (module doc): one entry per `PT_LOAD` segment, sorted by
start, binary search for the last entry at or below the address, kept when its object's whole range holds it.
Start-time index built by the first reader (CAS publish); after the first `dlopen` two copies behind a sequence
latch (Boehm, MSPC 2012); `dlclose` removes entries at unpublish. `NESTED` -> walk. Mapping failure -> walks.
- `_dl_find_object` (dynamic) finds every pc in an object's span, gaps included: **probe R-5**
  (`tools/probe/dl/span/probe.c`, `spec/behaviour/dynlink.md`); base gave -1 past a segment's `p_memsz`. Static
  path unchanged (per segment).
- `dladdr`: chain walk while `readers::published() <= 12`, else program node first, then index (the walk is
  ~17 instr/node; the index cost ~a dozen nodes).

**Counts (instructions per call, dynamic, Zen 1 host; wl_percall spec `validation/percall/dl_objects.c`, corpus
`validation/percall/dl_objects.sh` 600 libs in ~/work/dl-find-object-fast/q).** glibc / base d5764940d / final
head b4cba586a (code f92f73c40), job `dl-find-object-fast-pc3` (28 rows):

| row | glibc | base | head |
|---|---|---|---|
| _dl_find_object last 104 | 173 | 18387 (-1: wrong) | 403 |
| _dl_find_object last 600 | 159 | 110124 (-1) | 471 |
| _dl_find_object middle 32 | 181 | 5715 (-1) | 373 |
| _dl_find_object first 104 | 51 | 639 | 402 |
| _dl_find_object last 4 | 89 | 873 | 332 |
| dladdr last 600 | 3274 | 23721 | 968 |
| dladdr last 104 | 794 | 2300 | 735 |
| dladdr middle 104 | 540 | 1382 | 735 |
| dladdr last 32 | 436 | 1004 | 703 |
| **dladdr first 4/32/104/600** (program) | 301 | 1565 (1578.5 at 4) | **1607 (1633.5): +42..+55, slower** |
| **dladdr last 4 / middle 4** | 1499 / 82820 | 949.5 / 121320.5 | **959.5 / 121344.5: +10 / +24, slower** |
| dl_iterate_phdr walk 4/32/104/600 | 298/1278/3798/21171 | 633/4889/15833/95170 | same as base (untouched) |

C++ throw through 10 frames, instr/throw (job throw2): glibc prog/lib 8 objects 29608/31400, 104 objects
29609/32406; base 44929/65705, 104 objects 44929/**437417**; head 36692/36370, 104 objects 38292/**37640**.

**Cost (fix-forward, not done: no wall time left for a rerun).** The dladdr rows of a program address (+42..+55,
+2.7..3.5%) and two 4-object rows (+10, +24) are slower than base. The walk path differs from base only by one
relaxed load (`readers::published()`) and a compare, so the excess is codegen (the shared `holds` closure /
the larger read-section closure); trace of the head call: job trace1 (1606 instr: `containing` symbol search 415,
`dladdr_entry` 20, `fill_start` 17). A base trace for comparison (trace2) was stopped in the queue. Fix-forward:
test `holds(0)` before anything else (both paths start there) and keep the walk closure as base wrote it, then
trace both revisions (`validation/wl_trace.sh --spec validation/percall/dl_objects.c <dir|rev> dladdr_first_4 dladdr`).

**Proof.** Tests `crates/ld/tests/dl_find_object_index.rs` (map: span oracle at every segment edge and gap,
before/after dlopen, after dlclose (checks 41/42), reopen; churn: 3 readers x 3000 open/close, check 71).
Plants `handoff/DL-FIND-OBJECT-FAST-plants.txt`: 5/5 CAUGHT on b2a8a77 (job plants: stale-after-dlclose,
torn-writer-no-odd, torn-reader-no-validate, search-edge, added-missing); final head: plants2 5/5 CAUGHT, no WARN.

**Jobs on the final head d91a4174d (keke, started 21:17Z, pending at hand-back).** `~/work/dl-find-object-fast/seq3.sh`:
lint3 (workspace clippy + static), gate3 (direct tier), xtest3 (`cargo xtask test`), hot3 (NIJI_FULL_PROFILE so +
hot_text_products). `par3.sh`: plants3, pc4, throw3. The numbers above are from the same code one commit earlier
(b4cba586a: lint, gate2 786 passed / 0 failed / peak 2.4G, plants2, pc3, throw2, all rc 0); d91a4174d changes
only `index.rs`'s module doc (the reserved id), so pc4/throw3 should repeat them exactly. Superseded: those five
(content check: index.rs changed after them), xtest (b4cba586a, stopped for the resync), t2 (wrong test name),
gate (b2a8a77), plants (b2a8a77), pc1/pc2 (older heads), trace1/trace2 (diagnostics).

**Coordinator note (DL-SLOT-REUSE #7379, 20:41Z): remove its +4 instr per dl_iterate_phdr call.** Not done: that
change is not on next yet (not in this branch, base d5764940d), so there is nothing here to remove; this branch
leaves dl_iterate_phdr unchanged against next before DL-SLOT-REUSE (pc3 rows above, base = head). Whoever merges
both takes the REORDERED test out of the per-call path.

**Next step.** 1. The dladdr fix-forward above (cost rows). 2. Target not reached: `_dl_find_object` is
~400 vs the reference's 51-197: profile the constant part (read section counters, `region()` slice, bounds-checked
`word()`, `node_address`) with `validation/wl_trace.sh`; a first-object fast path is one option. dladdr `first` rows
(program) stay ~5x the reference (symbol search, not the object search). `dl_iterate_phdr` walk ~4x: separate story.
3. Start-up layout: the index is built at the first reader, nothing on the start path; layout-bins diff not taken.

**Not to redo.** Reading readers.rs / dlopen.rs View / dladdr.rs; probe R-5; the bench survey (cbench is
static-only for libniji; per-call rows use wl_percall dynamic).
