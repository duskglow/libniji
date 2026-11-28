# PLANTS-WARM-COPY-2: one build tag per warm plants copy

Branch `task/plants-warm-copy-2`, stacked on `task/plants-warm-copy` (6404aac03, which already carried the fix for the
last run's red: D-X.180 as a record, the view untouched). Code commit 5cb975f3e (so.rs left unchanged by b75aa23d8: a
doc line there escalated the gate to FULL). Rules: D-X.182 (amends D-X.180), `docs/plants.md` step 4, `plants.rs`
`warm_build_tag`.

## What changed

`NIJI_PLANTS_BUILD_TAG` is `plants-warm-<k>` in warm copy `k` in every run (was `plants-<pid>`, new each run); a cold
copy keeps `plants-<pid>`. The run prints `build tag <tag>`. `run_logged` takes the copy's tag. Test: the existing
`every_build_of_a_run_gets_one_build_tag` also runs two warm runs over two copies (every build of copy k gets
`warm_build_tag(k)`, the two copies differ, no pid in it). Plants 5/5 CAUGHT (handoff/PLANTS-WARM-COPY-2-plants.txt).

## Proof (mei, Xeon Platinum 8151; 12 logical CPUs a slot; zram; one copy; same tree; three slots at once)

Slots: cold (`--cold`, new code, CPUs 12-17,36-41), warm with the per-run tag (inherited head 6404aac03, 0-5,24-29),
warm with the fixed tag (new code, 6-11,30-35). Each warm root primed first by PIPELINE-TEST-TIME-1127, then the five
files in this order. Script `measure.sh` in the story work directory on mei; jobs pwc2-m-cold, pwc2-m-old, pwc2-m-new.

**Verdicts cold against fixed-tag warm: identical on all 5 files** (22 result lines; 20 CAUGHT).
**Left mutant:** gac-neither applied in warm copy-1, products built there with `plants-warm-1`, its test failed there
(rc 101: the cache held the mutant), file backdated to 2000-01-01; the next warm GAI run wrote 1 file, its baseline
passed and its 4 verdicts equal the cold run's; the original text was back after the run.

Wall per job, s; baseline `so` in brackets:

| file (order) | cold | warm, per-run tag | warm, fixed tag |
|---|---|---|---|
| XTASK-PRECHECK-SKIP-ASM-AFFECTED | 978 (269) | 839 (205) | 682 (65) |
| GAI-ADDRCONFIG-LOOPBACK | 1090 (267) | 1005 (196) | 986 (191) |
| PIPELINE-TEST-TIME-1127 | 547 (252) | 480 (194) | 454 (190) |
| SHSTK-OPT-IN-4 | 760 (261) | 992 (324) | 600 (65) |
| HEAP-WALK-ADDRESS-SPACE (no baseline `so`) | 599 | 594 | 522 |
| median | 760 | 839 | 600 |
| sum | 3974 | 3910 | 3244 |

Median paired ratio fixed-tag warm / cold 0.83 (-17%), sum -18%; against the per-run-tag warm: 0.88 (-12%), sum -17%.
Slots shared the host (memory bandwidth, zram), so single rows carry noise (SHSTK's 324 s per-run-tag baseline).

Fixed-tag warm baseline `so` after a run whose last plant was outside the product crates: 53 s of phases (was 194 s):
`early-cargo-ld/ldconfig/localedef` 59+60+58 s -> 0, `early-builds-wait` 83 -> 0, `cargo-libc` 25 -> 0.1,
`libc-archive` 49 -> 24. Left: `libc-archive` 24 s, `layout` 12 s, `early-data` 11 s.

## Why short of ~45%, and the next step

1. The cold baseline is ~260 s of a 550-1090 s job on this host (25-45%), not 45% of every job.
2. A warm baseline is fast only when the previous run's last plant (and the role's tree diff) left the product crates
   alone. After GAI (last plant in `crates/legacy`) or XTASK-PRECHECK (`csys/start/crti.S`) the restore's fresh mtime
   correctly rebuilds everything above that crate (~190 s): the cache holds the mutant's build. Not avoidable by mtimes.
3. Why that costs ~190 s: `crates/ld-so`, `ldconfig-bin` and `localedef-bin` each depend on `niji-boundary` with their
   own feature set (`ld`, `ldconfig`, `localedef`) in `target/product/ld-pie`, so boundary (135k lines) and what it
   is built on compile three more times after any product-crate change, in every mutant build as well as the baseline.
   **next:** a story to build the loader-side tools without three boundary builds (one feature set, or the tools
   depending on less than boundary), measured per mutant build; then `libc-archive` post-processing (24-48 s, not
   cached, D-X.85). Both are product-build changes, not plants changes.

## Notes

- RAM: as PLANTS-WARM-COPY (2.1 GB a warm copy). The measurement roots are under `/mnt/zram/claude/PLANTS-WARM-COPY-2/`.
- The role's real warm copies (`/mnt/zram/claude/plants-warm/s1/copy-1,2`) were filled by this story's first plants run
  (240 s) and reused by the second (60 s).
- Do not redo: the warm/cold verdict proof above, the left-mutant check.
