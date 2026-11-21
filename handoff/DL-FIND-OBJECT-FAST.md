# DL-FIND-OBJECT-FAST: _dl_find_object and dladdr through an address index

**State.** Branch `task/dl-find-object-fast` from next `d5764940d`; role keke:~/w/n4 (synced at the WIP test
commit). Leg 1 wrote the code, the tests and the per-call spec; NOTHING is measured or gated yet.

**Design (in force; record still to write as `records/pending/DL-FIND-OBJECT-FAST-1.toml`, D-NEW, series D-27).**
`crates/boundary/src/ld/index.rs` (module doc says it all): one entry per `PT_LOAD` segment
`[seg lo, seg hi, object lo, object hi, PT_GNU_EH_FRAME, index]`, sorted by segment start, binary search.
- Start-time objects: immutable index built by the first reader/`dlopen` in a private mmap, published by CAS
  (losers munmap theirs). Nothing at start-up.
- After the first `dlopen` (`readers::publish` -> `index::added`; `readers::unpublish` -> `index::removed`, both
  under the loader lock): a two-copy latch (`SEQ` odd -> readers on copy 1 while copy 0 is rewritten; even -> copy
  0 while copy 1 is rewritten); readers validate `SEQ` after an Acquire fence, 8 tries then the walk. Citation:
  Boehm, MSPC 2012 (seqlocks and the memory model).
- Flags per region: `OVERLAP` (segments of two objects overlap: `_dl_find_object` walks), `NESTED` (object ranges
  overlap: `dladdr` walks the chain). dladdr verifies the hit against the node's range (`View::range`), else walks.
- Growth: a copy moves to a new mapping 2x; the old one is leaked (stale readers), bounded 2x per copy. mmap
  failure -> `DISABLED`, the walks answer (as before).
- Old walks kept as cold fallbacks: `readers::find_walk`, `View::find` in dladdr.

**Files.** index.rs (new), ld.rs (`mod index`), readers.rs (`start_count`, `live`, `headers` pub(super), hooks,
`find_object` via index), dlopen/dladdr.rs (index first). Test `crates/ld/tests/dl_find_object_index.rs`
(map: equivalence with program headers at every segment edge/gap, before dlopen, after dlopen, after dlclose
(closed object never found: checks 41/42), after reopen; churn: 3 readers x 3000 open/close cycles, exact fields).
Per-call spec `validation/percall/dl_objects.c` + corpus builder `validation/percall/dl_objects.sh` (needs
`PC_DL_DIR`). clippy -p niji-boundary clean (job `dl-find-object-fast-clippy1`).

**Running at refresh:** keke job `dl-find-object-fast-t1` (`cargo xtask so` then the new test, dlclose,
dlopen_concurrent). The spec commit is NOT yet synced to the role.

**Exact next steps.**
1. `jobwait dl-find-object-fast-t1`; fix what fails (read `~/logs/dl-find-object-fast-t1.log`, grep `check [0-9]+`).
2. Counts: on keke build the corpus (`sh validation/percall/dl_objects.sh ~/work/dl-find-object-fast/q 600`, a job)
   then `PC_DL_DIR=... bash validation/wl_percall.sh --spec validation/percall/dl_objects.c .` under jobrun
   (`count` kind) for base/head/glibc. Then the C++ throw measurement (brief step 3: a percall case or a small
   program, 10 frames, 4 and 104 libraries; libgcc's unwinder calls `_dl_find_object`).
3. Plants in `handoff/DL-FIND-OBJECT-FAST-plants.txt` (`cargo xtask plants`): (a) stale: `index::removed` body
   emptied -> test `map` check 41/42; (b) torn: drop the writer's odd `SEQ.store(s.wrapping_add(1), Relaxed)` in
   `update_with` -> churn check 71 (probabilistic window: if it survives, add a deterministic variant, e.g. skip
   the reader's validation AND the odd store); (c) OVERLAP flag ignored in `search`; (d) merge of a same-object
   segment dropped.
4. Record (reserve no id; D-NEW pending), gate (`cargo xtask affected --run`), start-up checks (`cargo xtask test`,
   NIJI_FULL_PROFILE so + hot_text_products; loader is start-up code: layout-bins diff vs merge base), precheck,
   report json, handback-check.

**Not to redo.** Reading readers.rs/dlopen.rs View/dladdr.rs (summarised above); bench infra survey: cbench is
static-only for libniji, so the per-call rows use wl_percall (dynamic by default, `WL_LINK`).
