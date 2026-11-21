# DL-PROCESS-OBJECTS-2: part 2 of 3, the loader's object tables grow (audit HL-3 without TLS)

**State (leg 2 ended by a context refresh).** Branch `task/dl-process-objects-2` from next `ef7bc6fd3`; role
mei:~/w/s3 (synced at `ee91fc83c`'s tree). Part 2 is written, lint-clean as of leg 2's first lint, and its loader
tests pass (job `dl-process-objects-2-t3`, before the inline fix). Part 3 (TLS, `MODULES_MAX`) is NOT started
and stays for a later story. Design: handoff/DL-PROCESS-OBJECTS.md sections 3-7 (not to redo).

**Done in leg 2 (do not redo).**
- Fixed `dlsym::handle_index` (a handle in a later node generation was refused: `dlclose` failed at the 510th
  pair inside a walk); `dlclose.rs` room test check 60 now expects all 600 pairs to succeed (tables grow).
- New test `dlopen_process_objects_past_the_old_object_tables` (2000 objects: 1500 top/base pairs, each base
  `libbase<k>.so` patched into the copy's soname and its top's DT_NEEDED, plus a hub with 600 deps). Its expected
  lines are the **reference's own output** for the same program (probe `~/work/DL-PROCESS-OBJECTS-2/ref_many.py`,
  job `dl-process-objects-2-ref2`: identical head lines; exit order = load order, each top before its base).
- OOM sweep stops after two completed runs in a row: 11 limits instead of 48; the whole `dlopen_process_objects`
  binary (3 tests) took 40.6 s in t3, against 474 s for the OOM test alone in part 1's gate.
- Pending record `records/pending/DL-PROCESS-OBJECTS-2-1.toml` (D-NEW, series D-27; body one line).
- Plants file `handoff/DL-PROCESS-OBJECTS-2-plants.txt` (15 plants). The first run (label
  `dl-process-objects-2-plants`, SUPERSEDED: stopped after the inline fix changed reader code) had CAUGHT
  nodes-cap, recs-cap, readers-cap, fin-cap (~3 min product rebuild per plant, 2 copies).
- **Counts** (`~/work/DL-PROCESS-OBJECTS-2/count.sh`: count.c, user instructions/call, min of 5x1000, 100 and 600
  libraries; next's products built in `/mnt/zram/claude/DL-PROCESS-OBJECTS-2/base-src`, job
  `dl-process-objects-2-base`). Before the fix (job `-counts2`), N=100 (104 objects): dladdr own 1002 -> 1431,
  dladdr last 2441 -> 5707, find own 607 -> 607, find last 18464 -> 18872, phdr walk 16021 -> 16020. The fix
  (commit `ee91fc83c`): `Gens::at_in` inline gen-0 fast path + `#[inline(never)] at_past`; `View::node_of`
  (no zero test within CAP) for `field`/`next`; `View::find` gen-0 decode without the remainder test, later
  generations via `#[inline(never)] index_past`. Job `dl-process-objects-2-t4` (clippy workspace, static,
  records check, `xtask so`, count.sh, the three loader test binaries) was started on that tree: its counts
  are the after numbers. Reference (glibc) for context: dladdr own 238, last 779 (N=100); find last 207.

**t4 result (GREEN, rc 0, on `ee91fc83c`'s tree):** workspace clippy, static, records check, `xtask so`, the
three loader test binaries all pass. Counts N=100 (104 objects) next -> branch: dladdr own 1002 -> **922**,
dladdr last 2441 -> **3356** (+915, ~9 per walk step), find own 607 -> 607, find last 18464 -> **18872**
(+408, ~4 per object), phdr walk 16021 -> **16121** (+100, ~1 per object). N=600 branch: dladdr own 922, last
29419, find own 607, last 111631, phdr walk 95896 (next: "cannot load more objects" at libc510).

**Exact next steps.**
1. Close the remaining within-512 per-step costs (the brief: within 512, no change). Candidates: `View::find`'s
   loop `for _ in 0..=u32::MAX` (was `0..CAP`) and its decode (`next >= first && off < CAP*size` vs the old
   `checked_sub`/`checked_div` + `i >= CAP`); `View::range` = two `field` calls, each now with `node_of`'s
   `i < CAP` test (old: none); `_dl_find_object`/`dl_iterate_phdr` read `readers::slot(i)` (`TABLE.get(i)` then
   `reader_entry`), and `(base..len).filter_map(slot)` replaced a slice iteration: give `readers` a gen-0 slice
   loop (`TABLE.get(base..len.min(CAP))`, then the later generations in a cold loop). Re-run t4.sh (it reruns
   counts; job label `dl-process-objects-2-t5`). If a residue of a compare or two per step stays, say so in the
   report with the numbers (the design's stated cost is one compare past 512; ask RULING if more remains).
2. Start plants on the final head: `cargo xtask plants handoff/DL-PROCESS-OBJECTS-2-plants.txt --dir
   /mnt/zram/claude/DL-PROCESS-OBJECTS-2/plants-2 --timeout 600 --copies auto` under jobrun label
   `dl-process-objects-2-plants2` (the role is BUSY for sync while it runs: sync first).
3. Gate (direct tier) `dl-process-objects-2-gate`, then `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`; layout-bins diff vs the merge base.
4. precheck, report json, handback-check. Wall deadline 12:48Z: if ~45 min remain with only jobs left, hand back
   PENDING JOBS.

**For the report.** RULING NEEDED: freed slots are reused only from the top (`load_set` puts a new set at
`l.n`; `reclaim` lowers the top past freed slots only), so a process that closes objects out of order (FIFO
plugin churn) leaves holes. Before part 2 such a process failed at 512 slots ("cannot load more objects"); now
the tables grow by one node and its records per hole and every `dlclose`/`live_set` walks up to the top.
Recommend a follow-up story: reuse a run of freed slots below the top (first fit for the set's size).
The 2000-object test asserts only `reused >= 2` (pair 999's top slots). INFORMATIONAL: a program linked with
`-Wl,-rpath,<products>` naming `libc.so.6` by soname did not find it at start ("cannot open shared object
file"); the tests (and count.sh) name libc by path with its DT_SONAME made inert, so it was not chased.
INFORMATIONAL: libniji's `_dl_find_object` is linear in the objects (18464 instructions for the last of 104;
the reference 207) and `dladdr` 4x the reference; separate performance stories.
