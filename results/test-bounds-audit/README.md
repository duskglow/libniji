# TEST-BOUNDS-AUDIT: which numeric test bounds measure what they claim

Investigation, 2026-11-23, on next 0495c4684. No bound, test or product changed. Every number below was measured in
this story unless it is marked **guess** or **from the record**. Raw data: `counts-haswell-ep.tsv`,
`counts-skylake-sp.tsv` (label, file system of the staged program, pad configuration, mode, lowest of 5 runs after a
warm-up, all 6 runs). Method scripts: `measure.py` (sweep) and `analyse.py` (summary, perf attribution).

## 1. Answer in brief

* **The red that started this is not jitter from the stories that went red. It is next's own state on CI's host
  class.** `init_data_clusters_hold_at_every_text_phase` (blocking) went red on three unrelated stories on 11-23 with
  the same numbers (`tool`: 58 at every pad, 59 at `cold 0x2000`). Unchanged next products reproduce exactly that on
  the Skylake-SP host: `locale` 56 → 57 and `tool` 58 → 59 at `cold 0x2000`, and no other change in 28 pad
  configurations. On the Haswell-EP host the same products move at **no** pad (16 cold pads in 4 KiB steps over a
  whole 64 KiB window, plus the data, lbss and relro pads). The extra fault is a **text page outside the hot group**:
  under perf, the locale path executes cold text near 0x72d000 and 0x756000 on that host, and at `cold 0x2000` one
  of those functions crosses the 64 KiB window boundary at 0x730000.
* **An unrelated library change flips that verdict.** A one-line change in `glob` (libc.a and libc.so.6 change;
  start-up code does not) makes the Skylake-SP host read 58 at every pad again: the test would pass. So whether it is
  red on CI depends on where unrelated bytes put a cold function. It is **brittle**, and the cause it reports is real:
  a start-up path the build's trace does not cover.
* **Why the host classes differ: the locale data source, not the CPU and not the file system.** The Skylake-SP host
  has no `locale-archive` (it has per-locale directories); the Haswell-EP host has the archive. The same static probe
  reads `locale`/`tool` 43/45 on Haswell-EP and 56/58 on Skylake-SP; the reference reads 39/39 and 53/53 (+14 for
  both libraries). Loading from the per-locale directory runs code the hot group does not hold.
* **The file system under /tmp moves nothing on this head:** 0 of 1,156 (Haswell-EP, four heads) and 0 of 588 (Skylake-SP,
  two heads) count cells differ between ext4 and tmpfs staging, the dynamic probe included (`DYN_TRUE_BOUND`'s comment records an
  ext4/tmpfs split that D-27 `map_view` has since removed).
* **Changes that do not touch libc do not reach the counts at all:** a docs and test-only change gives byte-identical
  products; a libm change (`jn`) changes only `libm.a`. Every count identical, as it must be.
* **The runtime fault bounds are stale on both measured host classes and fire on every run** (non-blocking since
  D-X.136): fast `tool` 45 and 58 against 44, `float` 4 against 3, the dynamic start 65 against 73 (the "more than 4
  under" side). A report that fires on every run guards nothing.
* **Step 4 (the owner's question):** the layout machinery does control what it targets. The layout-attributable
  faults of the static probe are low and below the reference (start-up only: libniji 10, reference 29). But the
  faults users pay beyond the reference are not layout faults: in the static `tool` probe they are anonymous memory
  (heap first touches spread over size-class regions, locale data held in anonymous memory), and above all the
  **dynamic** start, which is how users run libniji (D-X.141): 65 / 95 against the reference's 48 / 66 (start-up only
  / tool, Haswell-EP). The phase tests now chase ±1 from cold code outside the trace.

## 2. Method

* Heads: next 0495c4684 (`base`); next plus a docs and test-only edit (`nocode`); next plus a one-line libm edit
  (`core::hint::black_box` in `jn`, `libm`); next plus a one-line libc edit unrelated to start-up (`black_box` in
  `glob`, `glob`). All scratch edits, never committed. Fast products (the gate's profile), built once per head on the
  Haswell-EP build host. The `base` and `glob` products were then run unchanged on the Skylake-SP host (CI's class);
  products do not depend on the build host (LAYOUT-TRACE-HOST-CANONICAL, from the record). The Zen 1 host class was
  **not measured**.
* Probe: `crates/core/tests/oracle/page_faults.c` linked exactly as `hot_text_products.rs` links it, with a start file
  padded as its phase tests pad it: `none`; `cold` 0x800 and 0x1000..0xF000 in 4 KiB steps (a full window);
  `pre` 0x800/0x2000/0x8000; `data` 0x800..0x3000; `lbss` 0x800/0x2000; `relro` 0x4d0/0xc18. All ten modes, the
  test's discipline (no address randomisation, argv0 `page_faults`, cwd `/`, environment `LANG` only, warm-up + 5,
  lowest). Staged from ext4 (`/tmp`) and tmpfs (`/dev/shm`) at equal-length paths. Every run after the warm-up was
  identical in every cell (0 exceptions).
* Dynamic: `page_faults.c` linked against the staged products' ld.so and libc.so.6 (as `dyn_startup_faults`).
* Reference: the same source built by the host compiler and C library, static and dynamic, same runs.
* Attribution: `perf record -e page-faults -d` per mode; addresses inside the probe are placed by its section table
  (exact); addresses outside it are placed by perf's mmap records (approximate, the records of the perf and relay
  processes are not separated; the dynamic breakdown is **not** given for that reason).
* Red history: all 1,520 CI gate logs (10-21 to 11-23), failures per test (`FAILED` lines, both log formats).

## 3. Measurements

Static probe, fast products of next, lowest of 5 (identical on ext4 and tmpfs, every run identical):

| mode | bound (fast) | Haswell-EP | Skylake-SP | moves with a pad (Haswell-EP / Skylake-SP) | reference static H / S |
|---|---|---|---|---|---|
| true (start-up only) | 11 | 10 | 10 | `pre` +1 only / `pre` +1 only | 29 / 29 |
| locale (setlocale + printf) | 43 | 43 | 56 | `pre` +1 / `pre` +1, **`cold 0x2000` +1** | 39 / 53 |
| tool (coreutils-shaped) | 44 | **45** | **58** | `pre` +1 / `pre` +1, **`cold 0x2000` +1** | 39 / 53 |
| fork child | 5 | 5 | 5 | none / none | 7 / 7 |
| localtime, localtime + mktime | 36, 36 | 36, 36 | 36, 36 | `pre` +1 / `pre` +1 | 32 / 32 |
| float (first %.2f) | 3 | **4** | **4** | none / none | 1 / 1 |
| strings, mktime, rebind | 0 | 0 | 0 | none / none | 0 / 0 |

Dynamic probe (true / locale / tool / fork): libniji Haswell-EP 65 / 93 / 95 / 5, Skylake-SP 65 / 107 / 109 / 5;
reference Haswell-EP 48 / 66 / 66 / 10, Skylake-SP 49 / 81 / 81 / 10. `DYN_TRUE_BOUND` fast is 73.

Variants (every mode, every pad, both file systems):

| head | products vs next | Haswell-EP counts | Skylake-SP counts |
|---|---|---|---|
| nocode (docs + test comment) | byte-identical | identical | (identical products) |
| libm (`jn`) | only libm.a differs | identical | (libc products identical) |
| glob (libc, not start-up) | libc.a, libc.so.6, libm.a differ | identical | `cold 0x2000` no longer moves: 58 at every pad |

Where the static `tool` faults come from (perf, Haswell-EP, counted process, 44 faults under perf): anonymous
memory 26 (heap pages 256 KiB apart, one per size-class region first used, and a second anonymous block),
stack 4, `.text` 4, `.lbss` 3, `.data1` 2, `.rodata1` 1, `.rodata` 1, `.data.rel.ro` 1, the program file's own pages
2. Reference static `tool` (38): `.text` 11, locale-archive 7, `.bss` 4, `.data.rel.ro` 3, heap 3, `.rodata` 2, and
single pages. So libniji's layout-controlled share (about 13) is about half the reference's (about 24); its excess
over the reference is anonymous memory.

The Skylake-SP `cold 0x2000` delta (both file systems, `locale` and `tool`): exactly one more `.text` fault. Next:
cold text at 0x7561c0 and 0x72da80. At `cold 0x2000`: 0x7581c0, 0x72fa80 **and 0x730000** (the function now spans a
64 KiB boundary). No data region moves.

## 4. Inventory and classification

Robust: moves only when the thing it protects changes. Brittle: moves with layout, padding, file system or host.
Wrong target: counts something other than what users pay for.

| bound (where) | protects | blocking? | class | evidence |
|---|---|---|---|---|
| `TRUE_BOUND`, `LOCALE_BOUND`, `TOOL_BOUND`, `LOCALTIME_BOUND`, `LOCALTIME_MKTIME_BOUND`, `FLOAT_BOUND`, `FORK_BOUND`, `WHOLE_COUNT_SLACK` (hot_text_products.rs) | static start-up faults per probe | no (D-X.136 report) except `FORK_BOUND` inside the data-phase test | **brittle (host environment)**, robust to FS and to unrelated code on Haswell-EP; partly **wrong target** (static -O1 non-PIE probe; users run dynamic) | locale/tool +13 between host classes with identical products; tool, float over their bounds on both classes |
| `DYN_TRUE_BOUND` | dynamic start-up faults | no | robust to FS now; **stale** (65 vs 73, two-sided check reports) | 0 FS differences; 65 on both classes |
| `init_data_clusters_hold_at_every_text_phase` (equal counts at every pad) | start-up state stays in its clusters whatever lies around it | **yes** | **brittle**: verdict is a function of unrelated bytes on the host class whose locale path is untraced | red on next itself (Skylake-SP), green with an unrelated glob change; 5 red runs since 11-12, all `tool` ±1 at one pad (`cold 0x2000`, `cold 0x8000` or `data 0x3000`) |
| `init_zero_fault_calls_hold_at_every_text_phase`, `init_first_string_calls…`, `…rebind…`, `…mktime…` (== 0) | first calls after start-up take no text fault | yes | **robust** | 0 at all 28 pads, both hosts, every head; the one red (11-13, mktime 1 at *every* pad) was a real change, not phase |
| `init_startup_data_pages_hold_at_every_data_phase`, `init_startup_rodata_stays_in_one_window_for_any_text_size`, `init_window_bins_start_on_window_boundaries`, `init_static_probe_has_no_got_at_any_relro_size`, data1/rodata1 span checks | layout structure, from the link map | yes | **robust** (a deterministic function of the link; no red in the history) | — |
| `WINDOW-BIN-BYTES` (layout/contracts.tsv): bin bytes, head-room 1024, windows per mode | per-story start-up layout gate | bytes yes; windows only on release (D-X.145) | bytes **robust** (cause, in bytes); fast-profile windows brittle (D-X.145, from the record) | — |
| `STARTUP-TEXT-WINDOWS`, `STARTUP-DATA-WINDOWS` budgets | windows/pages start-up touches | release metric (D-X.138) | robust to file system (link map); host-environment-dependent through the traced key set (**guess**, not measured here) | — |
| `DYN_START_INSTR` (per CPU model, +2%, and a 10% floor) | dynamic start-up instructions | yes | robust to layout (instruction counts); host-keyed by design; **no bound for Haswell-EP** (skipped there) | red 11-23: 173,794 > 173,600 (not attributed here) |
| `EMPTY_CALLS`, `THREE_CALLS`, `CLOCK_CALLS`, openat bound | dynamic start system calls | yes | **robust** (deterministic) | no red in the history |
| `AS_STEPS`, `AS_HEADROOM` (heap_meta_products.rs) | heap address space (issue #21) | yes | was **brittle (NUMA node migration)**, 5 red runs 11-21/22; pinned to one node since (ruling #7441, from the record) | — |
| `BOUND`, `FIRST_BOUND` (heap_meta_products.rs), `PAIR_BOUND` (heap_foreign_pair_cost.rs) | heap metadata faults, foreign-pair cost | yes | not measured here | no red in the history |
| `validation/wl_holds.tsv`, wl_verdict 1.10x / geomean 1.00 | workload parity against the reference | release/train | **robust by construction**: a ratio against the reference on the same host cancels host environment (locale source moves both by +14) | — |

Red history (distinct CI runs naming a `hot_text_products` test, 10-21 to 11-23): short programs 17, coreutils 10,
data clusters 5 (11-12, 11-13, three on 11-23), localtime + mktime 4, localtime 3, rebind 3, first mktime 1, zero-fault
calls 1, dynamic fault 1, dynamic instructions 1. The runtime-count reds stop after 11-15 (D-X.136 made them reports); since then every `hot_text_products` red
but one is the data-phase test or the dynamic instruction bound.

## 5. Replacements (proposed; nothing changed)

1. **Data-phase test: bound the cause, not the equality of counts.** Keep the structural half (`.rodata1` in one
   window, `.data1` in ceil(size / 4 KiB) pages, `FORK_BOUND`). Replace "the `tool` count is equal at every pad" by:
   *every text address the probe modes execute lies inside the hot group* (an address list from one perf run of the
   unpadded probe, checked against the link map's hot range; a miss names the function). That is red on next today
   on Skylake-SP **at every head**, not only on the stories whose bytes happen to cross 0x730000, and it names the
   function. It protects the same thing at least as tightly: any start-up text outside the group is caught, whether
   or not a pad makes it straddle.
2. **Cover the locale source in the trace.** The trace probes load the locale from the archive; add a run that loads
   it from the per-locale directory (both layouts are what distributions ship), so that path is grouped. Expected
   effect on Skylake-SP: `locale`/`tool` back toward the Haswell-EP levels (**guess**).
3. **Runtime counts as a parity ratio, per host, against the reference.** Replace the absolute fault bounds (stale on
   both measured classes, firing every run) by libniji / reference on the same probe, same host, in the same test
   (as the workload rule does): the host environment then cancels. The ratchet becomes "ratio never rises".
4. **Add the dynamic probe's locale and tool modes to the reported metrics.** They are the largest user-paid gap
   (+27 to +29 faults over the reference on both hosts), and no bound watches them.
5. **`DYN_START_INSTR`:** add the Haswell-EP model or key by CPU class, so a build host class does not skip it.

## 6. The ratchet, plainly

The ratchet itself is not what turns unrelated stories red today: the absolute counts it tightened are non-blocking
since D-X.136. What does is a blocking **equality** check measured on the build host's environment and enforced on a
host whose environment takes an untraced path, where a ±1 straddle comes and goes with unrelated bytes. The
ratcheted absolute values add noise instead (reports on every run, both directions). A rule that serves the goal
(reference parity, measured honestly): ratchet the **ratio to the reference** on the same host and probe, and gate
per story on **causes** (bytes per bin, text outside the hot group), never on a step function of the whole layout.
The owner decides any change to the ratchet.

## 7. Not measured

Zen 1 host class; release products; the gate wrapper's own run of `hot_text_products` with tmpfs `/tmp` (the sweep
staged on tmpfs instead, 0 differences); the dynamic probe's per-region breakdown; the 11-23 `DYN_START_INSTR` red.
