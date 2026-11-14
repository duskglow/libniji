# LIBM-JNF-LARGE handoff (run 3, LIBM-JNF-LARGE-3: the exhaustive sweep in 16 shards)

State: branch `task/libm-jnf-large-3` (carries runs 1 and 2), base origin/next 630f3afb2. Code head c605a3f05 (later
commits handoff/ only); product code unchanged since 833dd6654 (run 2). Role kanon:~/w/kn1 (Broadwell-EP, Xeon
E5-2699 v4, 5 slots x 7 CPUs). Briefs: ~/state/briefs/LIBM-JNF-LARGE.md (the change), -2.md, -3.md. Decisions in force:
D-10.109 (large band); pending record records/pending/LIBM-JNF-LARGE-1.toml (D-NEW, series D-10: the coordinator
assigns the id).

## The change (runs 1-2, unchanged here)
`special::jn` / `special::yn`, `float` only: on the recurrence tier's refusal one out-of-line `#[cold]` call
`jn_low` / `yn_low` sends orders 0, ±1 to `bessel32_rest` (the large band, its table, then the accurate path) or to
`j0`/`j1`/`y0`/`y1` (zeros, subnormals, inf, NaN); |n| > 1 keeps the accurate path. Every typical row equal to next
(run 2's counts below); band rows 1.78e5 -> 460-487 instr/call (Zen 1, run 2).

## Run 3: the sweep (why runs 1-2 went red)
`jnf_ynf_low_orders_are_the_order_functions_exhaustively` (every float, four modes, 10 calls each) is ~5.6 h on one
7-CPU slot (kanon: every 64th encoding in 314 s, job -size64): over the 150-min job limit (run 2's -sweep5 rc 124,
run 1's role released mid-run). Not a code fault.
Shard plan (in the code, `fast32::bessel::tests`): `LOW_ORDER_SHARDS = 16`, shard k = encodings
`k·2^28 .. (k+1)·2^28` (`low_order_shard_range`), tests `jnf_ynf_low_orders_are_the_order_functions_exhaustively_00`
.. `_15` (each `#[ignore]`, one long-sweeps Exhaustive row and one job each), through the new
`fast32::tests::exhaustive_range_by(start, end, check)` (exhaustive_by is its 0..2^32 case; its coverage assert is
per range). `low_order_shards_cover_every_encoding_once` (ordinary unit test) reads bessel.rs and checks the shard
tests run 0..15 once each and the ranges tile 0..2^32 without gap or overlap. Measured per shard at 1/64
(-shardsize64): shards 00-07 (x >= +0) 23-24 s, 08-15 (x <= -0) 14 s, so full ~26 / ~15 min on kanon: 6-10x inside the
limit. Each shard job runs `cargo xtask long-sweeps --group exhaustive --only <full test name>` (the runner fails a row
that ran other than one test). Scripts: kanon:~/work/LIBM-JNF-LARGE-3/ (shard.sh NN, final.sh, lint.sh).
Plants: handoff/LIBM-JNF-LARGE-3-plants.txt, 7 (shard number, shard count, range gap, range end, range start/end in
exhaustive_range_by, the sweep's own value check); run 2's 13 product plants (handoff/LIBM-JNF-LARGE-plants.txt) stand.

## Jobs (kanon), code head c605a3f05
libm-jnf-large-3-lint3 rc 0 (xtask clippy, all-features clippy, static); -percall1 rc 0 and -percall1-sse2 rc 0
(every Liella row identical to run 2's final column: jnf0_large 459.5 ... ynf1_mid 291.6, next 630f3afb2 equal on the
typical rows; the MISMATCH lines are glibc's values against Liella's, as in run 2); -gate1 rc 0 (affected --run
--base origin/next, peak 2.7G); -plants1 rc 0 (7 CAUGHT, 0 WARN). Shards -shard00 .. -shard15, test seconds under
load 30-35 of 35 CPUs (long-sweep lines), ALL 16 rc 0, `ok`: 00 1963, 01 1955, 02 1886, 03 2309, 04 2000, 05 2053,
06 2153, 07 2082, 08 1187, 09 1026, 10 1165, 11 1410, 12 1379, 13 1219, 14 1057, 15 1408 (max 38.5 min).
The whole exhaustive sweep therefore passed on c605a3f05: every float, four modes.
Superseded: -lint0 (inherited head, rc 0, before the change), -size64 and -shardsize64 (sizing, rc 0), -lint1 (rc 1:
clippy::panic in the tiling test), -lint2 (rc 1: the long-sweeps scan read the literal `#[ignore]` in it). Queued
shards 00, 02, 05, 06, 08, 09, 10, 14 and -percall1-sse2 were stopped before they started (no jobs.tsv row) and
requeued behind the gate and plants under the same labels.

## Run 2's counts (wl_percall instr/call, Zen 1, base next 630f3afb2)
jnf0_large 175480.4 -> 459.5 (glibc 329.2), jnf1_large 178368.1 -> 466.0, jnfm1_large 178370.1 -> 470.0, ynf0_large
175474.4 -> 479.0, ynf1_large 178367.1 -> 486.5; jnf0_mid 268.3, ynf1_mid 291.6, jnf3_mid 2747.1, ynf3_mid 2411.6
equal to next; jnf3_large +1.2, ynf3_large +3.0 (accepted cost, 0.003%/0.008%: n = 3 refusals pass the cold call).

## Not to redo
The single-test sweep as one job (5.6 h). Any test of |n| <= 1 ahead of the tier or inside `yn` (cost typical rows
2-4 instructions in run 1/2).

## Next step
1. Follow-up story for n = 2, 3 on the band: jnf(3)/ynf(3) at |x| >= 2^19 ~35,850 instr/call vs glibc ~690 (52x).
Analysis (DLMF 10.18.17-18, μ = 4n²): phase truncation inside LG_GABS for n <= 3 at 2^19 (n = 4 needs the x^-5
term); modulus W = (μ−1)/8; D1 = (μ−1)/(4π), D3 = (μ−1)(μ−25)/(192π). BLOCKER: `dfix = ((dq·2^84) as i64) << 42`
needs |dq| < 2^-21, at 2^19 |dq| reaches ~2^-18.7 (n = 2) / ~2^-17.5 (n = 3): needs a two-part conversion or a
coarser scale. Quadrant shift (n + [Y]) mod 4; route by folding the band test into the tier's `!x.is_normal()` test.
2. Optionally give the 16 shard rows `.secs()` from the shard jobs' `long-sweep: ... ok <s>s` lines.
