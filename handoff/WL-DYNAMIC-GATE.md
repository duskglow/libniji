# WL-DYNAMIC-GATE: the INVIOLABLE workload rule binds the dynamic kit (owner ruling #6965)

Branch `task/wl-dynamic-gate` (stacked on WL-DYNAMIC-KIT). Role keke:~/w/n3 (Zen 1 host, CPU class "AMD EPYC 7551
32-Core Processor"). Decision D-X.141 (reserved, written). Plants file: `handoff/WL-DYNAMIC-GATE-plants.txt` (5 plants).
Report json: `handoff/reports/WL-DYNAMIC-GATE.json`.

## Done

- Leg 1: `WL_LINK` defaults to dynamic (`validation/wl_lock.sh`; one `cargo xtask so` per sha caches both product sets);
  `validation/wl_verdict.py` (1.10x per row in instructions:u and page faults, geomean instructions:u <= 1.00 on a full
  run, held rows at their level, HOLD-RELEASE / MAY-TIGHTEN / HELD-UNRECORDED; `holds` ratchet against the merge base);
  `wl_count.sh` fails on the verdict for a dynamic release run; `wl_kits.sh` nightly/RC static sweep reports the rule
  for both kits; `wl_gate.sh` (timing hosts) pinned static; validation/README.md, D-X.141; self-tests.
- Leg 2:
  - The count (all 30 workloads, dynamic kit, new = branch 7a5d6fc7d, base = next 189ab9cbc) ended rc 0.
  - `validation/wl_holds.tsv`: 24 rows for the Zen 1 class. Levels come from the BASE column (next). The branch's
    new column is the older merge base's libc, so it is not next: printf-loop pf 2074829 against next's 2035209, ranges
    disjoint. Each level is the worst ratio observed on next: instructions:u next's mean over glibc's mean; page
    faults next's highest run (min..max, plus a "first measurement" note) over glibc's lowest run. That way a rerun
    of next cannot exceed a level (no flakes). The holds header and D-X.141 say so.
  - `wl_verdict.py`: a held page-fault row drops its hold only when its worst run is in bound too. A release on the
    median alone flakes back: sed-s faults run 283..322 for libniji and 251..280 for glibc. Test 3b and plant
    `pf-release-median` cover it.
  - Checks:
    - `wl_verdict.py holds`: 24 rows ok.
    - Next's column judged against the holds: PASS, geomean instructions:u 0.8958, page faults 1.1343 (advisory).
    - The branch's new column FAILs 4 pf rows (printf-loop, cut-tr-uniq, du, gawk-sum), on the older libc. The
      train measures the merge with next.
    - `wl_verdict_test.sh`: 38 passed (local).

Held (Zen 1 class, level = libniji/glibc): printf-loop instr 3.6309; pf: ls-l-enUS 1.3069, md5sum 1.2449, cut-tr-uniq
1.2609, seq 1.2353, printf-loop 1.3732, ls-lR 1.2253, du 1.1956, sed-s 1.2112, sed-utf8 1.2457, bash-loop 1.5205,
grep-F 1.1659, grep-E 1.1875, grep-i-utf8 1.1360, gawk-sum 1.1993, gawk-regex 1.2311, gawk-utf8 1.1711, gawk-printf
1.2009, tar-c 1.2108, tar-tv 1.2256, gzip-c 1.7578, gzip-d 1.1907, pipe-mix 1.1778, tar-gz 1.2204. Skylake-SP rows
not added (no count from that class on next in this run; the first count there records them, HELD-UNRECORDED until).

## Cost of the per-gate count (brief item 1)

The count took 8825 s wall, added 4.6 GB under ~/wl (28.8 M blocks written), with keke at load 18-30. It included
two fresh product builds (39 min) and printf-loop's dynamic strace passes (about 45 min: 20000 dynamic starts under
`strace -f`). Before: WL-DYNAMIC-KIT's static count took about 410 s per full count, plus 171.8 s for a second build,
on a quieter host. NOT VERIFIED like for like. Build cost per sha is unchanged by design (one `cargo xtask so` serves
both kits); a dynamic count's strace passes cost more than a static one's.

## Jobs (leg 2, keke)

`wl-dynamic-gate-count-dyn` (rc 0). `wl-dynamic-gate-lint1` (rc 0, head 4a2b2f3d9). On the final head, synced:
`wl-dynamic-gate-gate2` (gate: clippy -D warnings, then `cargo xtask affected --run --base origin/next`; direct tier =
xtask unit tests, static, wl_lock/wl_stats/wl_verdict self-tests) and `wl-dynamic-gate-plants2` (5 plants). Killed
while still queued, before they ran (they blocked the sync): lint2, gate1, plants1.

## Open / informational

- gzip-c faults vary 306..566 (libniji) and 322..407 (glibc) within one count, so its level of 1.7578 is a weak
  ratchet. sed-s and gawk-printf are noisy too. Recommend a story to find the fault noise in these workloads; then
  tighten (MAY-TIGHTEN).
- The nightly `wl_kits.sh` schedule belongs to the coordinator (ops cron). The INVIOLABLE line in thread_common.txt
  (ops repo, read-only here) should name the dynamic kit and ruling #6965.

## Next step

If gate2/plants2 are pending: `~/bin/jobwait wl-dynamic-gate-gate2 wl-dynamic-gate-plants2` on keke. Then
`cargo xtask precheck --jobs wl-dynamic-gate-gate2,wl-dynamic-gate-plants2 --head <sha>` and handback-check.
