# PLANTS-WARM-COPY: one warm plants copy per role

Branch `task/plants-warm-copy` from origin/next 6d0c3e0b6. Code head f781a4036 (this note and the results follow it).
Rules: D-X.180, `docs/plants.md` ("Warm copies"), `xtask/src/plants.rs` (`sync_warm`, `prune_to_cargo_cache`,
`take_warm`, `default_warm`, `auto_copies`).

## What changed

- Copy `k` runs in `<warm>/copy-<k>` when its `flock` (`copy-<k>.lock`) is free, else cold in `--dir` as before (also
  when pruning or the sync fails: never a red). `<warm>` = `--warm <dir>`, else `plants-warm/<tree name>` in `--dir`'s
  grandparent (`/mnt/zram/claude/<story>/plants-<n>` in role `sh1` gives `/mnt/zram/claude/plants-warm/sh1`); `--cold`
  for none. Logs and `results.txt` stay in `--dir`, which must still be new.
- Before use: `target/` and `fuzz/target/` pruned to cargo's cache (profile dirs' `.fingerprint build deps examples
  incremental`, nested target dirs alike, `CACHEDIR.TAG`); `rsync -rlpDc --delete` (content compared, no times kept:
  a changed file gets a fresh mtime); a `--dry-run` pass must then list nothing.
- `--copies auto`: also two plants or more, and 8 CPUs or more or 6 plants or more.

## Proof (shiki, Haswell E5-2697 v3, 26 CPUs; one copy per run unless named; same tree)

Verdicts, cold (`--cold`) against warm, per plant: **identical on all 5 files**: GAI-ADDRCONFIG-LOOPBACK (4 CAUGHT)
and XTASK-PRECHECK-SKIP-ASM-AFFECTED (6 CAUGHT) on zram; AFFECTED-LAYOUT-ROWS (7 CAUGHT, 2 comment lines), HEADERS-STAGE2-GAPS-J
(8 CAUGHT, 1 SURVIVED both ways: on today's head) and HEADERS-STAGE2-GAPS-K (10 CAUGHT) on tmpfs. XTASK-PRECHECK on 4
CPUs, warm, 1 and 2 copies: 6 CAUGHT each.

Left mutant (job `plants-warm-copy-measure-a2`): alr-length-not-literal applied in the warm copy, built and tested
there by its own command (rc 101), backdated to 2000-01-01; the next warm run rewrote it (`64 files written`: the
mutant and `__pycache__` files, below), its baseline passed and its verdicts equal the cold run's. Unit tests: the
same on a toy (`a_warm_copy_is_synced_by_content_and_never_tests_a_left_mutant`), the lock
(`a_busy_warm_copy_gives_a_cold_copy`). Plants: 11/11 CAUGHT (handoff/PLANTS-WARM-COPY-plants.txt).

Wall per job, s (cold -> warm). "first" = the warm root's first run (empty, so cold-like):

| file | fs | cold | warm | baseline `so` cold -> warm |
|---|---|---|---|---|
| XTASK-PRECHECK-SKIP-ASM-AFFECTED | zram | 1478 | 1288 | 416 -> 324 |
| HEADERS-STAGE2-GAPS-J | tmpfs | 357 | 357 | none |
| HEADERS-STAGE2-GAPS-K | tmpfs | 324 | 250 | none |
| AFFECTED-LAYOUT-ROWS (after the left mutant) | tmpfs | 343 | 284 | none |
| this story's plants (2 copies) | zram | 400 (first) | 349 | none |
| GAI-ADDRCONFIG-LOOPBACK | zram | 1897 | 1791 (first) | 430 -> 411 |

Median of the five true warm runs 349 against 357 cold; median paired ratio 0.87 (-13%). Runs overlapped the gate,
plants and measurement B (pinned to other CPUs), so they carry noise. One host class only (the role's host).

`--copies` on 4 CPUs (`taskset -c 0-3`, warm, XTASK-PRECHECK, 6 plants): 1 copy 1433 s, 2 copies 1221 s; priming
with 2 empty copies 1883 s. So SPLIT_PLANTS = 5 (split from 6 plants); fewer plants not measured.

## Why the gain is short of the report's ~45%, and the next step

The warm baseline still costs 324 s: `early-cargo-ld` 98 s, `early-cargo-ldconfig` 90 s, `early-cargo-localedef` 91 s,
`early-builds-wait` 154 s. These are the crates carrying the build tag, and `NIJI_PLANTS_BUILD_TAG` is `plants-<pid>`,
a new value every run. **next:** a tag fixed per warm copy (for example `plants-warm-<k>`) should bring the baseline
toward the report's 79-112 s. It changes PLANTS-BUILD-2's reasoning ("a run's copy and target directories are
new"), so it needs its own story with the same warm-against-cold verdict proof. The mutant product builds (the larger
share in products files) are untouched by this story.

## Notes

- RAM held per role: 2.1 GB a warm copy with products (769 MB with xtask builds only), times the copies (2 on
  a large host); zram stores about half of that.
- `__pycache__/*.pyc` written by the header checks differ between copies (each embeds its source's mtime), so the
  warm sync rewrites them every run. Python checks each pyc against its source, so this costs time only.
- The gate's `headers-gen` rewrites `libc-include/` in the role tree with the same content; the sync skips it.
- I synced the role during job A (a test-only change to `plants.rs`); later runs saw that tree, cold and warm alike.
- Job A was stopped after its XTASK-PRECHECK pair: its tail repeated A2's files and would have run past the deadline.
