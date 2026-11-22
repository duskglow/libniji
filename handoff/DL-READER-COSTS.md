# DL-READER-COSTS: the loader's per-call reader costs (run 1)

**State.** Branch `task/dl-reader-costs`, cut from next `ae9ea6879`; role keke:~/w/n2. Decision D-27.84
(`records/decisions/D-27/D-27.84.toml`). Plants `handoff/DL-READER-COSTS-plants.txt`. Report json
`handoff/reports/DL-READER-COSTS.json`.

**Done (brief items 1, 3-6 in part; item 2 not started).**
- Per-call rows: `validation/percall/dl_objects.c` gains `dlopen_new_N` (open + close of an object not loaded)
  and `dlopen_held_N` (open + close of one already open; libc.so.6 by name at 4 objects); corpus default 608.
- Fix-forward (5): `dladdr` tests the program node before the walk/index choice (`dlopen/dladdr.rs`).
- Fix-forward (6) and gap (3): `dl_iterate_phdr` per object: TLS flag in the entry's live word (3 = has
  `PT_TLS`, set at publication and by `fill_start`), fixed counters once per walk, vDSO only after entry 0
  (`readers.rs`). The REORDERED test stays (once per call).
- Gap (4): `niji_ld::elf::Symtab::holding` decodes only symbols whose range holds the address (`dladdr.rs`
  `containing`).
- Gap (1), part: `Env::read_with` parses only `LD_` environment entries (`entry.rs` `ld_entries`, `io.rs`
  `starts_with`): it measured and compared every entry at every `dlopen`.

**Counts** (instr/call, Zen 1 host, jobs `dl-reader-costs-pc0` (glibc, base) and `-pc1` (base vs head);
base = next `ae9ea6879` + the spec commit). glibc / base / head:

| row | 4 | 32 | 104 | 600 |
|---|---|---|---|---|
| walk | 298 / 637 / 388 | 1278 / 4893 / 2012 | 3798 / 15837 / 6188 | 21158 / 95174 / 38552 |
| dladdr first (program) | 301 / 1713 / 877 | 301 / 1687 / 864 | 301 / 1687 / 864 | 301 / 1687 / 864 |
| dladdr middle | 82820 / 126992 / 31659 | 361 / 718 / 619 | 540 / 748 / 649 | 1779 / 781 / 682 |
| dladdr last | 1499 / 994 / 600 | 435 / 716 / 617 | 794 / 748 / 649 | 3274 / 984 / 886 |
| dlopen held | 1523 / 20381 / 7539 | 1833 / 22663 / 9820 | 1833 / 22663 / 9820 | 1833 / 22727 / 9884 |
| dlopen new | 7722 / 85344 / 72586 | 21185 / 143186 / 130431 | 55689 / 292308 / 279553 | 293041 / 1357997 / 1345238 |
| _dl_find_object | unchanged (head = base on all 12 rows) |

No row slower. Fix-forward bases: (5) 1,565 before DL-FIND-OBJECT-FAST -> 864; (6) 16,030 before DL-SLOT-REUSE
(100 objects, other spec) -> 6,188 at 104.

**What remains against the reference, and why (attribution: `dl-reader-costs-trace0`, wl_trace of base).**
- `dlopen` new: ~72k at 4 objects vs 7.7k, and ~2,000 per loaded object vs ~480. Base trace (78k): ELF
  reading of the new file (`read_phdr`, `checked_add`, `get<u8>`, byte-wise field decode: ~30k), path
  comparisons against loaded objects (`niji_machine_equal`, `c_string`), `Env` auxv copy + CPUID features per
  call (~1k, cacheable: invariant). The per-loaded-object term is the next target (scope or name matching
  per object: trace `dlopen_new_104`).
- `dlopen` held: 9.8k vs 1.8k: the rest of `Env::read` (auxv copy, CPUID, cache mapping per call) and the
  name match; cache the invariant parts once, map the cache only for a bare name.
- `dladdr` program: 864 vs 301: the symbol scan is still linear; a per-object address-sorted symbol index
  (built at load or first use under the lock) makes it logarithmic (also libc: 31.7k).
- walk: ~60 vs ~36 per object: `call` through `niji_machine_call3`, the entry read and info build.
- Item (2), the name arena's reuse below the top: not started.

**Proof jobs on head `9b58edf33` (keke):** `dl-reader-costs-gate1` (static + direct tier),
`-plants1` (6 plants), `-xtest1` (`cargo xtask test`), `-hot1` (full-profile so + hot_text_products).
Results: see the hand-back / report json. Layout-bins diff against the merge base not taken (no start-path code:
`fill_start` runs at the first reader).

**Next step.** A follow-up for `dlopen` new-object cost (ELF read and per-loaded-object term), the arena
reuse (item 2), and the symbol index; trace with `validation/wl_trace.sh --spec validation/percall/dl_objects.c
. dlopen_new_104 dlopen` (needs `PC_DL_DIR`, corpus `sh validation/percall/dl_objects.sh <dir>`).

**Not to redo.** The base table above; traces of `dladdr_first_4`, `dl_iterate_phdr_walk_4`, `dlopen_held_4`,
`dlopen_new_4` (trace0).
