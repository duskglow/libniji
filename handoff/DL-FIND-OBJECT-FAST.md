# DL-FIND-OBJECT-FAST: _dl_find_object and dladdr through an address index

**State.** Branch `task/dl-find-object-fast` from next `d5764940d`; role keke:~/w/n4. Code head `f92f73c40`
(record `b4cba586a`, then handoff/report commits). Decision: `records/pending/DL-FIND-OBJECT-FAST-1.toml` (D-NEW,
series D-27; CI assigns the id).

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
`validation/percall/dl_objects.sh` 600 libs in ~/work/dl-find-object-fast/q).** glibc / base d5764940d / head
b2a8a77 (job pc2; `_dl_find_object` code unchanged since):

| row | glibc | base | head |
|---|---|---|---|
| _dl_find_object last 104 | 173 | 18387 (-1: wrong) | 403 |
| _dl_find_object last 600 | 159 | 110124 (-1) | 471 |
| _dl_find_object first 104 | 51 | 639 | 402 |
| _dl_find_object last 4 | 89 | 873 | 332 |
| dladdr last 104 (b2a8a77) | 794 | 2300 | 723 |
| dladdr last 600 (b2a8a77) | 3274 | 23721 | 954 |
| dl_iterate_phdr walk 104 | 3798 | 15833 | 15833 (not in scope of the index) |

dladdr on b2a8a77 was slower than base on the `first` rows (1565 -> 1874) and with 4 objects; f92f73c40 fixes
that (walk for short chains, program first): final rows in job `dl-find-object-fast-pc3`.
C++ throw, 10 frames, instr/throw (job throw1; head in `dl-find-object-fast-throw2`): glibc prog/lib 8 objects
29609/31399, 104 objects 29610/31950; base 44929/65705, 104 objects 44929/**437417**.

**Proof.** Tests `crates/ld/tests/dl_find_object_index.rs` (map: span oracle at every segment edge and gap,
before/after dlopen, after dlclose (checks 41/42), reopen; churn: 3 readers x 3000 open/close, check 71).
Plants `handoff/DL-FIND-OBJECT-FAST-plants.txt`: 5/5 CAUGHT on b2a8a77 (job plants: stale-after-dlclose,
torn-writer-no-odd, torn-reader-no-validate, search-edge, added-missing); rerun on the final head: plants2.

**Jobs on the final head (keke).** `~/work/dl-find-object-fast/seq2.sh`: lint (workspace clippy + static), gate2
(direct tier, 56 tests), xtest (`cargo xtask test`), hot (NIJI_FULL_PROFILE so + hot_text_products).
`par2.sh`: plants2, pc3, throw2. Superseded: t2 (wrong test name, killed), gate (b2a8a77, killed for the dladdr
change), plants (b2a8a77), pc1/pc2 (older heads).

**Open / next step (leg 3).** lint is GREEN (workspace clippy + static, code head f92f73c40). 1. `jobwait` the
other six labels (gate2, xtest, hot, plants2, pc3, throw2; seq2/par2 run them in order); fix a red. Then fill the
final numbers (pc3 dladdr rows: check no row slower than base; throw2 head row) into the table above and into
`handoff/reports/DL-FIND-OBJECT-FAST.json` (gate, impact), commit, push, sync, `cargo xtask precheck --jobs
<the seven labels> --head <sha>` on the role, `ops handback-check`, hand back COMPLETE (target not reached) or
PENDING JOBS if ~45 min before the 22:06Z deadline. 2. Target not reached: `_dl_find_object` is
~400 vs the reference's 51-197: profile the constant part (read section counters, `region()` slice, bounds-checked
`word()`, `node_address`) with `validation/wl_trace.sh`; a first-object fast path is one option. dladdr `first` rows
(program) stay ~5x the reference (symbol search, not the object search). `dl_iterate_phdr` walk ~4x: separate story.
3. Start-up layout: the index is built at the first reader, nothing on the start path; layout-bins diff not taken.

**Not to redo.** Reading readers.rs / dlopen.rs View / dladdr.rs; probe R-5; the bench survey (cbench is
static-only for libniji; per-call rows use wl_percall dynamic).
