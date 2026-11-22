# WL-LEVELS-RERECORD-2 report

## Commits

- 45dc550c9 Measure a long row's far-off column again; record Skylake-SP levels (D-X.157).
- the following commit: printf-loop's Skylake-SP rows from 2 counts, handoff and report.

## 1. Skylake-SP levels (Intel Xeon Platinum 8151)

Source: the 5 Skylake-SP counts of D-X.155 (jobs `wl-levels-rerecord-m1..m5`, next 0299fd6e1, base = rev, COUNT_FRESH=1;
full logs `~/logs/wlc-0299fd6e1-*.log` on the host). The rows were generated from the logs by a script (scratch
`skx_rows.py`): libniji = highest of the 10 libniji columns, glibc = lowest of the 5 glibc columns, level = ceil4.
They equal the predecessor's table. Every row repeated exactly over the 5 counts except tar-c (libniji 465..476, glibc
389..400 -> 476 / 389 = 1.2237). du 3284 / 3094 = 1.0615 and every instructions:u row (highest grep-F 1.0386): no hold.

printf-loop was not in those counts. Two counts of it were taken here (`wl-levels-rerecord-2-m-printf`,
`-m-printf2`, same revision and method). Count 1: instr new 19131965553, base 19131965450, glibc 5268875467; pf new
1960127, base 1960128, glibc 1460197 (runs 1460197..1460405). Count 2: instr new 19131966196, base 19131967207, glibc
5268874366; pf new 1960129, base 1960130, glibc 1460184 (runs 1460184..1460308). Rows: instr 19131967207 / 5268874366
= 3.6312, pf 1960130 / 1460184 = 1.3424; both counts PASS them. Why 2 and not 1: with one count the page-fault level's
rounding margin (about 44 faults) was no wider than the 37..42-fault spread printf-loop showed between Zen 1 counts.

`wl_verdict.py check` of m1..m5 against the new file: all PASS, 0 HELD-UNRECORDED, 1 MAY-TIGHTEN each (tar-c, whose
level takes the highest column). Zen 1 k1, k3, k4, k5: PASS; k2: FAIL tar-gz 758 (the old log, measured before the
re-measure existed; the replay below shows the new count's outcome).

## 2. Excursion re-measure (D-X.157)

Rule, threshold derivation and cost: D-X.157 and the comment in `validation/wl_stats.sh`. The k2 tar-gz case and the
planted regression are tests in `validation/wl_stats_test.sh` (replay with a stub batch, real `wl_verdict.py check`
against the real holds file):
- k2 as recorded (new 758, base 629, glibc 523): FAIL at the unchanged level 1.2371.
- new column measured again, batch 631..655: one batch, new 631, PASS at 1.2371 (`k2_level_unchanged` pins the row).
- planted regression of that size (every batch 758..786): 3 batches, still 758, note "still over 7 % after 3 batches:
  reported as measured", FAIL tar-gz pf 1.4493.
Limits: the stub batch value in the passing case is a value from the normal range of the other 8 columns, not a new
measurement taken under the same load: the method's effect on a real excursion is shown by the next loaded count.

## 3. printf-loop counts

Zen 1 rows: `recorded` ends "; 2 counts". Skylake-SP rows: "; 2 counts" as well.

## Jobs, gate, plants

All on mei:~/w/s3 (Skylake-SP), head 719512dba (later commits: handoff files only).
- Gate `wl-levels-rerecord-2-gate` (direct tier, `cargo xtask affected --run --base origin/next`): 6 suites, 619
  passed, 0 failed; peak 1.8G, rc 0.
- Plants `wl-levels-rerecord-2-plants` (`handoff/WL-LEVELS-RERECORD-2-plants.txt`, dir
  /mnt/zram/claude/wl-levels-rerecord-2/plants-1): 9 CAUGHT, 0 SURVIVED, 0 WARN.
- Counts `wl-levels-rerecord-2-m-printf`, `-m-printf2` (printf-loop, rc 0).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo xtask static`: clean (foreground).
- Locally: wl_stats_test 91 passed; wl_verdict_test 46 passed; `wl_verdict.py holds` against origin/next's copy: ok.

## INFORMATIONAL

- The brief says du gets no hold "as on Zen 1"; on Zen 1 du does have a hold (1.1891). On Skylake-SP du measures
  1.0615, under 1.10, so it gets none there by the rule.
- An excursion on the glibc column is not re-measured (no pair); it can only make a count lenient.
