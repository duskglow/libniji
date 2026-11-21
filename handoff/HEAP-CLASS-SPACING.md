# HEAP-CLASS-SPACING: handoff and report (DO NOT LAND: evidence only)

State: branch `task/heap-class-spacing`, base origin/next `7e5d8b3fa`; commits `520e74474` (eight classes per doubling
in [256, 1 KiB), 52 classes), `91cfc81d5` (plants), `9d89acf45` (plant line fix), `36e4e1270` (fused-path comment: class bound 27), `9b2f88563`
(`realloc_in_place_fit_rule` follows the class below 1 KiB: 960), then handoff/report commits. Role keke `~/w/n1`
(Zen 1, 60 vCPU). Measurements on RELEASE products (`NIJI_FULL_PROFILE=1` kits, ~/wl/kits/<sha>-release). Reserved
id D-3.23: NOT written (no decision taken; return it or keep it for a follow-up).

## Result
The rule (eight per doubling from 256 to 1 KiB, four elsewhere) does NOT lower the workload kits' total cost, so per the
brief (step 3) nothing should land. It wins only on find over 200000 entries, which is outside the kits and is the
one-program case the brief excludes.

| measure (release, Zen 1) | base 7e5d8b3fa | candidate 91cfc81d5 | delta |
|---|---|---|---|
| kits static, sum of 30 rows: user instr | 96,203,247,536 | 96,204,202,604 | +955,068 (+0.001%) |
| kits static: page faults | 1,205,379 | 1,205,698 | +319 |
| kits dynamic: user instr | 114,732,246,410 | 114,738,380,704 | +6,134,294 (+0.005%) |
| kits dynamic: page faults | 2,170,531 | 2,171,163 | +632 |
| du row (10 runs), static / dynamic faults | 3,525 / 3,954 | 3,775 / 4,202 | +250 / +248 |
| du row user instr (static) | 106,776,668 | 107,771,308 | +994,640 (+0.93%) |
| ls-lR / ls-l-enUS rows, static faults | 3,314 / 3,537 | 3,346 / 3,567 | +32 / +30 |
| printf-loop dynamic user instr (20000 starts) | 19,581,477,551 | 19,586,617,181 | +257 per start |
| `--version` starts, dynamic user instr (18 rows) | | | +258..+976, median about +290 |
| `--version` starts, faults | | | within +-3 both ways (jitter) |
| find 200k (static, findutils 4.11 objects) u / faults / max RSS | 263.3M / 8,036 / 34,080 KiB | 264.8M / 7,199 / 30,720 KiB | +0.5% / -837 / -3,360 KiB |
| find 3 entries u / faults / RSS | 148,954 / 93 / 2,400 | 149,470 / 95 / 2,400 | +2 faults |
| issue #21 probe (VmSize steps, RLIMIT_AS need) | 6528/10948/35524/45764/47816, 41296 | identical | 0 |
| per call (wl_percall): malloc_free / realloc_chain | 117.2 / 294.2 | 117.2 / 294.2 | 0 |

Deterministic: user instructions and faults repeat within a few units per rep (du 3523..3530 base, 3775..3776 cand). Not
usable: kernel instructions (sort-C, untouched by this change, moved +15% between the two kit runs; printf-loop's reps
spread 15.5G..16.5G; the candidate kits ran beside my gate and plants), so the kernel column is not counted for or
against. The kits record no RSS (raw column 9 is wall time). Small-program max RSS: see "RSS" below. Only the Zen 1
class was measured (only keke assigned); Skylake-SP not measured.

## Why the model was wrong (diagnosis, measured)
The model (handoff leg 1: LD_PRELOAD size histograms of the glibc kit, per class roundup(peak live x size, 4 KiB))
predicted du -80 KiB per run and ranked 256..1K near best. Measured, du is +25 faults per run. `perf record -e
page-faults -c 1 -d` on one `du -s /usr/include /usr/share/man` (static release kit, setarch -R): every extra fault
is in the heap's first segment (du 348 -> 374, ls 98 -> 101; every other region identical). Per 256 KiB slab, the
21 base slabs keep identical page counts and the candidate adds one slab with 25 touched pages. The new class takes
its blocks from a class that keeps its pages anyway: that class's pages are reused from freed blocks, so splitting it
only adds a slab. A finer table pays only where many blocks of one size are live together (find), and costs a slab
where blocks churn. Not verified: the cause of the dynamic +290 instructions per start (candidates: the per-class
work at cache creation over 52 classes instead of 44, and 512 B more cache words).

## Other rules
Only this rule was measured on real products. The model's ranking of the others (e8 from 128, caps 512/2K/4K, two per
doubling) rests on the same per-class-peak assumption that missed du by ~45 pages per run, so it is not evidence that
any of them would win. Per the RUT RULE a second spacing tweak is not tried here.

## Proof run on the final code head (9b2f88563)
Gate (direct tier) `heap-class-spacing-gate3`: 10 suites ok, 1456 passed, 0 failed, conformance 9 passed, peak 11G,
`affected: ok`. Plants `heap-class-spacing-plants3`: 5 CAUGHT, 0 SURVIVED, no WARN. Plants (`handoff/HEAP-CLASS-SPACING-plants.txt`, 5 plants: off-by-one class boundary, old
table restored, table rule, aligned index, above-1 KiB arithmetic). Per call on the final head: `heap-class-spacing-percall2`. Data on 91cfc81d5 (the same code
apart from a comment and the test; run from ~/wl/src copies, so precheck cannot judge them): `-kits`, `-percall`,
`-findrel`, diagnostics `-dufaults`, `-smallrss`; base: `-kits-base`, `-find-base2` (keke). Earlier: plants2 5/5 CAUGHT on
36e4e1270; gate2 red on `realloc_in_place_fit_rule` (it hard-coded 896 as the class below 1 KiB; fixed in 9b2f88563).
Not run (DO NOT LAND): `cargo xtask test`, full-profile hot_text_products, layout-bins diff, D-3.23.

## RSS
Max RSS (`/usr/bin/time %M`, one run each, this host reports it in 480 KiB steps), static / dynamic release kits, base
-> candidate: `--version` of bash, cut, du, gawk, grep, ls, sed, sort, tar and `du -s`, `ls -lR` over the kit tree:
4 of 22 rows one step higher (bash static 1920 -> 2400, sort static 1920 -> 2400, du -s static 3360 -> 3840, gawk
dynamic 2880 -> 3360), none lower. find 200k: 34,080 -> 30,720 KiB. Class tables: CLASSES, MAGIC and BLOCKS +64 B each
(8 entries), SMALL_CLASS unchanged (65 B); each thread cache +512 B (CACHE_WORDS 384 -> 448). Fused realloc path:
classes up to `FORECAST_CAP` are indexes 0..27, still below 32, so the mask with 31 and `CLASSES[31] >= FORECAST_CAP`
hold unchanged.

## Next step (recommendation)
Do not land. Return D-3.23. If the find case matters, rethink rather than retune: measure per-class touched pages
with the fault-address method above on the kits' heaviest rows first; a design that splits a class only where many
blocks of one size are live together (one uniform rule, decided at run time) is the shape the data points to.
Scratch: keke `~/work/heap-class-spacing/` (model-table.txt, kitcmp.py/kitcmp.txt, dufaults/, findrel.sh, rel-*).
