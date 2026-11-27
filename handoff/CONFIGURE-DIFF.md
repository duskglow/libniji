# CONFIGURE-DIFF handoff

## State
* Branch `task/configure-diff` from origin/next 6295b2301; handoff/ only (no libniji code, header or record change).
* Tool: `handoff/configure-diff/` (README: files, run line, outputs, limits). Results of the 2026-11-27.2 run:
  `handoff/configure-diff/results-20261127.2/` (summary, diff, noise, stock-vs-stock summary).
* Report: `handoff/CONFIGURE-DIFF-report.md` (totals, the libniji-class table L1-L15, behaviour B1/R1-R6, harness,
  sanity vs Yohane's 27.2 report, rulings).

## Runs (mei, role s1; data in /mnt/zram/claude/configure-diff/, job copies in ~/work/CONFIGURE-DIFF/t1..t3)
* `configure-diff-pilot2` (10 packages, tool t1), `configure-diff-full1` (438 Aqours stage-2 packages, tool t2,
  -j 20, about 95 min), `configure-diff-retry1` (the 15 whose chroot never started under load, tool t3 with the
  retry; all 15 passed first try), `configure-diff-diag1` (byacc alone: passes; the failure was transient).
* Superseded: `configure-diff-pilot1` (the keyring under ~ was not readable in the user namespace; fixed by copying
  it into the run dir).

## Decisions in force
None new. Recorded differences seen: fflush after ungetc, SIGRTMIN (D-20.1), gcvt digits (D-8.10), rwlock order,
iconv EUC-CN, dlmopen deferral (DL-22).

## Open questions (RULING NEEDED, recommendations in the report)
* B1 nested `abort()` from a SIGABRT handler (libsodium's configure): record or match.
* L14 `__libc_enable_secure` (GLIBC_PRIVATE): recommend not exporting.
* L13 `_BSD_SOURCE` deprecation `#warning`, L15 `SIGTSTP` text in `<signal.h>`: recommend matching.

## Not to redo
* gcc-14 must run libstdc++-v3's configure with `-nostdinc++`; without it libstdc++'s own `<math.h>` wrapper hides L1.
* `<printf.h>` `PA_*` is not visible to configure probes in this package set (compile-time stop).

## Exact next step
Header story takes L1-L13 as rows; snapshot stories rerun `cdiff.sh` (README "Run") with their debs and compare
`diff.tsv` with this run's: expect only the recorded behaviour and harness rows.
