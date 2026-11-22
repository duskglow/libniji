# WL-LEVELS-RERECORD-2: bind the CI gate's CPU class, and stop one high column from failing a count

Branch `task/wl-levels-rerecord-2` (from origin/next 8e1837914). Role mei:~/w/s3 (Skylake-SP). Decision **D-X.157**
(amends D-X.155), record `records/decisions/D-X/D-X.157.toml` (the generated view is left to the train).

## State

- 45dc550c9: excursion re-measure (`validation/wl_stats.sh` `wl_excursions`, `wl_long_row`, `wl_excursion_side`,
  `wl_row_lowest`; `validation/wl_count_run.sh` `xmeasure` and the call after the 2 % re-measure), tests in
  `validation/wl_stats_test.sh`, 20 Skylake-SP rows and printf-loop's "2 counts" in `validation/wl_holds.tsv`, D-X.157,
  plants `handoff/WL-LEVELS-RERECORD-2-plants.txt`.
- Jobs and results: see the report, `handoff/WL-LEVELS-RERECORD-2-report.md`.

## Decisions in force

D-X.151 (lowest of 5 runs, setarch -R), D-X.155 (fixed environment; level = ceil4 of its own counts; raise only as a
cited correction), D-X.157 (this story).

## The rule (D-X.157)

Row long = new or base column >= 4e9 instructions:u (about 2 s; deterministic). Column far off = its lowest run more than
7 % above its pair's (widest recorded normal spread 3.0 %, smallest recorded excursion 15.6 %). Then up to 3 batches of
5 runs of that column, the row keeps the lowest run of all; still over after 3 = a real difference, reported as measured.
Env overrides: WL_XPCT, WL_XROUNDS, WL_LONG_INSTR. The median method (WLC_ASLR=1) is never re-measured so.

## Open

- printf-loop on Skylake-SP: recorded from 2 counts taken here (`wl-levels-rerecord-2-m-printf`, `-m-printf2`):
  instr 3.6312, pf 1.3424.
- Not covered: an excursion on the glibc column (no pair to compare; it lowers the ratio, so it can only make a count
  lenient, never fail it), and both new and base high at once (no gap to see).

## Not to redo

The 10 counts of D-X.155 (logs fetched into the run's scratch; the Skylake-SP rows were generated from them by a script,
`skx_rows.py`, in the same scratch).

## Next step

If the gate or plants went red: fix what they name, rerun only that job. Otherwise land.
