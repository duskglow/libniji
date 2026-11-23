# THREAD-CREATE-COST: thread create + join cost

**State:** branch `task/thread-create-cost`, role keke:~/w/n1 (Zen 1 host, EPYC 7551, 60 vCPU). Code commits:
- 0fdd6889e `validation/percall/thread_cost.c`: per-call cases create + return + join, create + pthread_exit + join,
  pthread_exit five frames down, eight threads alive at once (per = 8).
- 47af88677 `validation/wl_trace.py`: a trace whose thread ends (entry, exit sequence) prints its split instead of dying.
- 4b4c55da5 key scan bound (D-22.25 part 1): `KeyTable::span` is an O(1) high-water mark, not a scan of 4096 numbers.
- 6c59ccf45 loader TLS area of a reused mapping left to the loader (D-22.25 part 2); released-ID dtv slot written 0.
Decision in force: D-22.25 (reserved id; records/decisions/D-22/D-22.25.toml). Related: D-22.9, D-22.19, D-27 DL-16d,
ruling #6904 (unwinder lookup stays uncached).

## Attribution (method: gdb single-step, `validation/wl_trace.sh`, libniji only, release products, call 11)
Zen 1, origin/next code (rev 0fdd6889e = origin/next e3b0caf + the spec), one create + return + join:

| part | static | dynamic |
|---|---|---|
| pthread_create (creator) | 18,089, of which `KeyTable::span` scan 17,415 | 22,013: scan 17,415, loader-area zero fill 3,886 |
| pthread_join (creator) | 157 | 151 |
| new thread, `entry` to the exit system call | 17,639, of which the same scan 17,415 | (not traced: symbol in libc.so.6; perf difference agrees) |
| pthread_exit path (dynamic, thread_create_exit) | - | 71,413: scan 17,415; the unwinder walk (niji_rt::unwind: FDE search, LEB, a 6,352-instruction move) ~50k |

perf (`wl_percall`, instructions:u per thread, 5 runs, sd 0) agrees: 36,014 static, 39,973 dynamic.
The two scans: `take_mapping` clears the key area up to `span()`, and `run_destructors` bounds each round by
`span()`; `span` was `seq.iter().rposition(..)` over all 4096 numbers (no key created: the whole table, every time).
The dynamic extra: `take_mapping` cleared the whole loader area (64 KiB surplus) of a reused mapping, though
`tls_init` writes every block in full and the dtv.

## Measurements (Zen 1 host, wl_percall release products, per thread, 5 runs; glibc = the reference, dynamic)
Before = job thread-create-cost-basecount (rev 0fdd6889e, origin/next code); after = job thread-create-cost-counts
(head 6c59ccf45). instructions:u sd 0 in every libniji row.

| case | link | glibc | before | after |
|---|---|---|---|---|
| create + return + join | static | 2,291 | 36,014 | **1,197** |
| create + return + join | dynamic | 2,291 | 39,973 | **1,332** |
| create + pthread_exit + join | static | 9,584 | 36,101 | **1,284** |
| create + pthread_exit + join | dynamic | 9,584 | 93,824 | **55,183** |
| pthread_exit 5 frames down | static | 15,249 | 36,124 | **1,307** |
| pthread_exit 5 frames down | dynamic | 15,249 | 112,999 | **74,358** |
| 8 alive at once (per thread) | static | 2,446 | 27,172 | **1,064** |
| 8 alive at once (per thread) | dynamic | 2,446 | 29,218 | **1,193** |

Kernel instructions (instructions:k, advisory: sd 60-1,500): static join 57,073 -> 54,139, exit 54,720 -> 53,735,
deep 54,764 -> 54,941 (+178, inside its sd 236), join8 73,720 -> 73,605; dynamic join 55,094 -> 54,253, exit
103,028 -> 101,303, deep 103,102 -> 98,250, join8 78,582 -> 72,375 (glibc 63,768-84,229). Page faults per thread
unchanged (0-2, equal before and after). The 1,000-thread workload is these cases at n = 1,000 (serial) and the
join8 case (8 at a time).
The static pthread_exit is cheap because a static program without frames to unwind takes no unwinder walk
(THREAD-EXIT-NO-UNWINDER); the dynamic one walks libniji's own unwinder (above).

## Jobs (keke, head 6c59ccf45; later commits: handoff/report only)
- thread-create-cost-gate: clippy (workspace, all targets, all features), `cargo xtask static`, direct tier
  `affected --run --base origin/next`: 17 suites ok, 10,568 passed, 0 failed, 1,015 ignored; conformance 49 passed;
  then the start-up checks `cargo xtask test` and NIJI_FULL_PROFILE=1 `xtask so` + `hot_text_products`.
- thread-create-cost-plants rc 0: handoff/THREAD-CREATE-COST-plants.txt, 3 CAUGHT, 0 SURVIVED, 1 EQUIVALENT
  (released-slot: reason in the file), 0 WARN.
- thread-create-cost-counts rc 0 (after), thread-create-cost-basecount rc 0 (before), thread-create-cost-trace and
  thread-create-cost-probe2 (attribution; the unit tests passed there too).

## Not done / next
- Skylake-SP not measured in this run (role on keke only); the N100 numbers of SHSTK-SCOPE are inherited.
- pthread_exit (dynamic): the remaining gap is the unwinder walk itself (~50k of 71k), not lookup caching: a
  follow-up story (frame-table index per object, or skip the handler search when no frame has a handler).
- Syscall split: the strace -c probe printed no rows (my awk filter); kernel instructions are in the table.
