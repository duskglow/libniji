# HEAP-ADDRESS-SPACE-8 report

Branch `task/heap-address-space-8`; merge of origin/next `3f2a84520` (`d7667973f`), plants file (`afeb37b5e`),
handoff and report commits on top. No code change in this run. Host class: Skylake-SP (the role on mei).

## Premise

Run -7 ended red on `init_dynamic_startup_only_program_stays_within_its_page_fault_bound` (90 > 89) against base
`de4cdfc8d`; the brief asked for the cause, with the 256-aligned `.got` of `entry.S` as the lead, measured on next as
it is (the STARTUP-GOT-PAGE chain closed without landing, #6916).

## Finding (measured, fast profile, warm-up + 5 runs, every run identical)

| tree | dynamic true | static true | static locale |
|---|---|---|---|
| head (merged) | 70 | 11 | 43 |
| origin/next `3f2a84520` | 71 | 11 | 43 |
| head, `.got` aligned 8 (variant) | 70 | 11 | 43 |

`DYN_TRUE_BOUND` on next is now [73, 67]. The +1 of run -7 does not reproduce on today's next: the head is one
fault below next, and the `.got` alignment does not move the dynamic count (lead ruled out; no GOT/RELRO change made).

Layout bins vs next: bin 0 +38 bytes (60694 -> 60732; refill +32, runtime::start +44, stdio::parts_first_use +45,
new_slot +5, first_cache -50, take_held -5, KernelPages::map 728 replaced by map_room 455); 104 sections (105).
hot-text.signatures: only the map -> map_room rename. Growth code stays in `#[cold] #[inline(never)] grow_segment`.

## Run -8 change: the 256-aligned `.got` dropped (`10e62ddd2`)

Coordinator note 11:53Z (rulings #6972-#6977): drop the `.got` align-256 unless measured needed. Measured: the variant
without it (`heap-address-space-8-mnogot`) has STARTUP-DATA-WINDOWS "7 written start-up data pages (rw-budget-fast 7)"
ok, and the same faults as the head with it (dynamic 70, static 11 / 43). Dropped: entry.S and xtask/src/so.rs are
again as on the merge base; `records/pending/HEAP-ADDRESS-SPACE-3-2.toml` removed; run 3's got-unaligned plant gone.
The dynamic start-up fault, recorded as a cost per the note: -1 (head 70, next 71), not +1.

## cbench before (next `3f2a84520`) / after (head `10e62ddd2`), Skylake-SP, `--compare --counters --runs 3`

cbench_malloc, libniji instructions per call: no row slower. malloc_free_16..1024 122.2 -> 122.1, lifo_batch64
8342.7 -> 8339.2, realloc_grow_x2 1748.5 -> 1747.6, realloc_grow_16step 5723.3 -> 5720.7, realloc_null_24 145.2 ->
145.1, calloc_free_4096 178.1 -> 178.0; calloc_free_64 172.2, realloc_within_usable 194.2, realloc_moving 428.4,
heap_mix_real 148.6 unchanged (glibc 147.1 / 11341.7 / 2071.0 / 10856.4 / ... : every row below glibc but
realloc_within_usable 194.2 vs 195.2, still below).

cbench_startup, first run: startup_instr_c 2161 -> 2169 (+8), startup_instr_setloc 61613 -> 61634 (+21),
startup_instr_printf 65227 -> 65247 (+20), startup_instr_wctype 62091 -> 62105 (+14), startup_kinstr_c 135536 ->
143150 (+7614, kernel-mode instructions). Two repeat runs on the same built products (`-cbench2-before/-after`,
`--no-build`): startup_kinstr_c 135501 -> 143438 (+7937) and 135904 -> 142908 (+7004); startup_instr_c +8 both;
setloc +6 / +17, printf +12 / +11, wctype +28 / +26. **So cbench_startup is slower after, reproducibly: kernel-mode
instructions of the C start-up +5.2..5.9%, user-mode +6..+28. Cause NOT FOUND in this run** (budget/context
checkpoint). Not yet known whether startup_kinstr_c's program calls malloc at all: if it does, candidates are the
first reservation's mappings (map_room: over-map 32 MiB + head, two trims; was map + trims) and its first touches;
if it does not, the release-profile layout of start-up text/data (+38 bytes in bin 0 fast; release not diffed).
User-mode candidates: the first reservation's arithmetic, the `mapped`/`seg_mapped` stores, the FIRST_LARGE clamp,
runtime::start +44 bytes. Never measured worse is not a claim this branch can make yet: it must not land as is.

## Jobs

- `heap-address-space-8-s0` (step 0, inherited head): rc 0. `-mhead`, `-mbase`, `-mnogot`: the fault measurements.
- Head `afeb37b5e` (superseded by the drop): `-lint` rc 0, `-tests` rc 0, `-plants` 13/13 CAUGHT; `-xtest` and
  `-fullhot` stopped (superseded).
- Final code head `10e62ddd2`: `-lint2` rc 0; `-fullhot2` rc 0; `-plants2` (12 plants); `-xtest3`;
  `-cbench-before`/`-after`, `-cbench2-before`/`-after`.
- SUPERSEDED on `10e62ddd2`: `-tests2` and `-xtest2` (rc 101 / 1): every failure is hot_text_products
  "target/niji/.hot-lists: No such file or directory": a `cargo xtask so` of mine (`-tests3`, started too early by a
  driver whose wait on a not-yet-existing label returned at once, then stopped) ran in the role beside them. Not the
  code; `-xtest3` reruns it alone.
- `cargo xtask affected` escalates to FULL (entry.S `_start`, xtask): FULL: the train's.

## INFORMATIONAL

- DYN_TRUE_BOUND [73, 67]: the head reads 70 fast on Skylake-SP (next 71); a Zen 1 count would show whether the
  fast bound can tighten.
- `kill-label` on a gated `cargo xtask test` ended its slots wrapper but left `xtask test` and nextest running
  (systemd scope); `kill-mine` ended them. `jobwait` on a label with no row returns 4 at once.
- Run -7's note stands: no test runs `Pages::map` to success under an address-space limit (next's own code).

## Next

handoff/HEAP-ADDRESS-SPACE.md "Exact next step" (HEAP-SPILL-COLD gets the findutils RLIMIT_AS case; this lands as
a partial fix for #21).
