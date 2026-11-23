# WL-COUNT-HONEST-CHEAP: page-fault noise in the workload count, holds, the count's cost

Branch `task/wl-count-honest-cheap` (from origin/next 8e34f9c72). Role keke:~/w/n1 (Zen 1 host, 60 vCPU, two NUMA
nodes, automatic NUMA balancing on, THP madvise-only). Decision D-X.151 (reserved, written; amends D-X.141).
Plants: `handoff/WL-COUNT-HONEST-CHEAP-plants.txt` (11). Report json: `handoff/reports/WL-COUNT-HONEST-CHEAP.json`.
Probe and repeat data on keke: `~/work/wl-count-honest-cheap/{noise1,noise2,noise3,rep-a,rep-b,rep-c}.tsv`.

## 1. Noise: cause and fix (probe-settled)

Probes: the count's own command lines, glibc and next's dynamic kit, `perf stat -e page-faults,minor-faults,
major-faults,task-clock`, 5..25 runs per condition, conditions interleaved run by run.
- **ASLR is most of it**: the stack, heap and mapping bases cross page edges differently each run. seq glibc
  136..142 / libniji 165..170 with ASLR, 133 / 163 in every run under `setarch -R`; ls-lR 3148..3158 vs 3132;
  printf-loop (20000 starts) about +41000 faults for glibc with ASLR.
- **Left over: rare positive excursions** on runs of 2 s or more (gzip-c glibc 403 once in 10 against 269): minor
  faults only, no THP, NOT removed by `numactl --cpunodebind --membind` (either node) or `--preferred`, so most
  likely NUMA balancing's time-driven hinting faults (not proved). They only add faults.
- Ruled out: page cache state (no major faults in any outlier), THP/khugepaged, run order.
- **Fix (D-X.151)**: every measured run of every column under `setarch -R`; a row's faults = the LOWEST of its 5
  runs. `WLC_ASLR=1` keeps the method before (cached apart, `.q.tsv` vs `.p.tsv`).

## 2. Repeat counts, 5 before / 5 after (jobs rep-a/b/c; 25 runs per condition, groups of 5 = one count)

Before = ASLR + median of 5; after = setarch -R + lowest of 5. Range over the 5 counts (libniji / glibc), next 8e34f9c72:

| workload | before libniji | before glibc | after libniji | after glibc |
|---|---|---|---|---|
| ls-l-enUS | 3910..3911 | 3133..3135 | 3903 x5 | 3113 x5 |
| md5sum | 175..176 | 147..148 | 172 x5 | 143 x5 |
| cut-tr-uniq | 583..591 | 488..492 | 578 x5 | 477 x5 |
| seq | 160..161 | 136..138 | 157 x5 | 133 x5 |
| printf-loop | 1895023..1895493 | 1481596..1482076 | 1880131..1880178 | 1440261..1440700 |
| ls-lR | 3689..3692 | 3151..3156 | 3683 x5 | 3132 x5 |
| du | 3935..3936 | 3357..3361 | 3922 x5 | 3332 x5 |
| sed-s | 277..279 | 218..220 | 273 x5 | 213 x5 |
| sed-utf8 | 275..277 | 230..231 | 271 x5 | 225 x5 |
| bash-loop | 255..256 | 193..196 | 252 x5 | 191 x5 |
| grep-F | 485..488 | 427..430 | 480 x5 | 420 x5 |
| grep-E | 499..503 | 435..439 | 493 x5 | 428 x5 |
| grep-i-utf8 | 507..510 | 454..457 | 501 x5 | 448 x5 |
| gawk-sum | 297..300 | 249..252 | 294 x5 | 245 x5 |
| gawk-regex | 305..307 | 251..254 | 301 x5 | 246 x5 |
| gawk-utf8 | 351..354 | 305..308 | 348 x5 | 301 x5 |
| gawk-printf | 296 x5 | 247..275 | 291 x5 | 241 x5 |
| tar-c | 479..483 | 410..411 | 473 x5 | 402 x5 |
| tar-tv | 276..278 | 234..237 | 273 x5 | 230 x5 |
| gzip-c | 295..320 | 273..304 | 292 x5 | 269 x5 |
| gzip-d | 245..246 | 216..218 | 243 x5 | 212 x5 |
| pipe-mix | 1288..1300 | 1132..1139 | 1280..1288 | 1121 x5 |
| tar-gz | 624..753 | 532..545 | 618..626 | 524 x5 |

Stated bound after, on one unchanged tree in one process environment: 0 for every single-process row; pipe-mix and
tar-gz 8 (0.6%, 1.3%); printf-loop 47 / 439 (0.003% / 0.03%).

**Environment caveat (found at the end, NOT fixed):** in one address layout the size of the process environment
decides where the stack starts, so a different environment moves a row by about one fault per process. A real
wl_count (new tool, job sub-new-dyn) gave seq 158 / 133 = 1.1880 where the probes gave 157 / 133 (printf-loop 1.2872
vs 1.3055). Each environment repeats exactly; two invocations need not have the same environment.

**Holds:** only gzip-c's hold is dropped (1.7578; 292 / 269 = 1.0855 in the probes and the real count, about 4 faults
under 1.10). The other 19 levels the probes would allow lower (bash-loop 1.5205 -> 1.3194, ls-l-enUS 1.3069 ->
1.2538, ...) are NOT applied: with the environment effect a real count could exceed them (seq would have).
`wl_verdict.py holds --base <the file before>`: 23 rows ok.

### RULING NEEDED: three levels below what next measures

sed-s (level 1.2112; next 273 / 213 = 1.2817), gawk-sum (1.1993; 294 / 245 = 1.2000), gawk-printf (1.2009; 291 / 241 =
1.2075). The recorded glibc numbers (251, 251, 249) are above anything glibc measured here in 50 runs per row (213,
245, 241; with ASLR 218..225, 249..252, 244..275); the recorded revision 189ab9cbc measures sed-s 278 / 213 = 1.3052.
Under the method before, the verdict on these rows depends on noise: the old-tool count of this story passed sed-s
because glibc's median came out 279 (runs 218..302). With this branch a dynamic count of next FAILS sed-s (seen: job
sub-new-dyn) and, by the probe numbers, gawk-sum and gawk-printf. Recommend: re-record the three from counts in a
fixed environment (next step) as a correction of a noise-inflated recording, owner to confirm since the numbers rise.

## 3. Cost

Like for like (keke, same tree 8e34f9c72, products cached, all three started together, load 10..20; 5 workloads
seq md5sum printf-loop gzip-c sed-s, COUNT_FRESH=1, base = new so glibc + two libniji columns measured):

| count | wall | CPU user+sys | file-system writes |
|---|---|---|---|
| before, dynamic (sub-old-dyn) | 3434 s | 990 + 1949 s | 1.13 GB |
| before, static (sub-old-static) | 2698 s | 791 + 1523 s | 0.86 GB |
| after, dynamic (sub-new-dyn) | 2288 s | 721 + 1339 s | 0.74 GB |

After vs before (dynamic): wall -33%, CPU -30%, writes -34%; the dynamic count is now cheaper than the static one was.
Per step (after, from the new `wl_phase`/`wl_cost` lines): printf-loop 655..707 s per column (perf 250, md5 30,
strace 366..430 in 2 passes, about 200 s per pass; before: clone pass + 3 passes); every other row under 50 s; threaded
sort rows still need 3 strace passes (futex differs), ls-l-enUS 2..3. Full 30-workload counts (3 started together,
stopped at 17:03): all three at 9 tables, inside printf-loop, after about 40 min of counting.
- The 8825 s count: two cold product builds (about 20 min each here; one build serves both links when it runs first)
  plus fresh new and base columns with printf-loop's strace passes and full-row re-measures on fault differences.
- The "410 s static" figure was not a measured count: WL-DYNAMIC-KIT-report.md computed it as about 10 passes x 40.9 s
  for ONE libniji column, glibc cached, base column and builds not included.
- Considered, rejected: a shared cargo target across `~/wl/src/<sha>` trees (git archive sets mtimes to commit
  time, so cargo could reuse stale objects: a wrong result); fewer printf-loop starts (changes the workload and its
  held levels).

## Jobs (keke)

Final head: `wl-count-honest-cheap-gate2` (direct tier), `wl-count-honest-cheap-plants2` (11 plants). Earlier head
dbe683bb5 (same code): gate (rc 0), plants (11 CAUGHT). Evidence: noise1..3, rep-a/b/c, sub-old-dyn, sub-old-static,
sub-new-dyn (rc 1: the verdict's sed-s FAIL above). Stopped by me: cost-old-dyn2/static2 (role busy),
cost-old-dyn3/static3/new-dyn (full counts, would not end in time); cost-old-dyn/static failed at start (base not
primed for 189ab9cbc). Clippy -D warnings on the role: rc 0.

## Open / informational

- `validation/wl_stats_plants.sh`: plants `total_keeps_mtmem` and `total_keeps_futex` (wl_syscls.sh, untouched here)
  report "BAD-PLANT (no change)" on origin/next too: their patterns no longer match.
- `cargo xtask plants` rebuilds the products for each plant on a `validation/` script (about 70 s each, 18 min for 11).
- The first full counts showed a `-release.build.log` created a minute after the `-release-dyn` one for the same sha
  while both jobs should have waited on one lock; not reproduced (the later runs logged a 579 s lock wait). Unverified.

## Next step

Run measured commands in a fixed environment (`env -i` with a fixed PATH/HOME/LANG set, in `wl_run_prefix`), then
5 real `COUNT_FRESH=1` counts of next (the 23 held rows) and re-record the levels from them (MAY-TIGHTEN; the three
over-level rows per the ruling). Plan rows: D-X.141, D-X.151.
