# Decision-evidence audit (DECISION-EVIDENCE-AUDIT, 2026-11-23)

Which recorded decisions, defaults and limits rest on supposition rather than evidence, judged against the owner's
decision principles of 2026-11-23 ("when we don't know, more data"; "defaults need evidence"). Read-only: no
decision, product or test is changed here. Items that do not fit are proposals; anything touching a departure, an
owner ruling or a public behaviour is marked RULING NEEDED.

Files: `tools/decision_evidence_audit.py` (the filter, re-runnable), `items.tsv` (its output: every flagged item,
class, evidence level, excerpt), `probes/` (the black-box probes run for this audit and their runners).

## 1. Mechanical filter

`python3 tools/decision_evidence_audit.py` (from the tree root; `--rulings` names the owner rulings file, default
`~/src/liella-ops/state/PH14-OWNER-RULINGS.md`). An item is **flagged** when it sets a default, size, limit, budget,
window, reserve or bound, or departs from the reference (every differences row departs). Evidence level:
`measured` (cites a probe, measurement, job or results file), `spec` (cites a spec clause or standard only; that
fixes a behaviour, rarely a value), `none`. Class: **a** a cost every process, thread or call pays (start-up,
memory, per-thread or per-call work, layout windows, budgets, bins), **b** a departure, **c** another value;
`provisional` is a flag. The filter is regular expressions only: it decides what a person reads, not the verdict.

| Source | Items | Flagged | Measured | Spec only | None | Provisional | a | b | c | a not measured | b not measured | c not measured |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Decision records (`records/decisions`) | 781 | 516 | 346 | 58 | 112 | 7 | 318 | 56 | 142 | 83 | 19 | 68 |
| Differences rows (`docs/differences-from-glibc.md`) | 384 | 384 | 61 | 308 | 15 | 2 | 18 | 366 | 0 | 12 | 311 | 0 |
| Owner rulings entries | 309 | 108 | 47 | 1 | 60 | 25 | 45 | 34 | 29 | 19 | 23 | 19 |
| **Total** | 1,474 | 1,008 | 454 | 367 | 187 | 34 | 381 | 456 | 171 | 114 | 353 | 87 |

(The brief's "about 410" differences rows counts table header rows; 384 are data rows. The rulings file is split
into one item per table row, bullet or paragraph: 309 items; 34 of the flagged ones mention "provisional".)

Reading: 554 of 1,008 flagged items (55%) cite no measurement; 187 cite nothing at all. Of the 381 class-a items
(costs everyone pays), 114 cite no measurement and 79 cite nothing.

## 2. Judgement (class a first, most expensive first)

Judged individually: the class-a items below, chosen from the 114 unmeasured class-a items plus the values in code
that every process or thread pays (found by reading `crates/boundary/src/thread.rs`, `crates/rt/src/thread.rs`,
`crates/ld/src/tls.rs`, `crates/boundary/src/heap.rs`), then the provisional rulings. **Not judged
individually:** the other class-a items and the class-b/c items in `items.tsv` (353 unmeasured departures, mostly
differences rows that cite a spec clause, which is the evidence a behaviour departure needs; 87 class-c values).
They stay listed, unjudged, for a follow-up.

Measurements: Haswell-EP build host (Xeon E5-2697 v3, 26 vCPU), Linux user-mode `perf stat` counts, libniji
release products of `next` 0495c4684 (the base of this branch, which changes no product file), the dynamic kit
(libniji's `ld.so`) unless marked static; the reference is the installed glibc 2.39. Every count was repeated three
times; instructions varied by under 0.01%.

| # | Item | Recorded in | Class | Verdict | Evidence, or cheapest data and decision rule | Per-process cost (measured unless marked) |
|---|---|---|---|---|---|---|
| 1 | Thread-key capacity `KEYS = 4096` and the exit/reuse scan over it | D-22.5 ("keys beyond `PTHREAD_KEYS_MAX` are allowed", no value or measure); `crates/rt/src/thread.rs` `KEYS`, `KeyTable::span` | a (per thread), b | **DOES NOT FIT** | No record gives a reason for 4096 or a cost. Measured: `span()` scans all 4096 sequence words from the top (`rposition`) and finds nothing in a program that made no key; it runs on every cached-stack reuse (`take_mapping`) and at every thread exit (`run_destructors`). perf: 98.2% of user instructions of create+join are self time in `create_inner` (51.4%) and `finish` (46.9%); `perf annotate` puts 97% and 99% of those on one 4-way unrolled loop over 0x4000 bytes (4096 × u32) = about 34,700 of the 36,026 instructions per pair. Proposal (internal, no ruling): keep a high-water mark of created keys so `span()` is O(1); then size the per-thread key area to the reference's 1,024 entries with growth for more (D-22.5's "more are allowed" kept). Rule: if create+join on the percall row is then at or below the reference's 2,274, done; else profile again. | 34,700 instructions per `pthread_create`+`join` (of 36,026; reference total 2,274); 64 KiB key area of address space per thread mapping |
| 2 | Static TLS surplus 64 KiB under libniji's `ld.so` (`ld::tls::SURPLUS`) | R14-4 (provisional), D-22.6, D-27.20; already raised as #8009 | a (per process and per thread), b, d | **DOES NOT FIT** (as #8009 found) | Adds two facts to #8009. (i) D-22.6 says the surplus "is sized by tunable"; the code has a constant, and `glibc.rtld.optional_static_tls` is parsed (`crates/core/src/config.rs`) but read by nothing: the record does not match the code. (ii) On every cached-stack reuse `take_mapping` zeroes the whole static area including the surplus (`thread.rs`, "the surplus as in a fresh mapping"), so the surplus's comment "no page is touched until used" holds only for a thread's first mapping. Measured: a libniji thread's mapping is 8,396 KiB against the reference's 8,192 KiB; the stack top sits 69,820 bytes below the thread pointer (reference 2,060); the first thread costs 32 extra faults (reference 9). The share of the surplus in those faults is not isolated (guess: 16 of the 32). Rule, from the principle: the reference's default unless a named, measured case; honour `optional_static_tls` for the rare case. **RULING NEEDED** (owner ruling R14-4). | about 64 KiB zeroed per created thread after the first (`rep stosb`: few instructions, about 64 KiB of stores); 64 KiB address space per thread and per process |
| 3 | The start-up data budget charges the heap reserve's `.lbss` page | D-X.138 (`WINDOW-BIN-BYTES`: "the RIP-relative operands of the `text-keys=` functions, each cut to its first unit"), D-X.104 | a (budget) | **DOES NOT FIT** for this member | The budget counts an object whose address start-up code takes as a touched page. `RESERVE_MEM` is only address-compared at start-up (`in_reserve`, `reserve_base`). Measured (probe 3, static release, fault addresses): 23 user faults for a one-allocation program, **0** inside `RESERVE_MEM` [0x8dd000, 0x8ed000). The release profile's second `.lbss` buffer was not checked. Proposal (tooling, tightening): before charging a page, require a fault on it in the trace; rule: a budgeted page with 0 faults in every kit program leaves the budget, and `rw-budget` drops by that page. | 0 faults (the budget charges 1 page fast, 2 release, that no measured process takes) |
| 4 | Per-thread area above the thread pointer | D-22.9 (layout), `Layout::new` | a (per thread) | **UNKNOWN** | Measured: the thread pointer is 139,200 bytes below the mapping's end (reference 2,368). 65,536 of that is item 1's key area and 544 the result slots; about 73 KiB is not attributed by this audit (guess: the thread block, descriptor and loader tail under `ld.so`). Cheapest data: a unit test printing `BLOCK_SIZE`, `DESC_SIZE` and the loader's tail. Rule: any part larger than the reference's with no named case shrinks or moves to first use. | 136 KiB address space per thread; resident only where touched |
| 5 | Heap reserve `RESERVE = 64 KiB` | `crates/boundary/src/heap.rs` (T22), no record of the value | c (value nobody states a reason for) | **UNKNOWN** (cost near zero) | Never used while the heap serves; 0 faults measured (item 3). Cheapest data: the reserve's high-water mark across the exhausted-heap tests. Rule: size it to twice the measured peak rounded to a page; not worth more than that, its cost is address space only. | 64 KiB address space; 0 resident |
| 6 | Heap segments 16 MiB, 64 shards, first reservation one mapping | D-3.10, D-3.19 | a (per process) | **FITS** for resident memory, **UNKNOWN** for address space | Measured (dynamic, one `fopen`): RSS 992 KiB against the reference's 1,352; `VmSize` 11,192 KiB against 2,684 (+8,508). The 16 MiB and 64 values have no recorded reason. Cheapest data: the dynamic kit under an `RLIMIT_AS` a little above the reference's `VmSize`. Rule: if a program the reference runs fails there, RULING NEEDED (address-space default); else keep. | +8.3 MiB address space; RSS 360 KiB lower |
| 7 | Thread stack cache up to 40 MiB, default stack, guard | D-22.9 | a (per thread) | **FITS** | Matches the reference (principle: matching needs no data). Measured: default stack 8 MiB and guard 4,096 on both. | as the reference |
| 8 | Dynamic start-up | the start-up rows (D-X.133 and others) | a (per process) | **UNKNOWN** (no link found) | Measured: a one-`fopen` dynamic program takes 201,021 user instructions and 76 faults against the reference's 138,655 and 57; an empty threaded `main` 194k and 61 against 127k and 55. No assumption judged here was shown to cause this gap; items 2 and 3 do not (start-up touches neither page). | +62k instructions, +19 faults per process |
| 9 | Per-thread regex DFA store 32 KiB | D-14.12 | a (per thread that uses `regexec`) | **UNKNOWN** | No reason for 8,192 states. Cheapest data: peak DFA states over the regex bench rows. Rule: if the peak is far below, size to it with growth. | 32 KiB on first `regexec` per thread |
| 10 | `regexec` stack scratch about 4 KiB | D-14.11 | a (per call) | **FITS** | The cap is set by a measured case: a thread on a `PTHREAD_STACK_MIN` stack (conformance program `s14_regexec_min_stack`). | 3,712 bytes of stack per call |
| 11 | Per-thread cache words | D-X.17 | a (per thread) | **FITS** | Stated +0 bytes per thread (inside the reserved block). | 0 |
| 12 | Lazy internal signal handler | D-20.9, owner ruling #7075 | a (start-up) | **FITS** | Removes start-up work; the ruling names the case. | removes work |
| 13 | `GLOB_BRACE` expansion bound | rulings: provisional 11-04 (#2401) | b, c, d | **UNKNOWN** | "sized far above realistic use" with no number. Cheapest data: the largest brace product in a corpus of real patterns (the shell-script corpus the package builds use). Rule: if the bound is at least 100 times the corpus maximum, keep and record the number; else RULING NEEDED. | none at run time (a bound on hostile input) |
| 14 | RPC TCP record cap 4 MiB | rulings: provisional 10-23 (#1466) | b, d | **MOOT** | The ONC RPC client and server are failing stubs (D-29.5, owner 2026-10-27), so the cap guards no code. Proposal: drop the entry from the provisional list. | none |
| 15 | `__libc_freeres` inert | rulings: provisional differences list (#1317) | b, d | **MOOT** | Already reversed (#7929, D-29.7). The provisional section still lists it under the heading; proposal: move it. | none |

Provisional rulings not in the table (the regex fuzz-timeout fixes #1782/#1818, the generator design #1811, the
f64 entry and strstr scanner trades #1891/#1896): each cites a probe or an instruction count (`measured` in
`items.tsv`) or changes nothing observable; not re-judged.

## 3. Performance links (the owner's hypothesis)

1. **Thread-create cost (THREAD-CREATE-COST): caused by item 1, measured.** Probe 1 (`probes/thread_cost.c`,
   N = 1,000 and 2,000 create+join pairs, the difference over 1,000 so start-up cancels): libniji 36,026 user
   instructions per pair, the reference 2,274 (the story's figures: 36,092 and 2,266). `perf record` /
   `perf annotate` (probe 2) put about 34,700 of the 36,026 on the 4,096-entry `KeyTable::span` scan, run twice per
   pair (create on a cached stack, exit). The assumption: a 4,096-entry key table (D-22.5, no recorded reason) that
   is cheap to scan. Removing the scan (high-water mark) leaves about 1,300 instructions per pair: a projection from
   the profile, not a measurement. Kernel instructions per pair are lower than the reference's (61k against 70k).
2. **Thread memory: item 2 and item 1's key area, measured.** A libniji thread maps 204 KiB more than the
   reference's and its first use faults 32 pages against 9. On a reused stack the 64 KiB surplus is zeroed again
   (cycles, not instructions). The fault split between surplus, key area and the rest is a guess.
3. **Start-up data-cluster bound (`hot_text_products` / STARTUP-DATA-WINDOWS): item 3, measured for one member.**
   The budget charges pages by address-taken objects; `RESERVE_MEM`'s page takes no fault. This does not slow any
   process; it makes the bound count a page no process pays, so a story can go red (or be charged headroom) for it.
   Whether this explains past start-up bound churn is a guess.
4. **`DYN_TRUE_BOUND` (dynamic start-up faults 73 fast, 67 release): no class-a assumption was linked.** The surplus
   and the reserve are not touched at start-up (probe 3; the surplus is anonymous memory). The dynamic start-up gap
   (+62k instructions, +19 faults on a one-`fopen` program, item 8) remains unexplained by this audit.

## 4. Probes run

| Label (jobrun, kind probe) | What |
|---|---|
| decision-evidence-audit-probe1 | `probes/run.sh`: thread_cost against both libraries, `perf stat` (N = 0, 1,000, 2,000, 3 repeats), thread mapping shape, `perf record` |
| decision-evidence-audit-probe2 | `probes/run2.sh`: `perf annotate` of `create_inner` and `finish`, `PT_TLS` of both `libc.so.6`, one-allocation memory and counts |
| decision-evidence-audit-probe3 | `probes/run3.sh`: static one-allocation program, every user fault's address against `RESERVE_MEM` and the sections |
