# Layout budgets: why page-quantized start-up bounds flap, and a layout unrelated bytes cannot move

LAYOUT-BUDGET-RETHINK (2026-11-14), design story: no product, test or gate change. Pending record
`records/pending/LAYOUT-BUDGET-RETHINK-1.toml` (series D-X). Reads with `plan/layout-rethink.md`,
`handoff/layout-stability-rethink.md`, LAYOUT-TOOL-BOUND-SHARED's handoff (on its branch), D-X.49, D-X.62, D-X.63,
D-X.88, D-X.92. Proposed stories: `handoff/LAYOUT-BUDGET-RETHINK/*.md`. Everything below that is not marked
NOT MEASURED was measured on next e47a71997 (section 2.2) or read from the ops history (2.1).

## 1. Answer in brief

The coordinator's hypothesis holds (section 2), with one correction: a ratchet at the measured count does not put
the bound on the edge of its step, but nothing keeps it away from the edge either, and today's layout has many edges
per bound. Fast `tool` reads 50 against bound 50 with **under 512 bytes** of headroom for growth anywhere in the
442 KiB of hot code ahead of the clusters. Three structural defects, not bad luck:

* **D1. Power-of-two cluster alignment fixes a cluster's own window count, not which clusters share a window.**
  `cluster_align` (xtask/src/hottext.rs) gives a 14-21 KiB cluster 32 KiB alignment. Whether two clusters share a
  64 KiB window then depends on the block's start modulo 64 KiB, which every byte ahead of the block moves (the
  11-10 design's "clusters share windows by their own sizes only" is false for alignments under 64 KiB).
* **D2. The hot group is one first-touch sequence over all probes.** Each probe's code is sparse in it (the tool
  path touches 6 windows of the group), so a probe's count depends on the bytes of functions it never runs.
* **D3. The gate counts a step function of the whole layout.** It cannot say which bytes crossed, and it fires on
  stories whose code no probe runs. Fat LTO couples them anyway: a new caller of a shared helper changes inlining at
  start-up call sites (class 3 of `plan/layout-rethink.md` 1.2), so "unrelated" growth reaches start-up bytes.

Proposal: **window bins** (section 3: start-up text in 64 KiB-aligned bins chosen by which probes run each
section, gaps filled with cold code), **byte budgets per bin from the link map as the per-story gate, with fault
counts as a ratcheted release metric** (section 4, owner decision), and **a start-up set declared by rule and checked
against traces** (section 5).

## 2. Evidence (Q1)

### 2.1 History (ops store, 2026-10-21 to 11-14)

| measure | value |
|---|---|
| CI red verdicts naming a start-up fault bound (`hot_text_products`) | 13 of 76 red verdicts (17%); **9 of the 13 are libm stories**, whose code no fault probe runs |
| trains not green that named a layout bound | 43 of 286 (15%) |
| events naming the LAYOUT-JITTER hold (#4347) | 74, over 35 stories (24 libm, 6 dynamic loader, 5 other) |
| red/fail events naming a layout bound; escalations naming one | 112 over 60 stories; 91 over 82 stories |
| layout-family stories (LAYOUT-, HOTTEXT-, HOTLIST-, STARTUP-) | 124 of 1,773 runs; **590M of 5,108M tokens (11.6%)**; chains TEXT-PIN 15, DATA-CLUSTERS 14, STARTUP-RODATA 8 |

### 2.2 Byte margins on next (the claim measured)

Method: `page_faults.c` linked exactly as `crates/core/tests/hot_text_products.rs` links it, with `niji-crt1.o`
`ld -r`-merged with a pad (the test's `padded_start` method) at nine insertion points, pads 0, 64, 128, 256, then
512-byte steps to 64 KiB (133 pads per point, 2,394 links); every mode run as the test runs it (relay, 35-byte path,
argv0, cwd /, LANG only, ADDR_NO_RANDOMIZE, warm-up plus two runs). Haswell-EP build host, both profiles built from
next e47a71997. Pad 0 reproduces the test (fast 11/46/50/5/17/17/3, release 10/47/49/5/18/18/3; 0-fault modes 0).
Cell: first pad (bytes) at which the count exceeds its **bound**, and the share of the 133 pads that are red.

| point (what grows) | fast | release |
|---|---|---|
| `pre`: code ahead of the group (a program's own `.text.startup` / `.text.unlikely`) | true 1,024 (95%); tool 512 (48%); locale 4,096 (42%); fork 6,656 (31%) | locale 1,536 (44%); fork 7,168 (30%); tool 36,864 (25%) |
| `tier`: start-up tier end (start-up code growth) | tool 512 (48%); float 4,608 (33%); locale 4,096 (43%); localtime 21,504 (17%) | locale 1,536 (44%); localtime 1,536 (21%); tool 36,864 (25%) |
| `mid`: hot code after the tier (printf, setlocale, gettext, ...) | **tool 512 (97%)** | none moved |
| `cold`: all other text (libm, error paths) | float 18,432 (70%); tool 17,408 (2%); true 17,920 (1%); strings, rebind 0 -> 1 | float 18,944 (70%); locale 15,360 (5%) |
| `relro`: relro head | tool and float 20,992 (61%) | locale 37,376 (36%) |
| `ro`, `data`, `bss`, `lbss` heads | no count moved | no count moved |

Geometry of the same links (link map): the 442 KiB of grouped code ahead of the clusters ends **470 bytes** (fast)
below the 32 KiB boundary the cluster block aligns to (release: 29,444 bytes); the five clusters (20.6, 16.8, 19.2,
16.6, 14.4 KiB) sit at 32 KiB phases and pair up in windows by that phase. The written start-up cluster `.data1`
(11,744 bytes, page-aligned) is **544 bytes** under its third page's end, both profiles (STARTUP-DATA-WINDOWS rw: 7
of budget 7, fast). The start-up read-only cluster `.rodata1` (47,040 / 44,684 bytes, 64 KiB-aligned) has 18 / 21
KiB of headroom. On the Skylake-SP CI host class, LAYOUT-TOOL-BOUND-SHARED measured 1,494 bytes for the tool bound
on an older next; the margin differs by host, which is itself a defect (section 5).

Readings. (1) Every whole-program bound is red for a quarter to all of the growths at some point, and the
smallest such growth is 0.5-1.5 KiB in four of them. (2) The data clusters work: shifts ahead of `.rodata`,
`.data`, `.bss` and `.lbss` move nothing. What remains on the data side is growth inside a cluster (544 bytes left
in `.data1`), which a byte budget reports exactly. (3) The text side is where D1 and D2 bite: `pre`, `tier` and
`mid` move counts at a few hundred bytes. (4) The `pre` row is also the spread between programs: an -O2 program
puts its own `main` ahead of the group (section 6), so the counted phase is one of many.

### 2.3 The 11-14 cases in this light

`tool` 51 > 50 on three branches: the 470-1,494-byte margin ahead of the cluster block (D1); one branch grew start-up
code by 2,176 bytes (its own cost, unattributed by the gate), two did not change start-up code at all. Written pages
8 > 7: a 1,640-byte table pushed a page-aligned start-up group over its page count, on the CI host only (the start-up
set followed the tracing host: section 5). First float 4 > 3: the float row above (`tier` from 4,608 bytes).

## 3. Layout invariance by construction (Q2)

**Can the start-up groups get fixed, aligned positions with everything else after them? Yes for data (done: 2.2
shows it), and yes for text only if the unit of alignment is the 64 KiB window and the unit of grouping is the set of
probes that run a section.**

**Window bins.** Trace-at-build already knows, for every grouped section, which probe modes execute it. Group by that
**signature**: pack the sections into bins of at most 64 KiB, signatures kept together so that each probe touches as
few bins as possible (a few dozen signature groups: an exact, deterministic plan at build time). Every bin starts on
a 64 KiB boundary. A probe's text count is then the number of bins it touches, a function of the plan alone:
growth inside a bin changes nothing until the bin is full, and growth anywhere else changes nothing at all. The
0-fault phases (time, strings, rebind, float) are bins, or parts of the bins, their first call touches.

**Gaps cost almost nothing.** The space between a bin's end and the next boundary is filled with cold sections
(code no probe runs, best fit by size, from the Rust object every link pulls in), named so that GNU ld's
`SORT(.text.sorted.*)` puts them between the bins. A misfit (a filler that grew, a member a program does not pull)
only adds alignment fill; the next bin still starts on its boundary. Residual fill per bin: NOT MEASURED (the cold
pool has thousands of small functions; expected under a few hundred bytes).

**The head bin.** Whatever precedes the group (crt1, `.init`, `.iplt`, the program's own `.text.startup` and
`.text.unlikely`) sets the head's phase. Variant (i): anchor the head bin at 64 KiB; every program then gets the same
counts, for up to 64 KiB - 1 bytes of fill ahead of it in a static program (mean 32 KiB, file bytes; the D-X.51
anchor cost, now worth paying because the bytes behind the anchor are dense). Variant (ii): no head anchor; the head
bin costs 1 or 2 windows depending on the program, fixed per program, never per story. `libc.so.6` can take (i) at
no per-program cost (its own segment start; a 64 KiB `p_align` also fixes the window phase under address
randomisation, which today's page-aligned mapping does not).

**What changes in the pipeline.** The trace records per-section signatures (today: first-touch order and tier); a
planner replaces `group()`'s single sequence and `cluster_align`; the plan is written to
`target/product/<profile>/layout-bins.tsv` (generated), which the byte gate reads. Users' links are unchanged (section
names and alignments carry the plan, as today). Data: keep LAYOUT-DATA-CLUSTERS' clusters; report each one's bytes
against its page or window capacity.

**Alternatives.** Keep pow2 alignment and add a 64 KiB anchor at the cluster block only (LAYOUT-TOOL-BOUND-SHARED's
option A): removes this incident's edge (the block's phase), not D2 (the tier's internal windows; `tier` and `pre`
rows). Phase-free bounds (bound = worst phase): stable but looser by 1-2, rejected in the 11-10 design and here.

## 4. Gate on the cause (Q3): options for the owner

| | G0 today: fault counts per story | G1 byte budgets on today's layout | **G2 bins + byte budgets per story; counts at release** |
|---|---|---|---|
| false reds from unrelated growth | 9 of 13 CI reds were libm; 35 stories held (2.1) | none from phase, but counts still move with no group growth (D1, D2): misses | none: a count can change only when a bin outgrows its capacity, which that bin's budget reports |
| real regressions caught | yes, unattributed (setlocale +2,176 bytes read "51 > 50"; attribution took a story) | partly | yes, at the branch's own gate, naming the bin, bytes and symbols |
| host dependence | the trace and the CI host differ (2.2, 2.3) | link map: host-free once the set is canonical | same |
| cost per story | runtime fault tests in the products step | reading the probe link map `so` already makes (seconds) | same; fault counts once per train |
| ratchet | counts only tighten (#4079), at the measured value, no headroom | bytes | a bin's budget is its capacity; tightening removes a bin (a planned change), never after each story |

Non-layout faults (stack, heap, locale archive: 33 of 55 of the tool count in the 11-10 attribution) are outside every
byte budget; the release metric keeps them, per class (text, read-only, relro, written, other).

## 5. Canonical start-up set (Q4)

The set is what the probes ran on the build host. It follows that host's CPU (IFUNC choice, CPU-class pages), locale
data and environment, so CI's build and the developer's build can group different bytes (11-14, the tunables table).
(a) A fixed trace environment removes environment and data dependence, not CPU dependence. (b) A union of host
classes needs traces committed from other hosts: names cross builds again (what LT-5 removes); rejected. (c) **A
declared set by rule**: the traced set plus every candidate of a traced IFUNC resolver, both pages of each CPU-classed
pair, and every object a start-up function references RIP-relatively (one level, data only; the call closure was 2-4x
the traced bytes and rejected in `plan/layout-rethink.md` 3.3, the data closure is NOT MEASURED). Each build checks
that the traced set is a subset; an extra member is red, naming it. Recommend (a) + (c).

## 6. The frame (Q5)

1. **The counted configuration is one phase of one program.** The probe is built -O1, static, non-PIE, run without
   address randomisation. GCC at -O2 places `main` and constructors in `.text.startup` and cold paths in
   `.text.unlikely` (`-freorder-functions`), which GNU ld's default script puts ahead of `.text.sorted.*`; the `pre`
   row of 2.2 is therefore the spread across real static programs (true 11 or 12, tool 50-52). Dynamic programs map
   `libc.so.6` at a page-granular random address. Bounds tuned to one phase optimise that phase.
2. **Ratcheting a quantised metric at its measured value, with the headroom invisible**, turns each tightening into
   a tripwire for the next story. Ratchet what the code controls (bytes per bin, bins per probe).
3. **A symptom gate without attribution** costs an investigation story per incident (LAYOUT-TOOL-BOUND-SHARED).
4. **Precision work on placement before the measurement model**: 37 runs of TEXT-PIN, DATA-CLUSTERS and
   STARTUP-RODATA moved bytes against a metric that moves with unrelated bytes.

## 7. Rulings needed and stories

* **R1** adopt window bins (LAYOUT-WINDOW-BINS); head variant (i) for `libc.so.6`, (i) or (ii) for `libc.a` by the
  story's measured fill and counts. Recommend yes.
* **R2** per-story gate = byte budgets per bin (LAYOUT-BYTE-BUDGETS, after R1); fault counts become a per-class release
  metric ratcheted at the train. Changes what blocks a landing. Recommend G2.
* **R3** canonical start-up set by rule plus subset check (LAYOUT-CANONICAL-SET; independent, can land first).
  Recommend yes.
* **R4** interim, until R1/R2: keep the LAYOUT-JITTER hold, and let the coordinator clear a held story when its link
  map shows no growth of the start-up groups against next (the geometry of 2.2, seconds per branch). Recommend yes.

No bound, contract or gate changes in this story.
