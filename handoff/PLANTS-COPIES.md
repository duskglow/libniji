# PLANTS-COPIES: `cargo xtask plants --copies <n>|auto`

State: branch `task/plants-copies-2` (run 2, PLANTS-COPIES-2; continues `task/plants-copies`, base origin/next).
Code commits 9544177aa (`--copies`, per-copy worker, merge), 658963e7e (pure `smallest_limit` and its test, docs,
plants file), 1c37ce639 (merge of origin/next, run 2). Plants files `handoff/PLANTS-COPIES-plants.txt` (run 1, 8
plants) and `handoff/PLANTS-COPIES-2-plants.txt` (run 2, 10 plants). Report records `handoff/reports/PLANTS-COPIES.json`
and `handoff/reports/PLANTS-COPIES-2.json`. Rulings in force: #6824 (keke-only numbers; RAM limit order as below),
#6862 (auto = 2 where RAM allows).

## Run 2 (PLANTS-COPIES-2): merge of next

The last run ended in a CI conflict: TOOLING-PAPERCUTS-2 had changed the serial plants loop's messages in
`xtask/src/plants.rs`, and this branch had moved that loop into the per-copy worker `run_copy`. Merge 1c37ce639 keeps
both. The worker keeps its copy prefix (`plants[copy k]:`), its `stop()` (sets the abort flag and the exit status)
and its per-copy results. It now prints next's wording: a timed-out baseline or products rebuild names its limit and
flag (`ended`), a failed baseline names the plants whose command it ran (`baseline_plants` over this copy's share
only, so `owned` is built before the baselines), and a timeout catch names the command.

Checks on mei:~/w/s2 (Skylake-SP), head 4398ba4d9 code-identical to the final head:
- `plants-copies-2-unit`: `cargo test -p xtask plants::` 33 passed; clippy `-D warnings` and `cargo xtask static` clean.
- `plants-copies-2-xtask`: `cargo test -p xtask` 517 + 12 passed, 0 failed (the gate is FULL-escalated by
  xtask/src/main.rs, so it is the train's; this is the changed crate's tests).
- `plants-copies-2-plants`: `--copies 2`, 10 plants (the 8 PLANTS-COPIES plants plus the 2 TOOLING-PAPERCUTS plants on
  plants.rs), 10 CAUGHT, no WARN lines, 569 s, 5 plants per copy.
- Not covered by a test: the wording printed by `run_copy` (stdout only). The parts that decide it (`ended`,
  `baseline_plants`) are unit-tested and planted.

## What changed

- `xtask/src/plants.rs`: `run` is now the shared checks, then one `run_copy` worker per copy (threads, `std::thread::scope`),
  then `merge`. Plant `i` goes to copy `i % n` (`distribute`). Each copy copies the tree, builds, runs the baselines of
  its own plants' commands and then its plants, exactly the serial steps. Copy 1 uses `<dir>/src`, `<dir>/logs` (a
  serial run's paths); copy k uses `src-<k>`, `logs-<k>`, and its lines start `plants[copy k]:`. Verdicts, summary and
  `results.txt` are merged in the plants file's order. A copy that stops (refused / baseline failed / unusable copy)
  sets an abort flag the others check before each step; the exit status is the one a serial run gives (3, then 2, then 1).
- Default `--copies auto`: 2 when the RAM this job may use is at least 2 x `COPY_PEAK_BYTES` (14 GiB, from PLANTS-BUILD-2's
  largest per-copy peak, 13.6 GB on Skylake-SP), else 1. The limit is the smallest of `NIJI_PLANTS_RAM_GB` (GiB), every
  cgroup `memory.max` from the job's cgroup up to the root, and `MemAvailable`; none known means 1. It is printed.
- Unit tests (xtask, `plants::tests`): dealing, auto choice, the limit rule, option parsing, a 2-copy run whose
  verdicts, exit status and `results.txt` equal the serial run's with a SURVIVED in the second copy, and a baseline
  failing in one copy giving exit 2.

## Measurement (Zen 1 host, 60 vCPU, each job in one 4-core slot; serial and 2 copies started together, same head, same plants file `handoff/PLANTS-BUILD-2-plants.txt`, 12 plants; host load 11-36)

| run | wall s | CPU s | jobs.tsv mem_peak | anon peak | verdicts |
|---|---|---|---|---|---|
| `--copies 1` (serial), label plants-copies-k-c1 | 3262 | 7613 | 2.41 GB | 1.78 GB | 12 CAUGHT |
| `--copies 2`, label plants-copies-k-c2 | 2789 (-14.5%) | 8574 | 4.15 GB | 2.89 GB | 12 CAUGHT |

`results.txt` of the two runs is byte-identical. The baseline `cargo xtask so` took 377 s serial and 564 / 574 s in the two
copies (the two builds share the slot's 4 cores). Each copy's tree under `/dev/shm` was 576 MB at the end.
Skylake-SP: not measured in this run (coordinator ruling #6824: no Skylake-SP role); PLANTS-BUILD-2 (#6778) measured 12 plants
1591 s serial vs 860 s with two copies (copy peaks 13.6 and 9.2 GB, serial 19.8 GB).

## Checks on the code head (658963e7e)

- `plants-copies-unit3`: xtask `plants::` 32 passed; `cargo clippy --workspace --all-targets --all-features -D warnings`
  clean; `cargo xtask static` clean.
- `plants-copies-plants`: 8 plants, 8 CAUGHT, no WARN lines (run itself with `--copies auto` = 2, 1749 s).

## Open questions

- RULING NEEDED: the Zen 1 gain (14.5% in a 4-core slot) is about half of PLANTS-BUILD-2's 30% there. Two copies in one
  4-core slot mostly share the CPU; the gain comes from overlapping one copy's test and wait time with the other's
  builds. Recommendation: keep `auto` = 2 where RAM allows (it never measured slower, and verdicts are identical), and
  consider giving plants jobs a larger slot (SLOT_SIZE=L) on the Zen 1 host when copies > 1. That is an ops change.
- `COPY_PEAK_BYTES` (14 GiB) is far above this run's Zen 1 peaks (about 2 GB per copy by memory.peak). It is kept
  conservative until a Skylake-SP rerun on the current head confirms its peak.

## Next step

None needed to land (run 2 merged next; nothing else is open). Do not redo the merge or the Zen 1 measurement. Optional: one Skylake-SP pair (`--copies 1` and `--copies 2`, same head) to replace the PLANTS-BUILD-2 figure.
