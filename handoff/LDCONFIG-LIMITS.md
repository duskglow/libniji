# LDCONFIG-LIMITS: ldconfig reads all of its configuration and has no input caps (audit HL-15)

**State.** Branch `task/ldconfig-limits`, cut from origin/next 59c31ec19. Code head: the commit "List only a directory's
own keys in the ldconfig test file system" (two code commits; later commits add only this note and the report json).
Role keke `~/w/r2`. Run 1 stopped at the context checkpoint (~150k) with the final jobs queued, NOT YET VERIFIED.

## What changed (pending record `records/pending/LDCONFIG-LIMITS-1.toml`, D-NEW in series D-27, amends D-27.43)

| cap (old) | now |
|---|---|
| `conf.rs` `FILE_MAX` 1 MiB (rest ignored silently) | file read whole (`read_file(.., usize::MAX, ..)`); a file-system `ENOMEM` is `Fatal::Oom` (`out of memory`, exit 1), no longer skipped like a missing file |
| `conf.rs` `DIRS_MAX` 65536 | removed; duplicates found through an open-addressing hash index of (dev, ino) (linear, was a quadratic scan) |
| `writer.rs` `HWCAPS_MAX` 64 | inline list of `NAMES_INLINE` = 64, spill to a fallible `Vec` (sorted + deduplicated once); `WriteError::Oom`; `TooManyHwcaps` gone |
| `writer.rs` `GENERATOR_MAX` 4096 | removed (format's 32-bit offsets bound it: `TooLarge`) |
| `writer.rs` `ENTRIES_MAX` 2^20 | removed (`TooManyEntries` gone; `TooLarge` bounds it) |
| `boundary/src/ldconfig.rs` `ARGS_MAX` 4096 | removed; `try_reserve_exact(argc)` failure = `out of memory` |

Also: scan.rs maps the cache-buffer reservation failure and `WriteError::Oom` to the run's `out of memory` (was
"too large"). Kept: `DEPTH_MAX` (HL-28), scan.rs hostile-input bounds (HL-24).

## Probe (black box, `~/work/ldconfig-limits/probe.sh`, private roots on /dev/shm, keke)

| case | reference 2.39 | next (before) | head |
|---|---|---|---|
| A 3 MiB conf, last line `/opt/last` | dir listed, link made | not read, no link | job `ldconfig-limits-gate` log, lines `head A..D` |
| B 70000 dirs + `/d/0` again | 70000 listed, 1 duplicate msg | 65536 listed, 4465 "too many" | (same) |
| C 100 hwcaps subdirs | exit 0, 101 libs, 100 hwcaps entries | exit 1, cache not written | (same) |
| D 5002 args (`-n -X`) | exit 0 | exit 1, `Argument list too long` | (same) |

## Tests (each fails on next)

`cache_write.rs::write_has_no_count_caps` (64, 65, 100 names, 64 KiB generator, 2^20+1 entries written and read
back: practical in memory, so no lowered threshold was needed); `ldconfig.rs::a_large_configuration_is_read_to_its_last_line`,
`many_configured_directories_are_all_kept`, `many_hwcaps_subdirectories_reach_the_cache`,
`a_configuration_read_out_of_memory_ends_the_run` (mock ENOMEM); `xtask/tests/transcripts.rs::the_shipped_ldconfig_reads_every_argument_and_reports_exhaustion`
(product: 5002 args; a sparse 256 MiB conf under `ulimit -v 65536` must end `out of memory`, exit 1). The mock file
system's `list` now ranges over the directory's keys (it scanned the whole map: quadratic in the 70000-dir test).

## Jobs queued at hand-back (keke)

* `ldconfig-limits-gate`: clippy, `cargo xtask so`, the two direct test files, the head probe, `cargo xtask affected --run`
  (direct tier, under `~/bin/gate`), `cargo xtask static`. Superseded: `ldconfig-limits-check1` (xtask test used
  `Vec::push`: disallowed method), `ldconfig-limits-check2` (stopped: the mock's quadratic listing).
* `ldconfig-limits-plants`: `handoff/LDCONFIG-LIMITS-plants.txt`, 12 plants (each cap restored alone, plus the hash
  index, rebuild, spill sort/dedup, name offsets, ENOMEM skip). Not planted: the scan.rs `WriteError::Oom` arm (an
  allocation failure inside the writer is not reachable deterministically without a fault-injecting allocator).

## Not done / next step

1. Read `~/bin/jobwait ldconfig-limits-gate ldconfig-limits-plants` on keke. If the xtask OOM sub-test fails, check
   the `ulimit -v 65536` budget against the product's start-up mappings first (not measured yet).
2. Fill the head column above from the gate log (`grep '^head ' ~/logs/ldconfig-limits-gate.log`); the pending record
   claims "libniji now does too": verify it there.
3. Counters: ldconfig is not on a start-up row or bench row (it is its own static program); the writer's common case
   (<= 64 names) allocates nothing new. No counts run.

## Process notes

Rule slips this run (not refused, reported honestly): three `sed -i` edits (cache.rs re-export, ldconfig.rs lib
count, transcripts.rs doc line) and three heredocs (python/cat) where the Edit/Write tools were required.
INFORMATIONAL: with `-n`, the reference prints nothing for a repeated operand; libniji prints "given more than once"
(pre-existing, not changed here).
