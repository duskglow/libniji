# WL-LEVELS-RERECORD: re-record the workload page-fault levels from honest counts

Branch `task/wl-levels-rerecord` (from origin/next 0299fd6e1). Role mei:~/w/s1 (Skylake-SP). keke:~/w/wllr (Zen 1,
coordinator ruling #7429) was REMOVED at the end of leg 2 (no /dev/shm scratch existed; nothing of the story runs there).
Decision: **D-X.155** (amends D-X.151), record `records/decisions/D-X/D-X.155.toml` (59f05c191 first put it in the
generated view by mistake; the next commit moves it to the record and leaves the view as on the base).

## State: done (legs 1 and 2)

- 8822525a4 fixed run environment (`validation/wl_stats.sh` `WL_RUN_ENV`, `env -i ... setarch -R`; `.r.tsv` cache).
- 927e92c21 `wl_verdict.py holds`: level must equal ceil4(libniji/glibc) of its row; a raise only as a cited correction
  (`correction D-X.<n>` in cause, new `recorded`, libniji not above the base's).
- 59f05c191 every Zen 1 level re-recorded (`validation/wl_holds.tsv`, header explains), D-X.155 row, plant
  `level-edited`.
- db9ae78d7 test fixture no longer looks like a lab address (static lab-names / private-paths were red on it).
- Jobs: counts keke `wl-levels-rerecord-k1..k5`, mei `wl-levels-rerecord-m1..m5` (all of next 0299fd6e1, fresh base and
  new); gate `wl-levels-rerecord-gate`, plants `wl-levels-rerecord-plants` (mei, dir
  `/mnt/zram/claude/wl-levels-rerecord/plants-1`). Results: report below and `handoff/reports/WL-LEVELS-RERECORD.json`.

## Repeatability (5 counts per class, page faults, lowest of 5 runs per column)

Every single-process row: one value in all 10 libniji columns (new and base) and all 5 glibc columns, on both classes.
Pipelines: Zen 1 cut-tr-uniq 584..585; Zen 1 tar-gz 629..647 in 8 columns, 758 and 765 in 2 (every run of those
columns high: positive excursions, D-X.151), glibc 523..526; Skylake-SP tar-c 465..476 / glibc 389..400.
printf-loop (Zen 1 only, k1 and k2): libniji 1960133..1960175, glibc 1440309..1440346; instr 19142786678..19142789292
/ 5203469249..5203471555.

## Before / after (Zen 1): in D-X.155 and the commit body of 59f05c191

7 raises, each a CORRECTION (stored glibc did not reproduce; libniji below its old count): sed-s 1.2112 -> 1.2972,
gawk-sum 1.1993 -> 1.2173, gawk-regex 1.2311 -> 1.2358, gawk-utf8 1.1711 -> 1.1718, gawk-printf 1.2009 -> 1.2200,
tar-gz 1.2204 -> 1.2371, printf-loop instr 3.6309 -> 3.6789. 16 tighten (bash-loop 1.5205 -> 1.3164, ls-l-enUS
1.3069 -> 1.2700, ...). None at or under 1.10.
Check of the 5 Zen 1 counts against the new levels (`wl_verdict.py check`): k1, k3, k4, k5 pass; k2 fails only tar-gz
(758 = 1.4493, an excursion). Before (old levels): every count failed 5 or 6 rows (sed-s, gawk-sum, gawk-regex,
gawk-utf8, gawk-printf; tar-gz in k5).

## Open (rulings, for the coordinator)

1. tar-gz on Zen 1 can still fail a count (1 of 5 here, under a host running 5 counts at once): lowest-of-5 does not
   remove excursions that hit all 5 runs of a 4 s pipeline. Recommend a fix-forward story in the count (more fault runs,
   or re-measure a column whose lowest is far over its pair's, for rows over ~2 s), not a looser level.
2. Skylake-SP has no recorded levels (every row reports HELD-UNRECORDED, advisory). The data to record them is in
   the m1..m5 counts (exact repeat): ls-l-enUS 3434/2654 = 1.2939, md5sum 177/145 = 1.2207, cut-tr-uniq 588/482 =
   1.2200, seq 162/135 = 1.2000, ls-lR 3214/2754 = 1.1671, sed-s 277/215 = 1.2884, sed-utf8 276/227 = 1.2159,
   bash-loop 258/197 = 1.3097, grep-F 484/424 = 1.1416, grep-E 497/432 = 1.1505, grep-i-utf8 505/452 = 1.1173,
   gawk-sum 299/247 = 1.2106, gawk-regex 306/249 = 1.2290, gawk-utf8 350/300 = 1.1667, gawk-printf 296/244 = 1.2132,
   tar-c 476/389 = 1.2237, tar-tv 280/232 = 1.2069, gzip-d 249/214 = 1.1636, pipe-mix 1293/1126 = 1.1484, tar-gz
   623/520 = 1.1981 (du 1.0615: no hold; printf-loop not counted). Recommend recording them (a new class, allowed by the
   ratchet) in a follow-up once ruled, so the class the CI gate runs on is bound too.

## Not to redo

The counts (10 jobs, ~4 h of host time). Scratch with the fetched logs and parser: `~/w/.scratch/WL-LEVELS-RERECORD/`
(`logs/kfull*.log`, `logs/mfull*.log`, `parse_counts.py`).

## Next step

If the gate or plants went red: fix what they name, rerun only that job. Otherwise land.
