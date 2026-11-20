# PLANTS-COPIES: `cargo xtask plants --copies <n>|auto`

State: branch `task/plants-copies` (base origin/next). Code commits 9544177aa (`--copies`, per-copy worker, merge),
658963e7e (pure `smallest_limit` and its test, docs, plants file). Plants file `handoff/PLANTS-COPIES-plants.txt`
(8 plants on the new code). Report record `handoff/reports/PLANTS-COPIES.json`.

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

None needed to land. Optional: one Skylake-SP pair (`--copies 1` and `--copies 2`, same head) to replace the PLANTS-BUILD-2 figure.
