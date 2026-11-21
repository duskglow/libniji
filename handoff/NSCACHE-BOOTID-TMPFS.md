# NSCACHE-BOOTID-TMPFS (handoff; run 2 = NSCACHE-BOOTID-TMPFS-2)

**State.** Branch `task/nscache-bootid-tmpfs-2` (run 1's `task/nscache-bootid-tmpfs` merged with origin/next
43058b7df). Role mei:~/w/s2. Owner ruling #3233 (skip the boot ID on tmpfs). Decision record **D-16.15** (amends
D-16.10; id reserved for this story). The plan view is next's, unchanged: the train regenerates it from records/.

## The change
- `crates/legacy/src/nscache.rs` `map_region`: `fstatfs(fd)` on every attach; for an existing region on
  `TMPFS_MAGIC` the stamp takes the header's boot words (`Table::boot()`), so `/proc/sys/kernel/random/boot_id` is
  neither read nor compared. If the header is busy it falls back to `/proc`. A region created, or reinitialised
  under the OFD lock, records the boot ID read from `/proc`, so a region on another file system is still compared.
- `crates/core/src/names/cache.rs`: `Table::boot()` (`#[inline]`). `crates/kernel/src/uapi.rs`: `TMPFS_MAGIC`.
- Every other attach check is unchanged (AT_SECURE, real vs effective IDs, overflow UID, initial user namespace,
  O_NOFOLLOW/O_NONBLOCK/O_EXCL, descriptor above the standard ones, fstat regular/owner/0600, size and geometry,
  header magic/layout/size/owner/account stamp, stamp files, seqlock and checksums): the diff touches only the
  source of the boot words.

## Run 2 (what changed and why)
1. Merged origin/next (4914 commits; crate rename liella -> niji; plan/07-decisions is now a generated view). The
   D-16.10 amendment run 1 wrote into the view became the record D-16.15 (`amends`, §10 of the developer-flow
   reference; the original brief's "amend the row" predates records).
2. Plants file: package `niji-boundary` (run 1's name made the first plants baseline fail: SUPERSEDED).
3. **Layout fix found by the counts.** With run 1's code a process's first cached lookup (the attach) took
   +2 faults and ~+12k kernel instr vs next, although a warm re-attach saved 13.4k. A fault map
   (`tools/hottext/faultmap.py`) named the two faults: `niji_sys_fstatfs` and `Table::boot`, both outside the
   hot group, because the build's layout trace always creates its region (ruling #5223) and so never ran the
   attach-to-an-existing-region path. Fix: `fstatfs` also on creation (one syscall once per region) so the trace
   runs and groups it; `Table::boot` `#[inline]`.

## Counts (mei, Skylake-SP; `validation/probes/nscache/measure.sh`, static percall, region on /dev/shm)
Before = origin/next 43058b7df, after = final head; two interleaved rounds, 4 cached-attach samples each per round
(8 per side), fast profile. Logs: `~/logs/nscache-bootid-tmpfs-2-counts2.log` (final), `-counts.log` (run 1 code).

| row | before (next) | run-1 code (superseded) | after (final) |
|---|---|---|---|
| 1st cached lookup of a process (attach), kernel instr, median of 8 | 136.5k (135.0k-146.8k) | ~149k (148.1k-161.8k) | **126.3k** (123.8k-131.5k) |
| same, page faults | 8 | 10 | **8** |
| same, user instr | 18.54k | 17.61k | **17.61k** |
| warm re-attach (setuid, call 3), kernel instr | 117.0k / 123.9k | 103.6k / 105.5k | **105.9k / 111.6k** |
| hit, uncached calls | 4.0k / 19.1k | unchanged | unchanged |

Attach saving: ~10k kernel instr (-7.5%) and ~0.9k user instr per process that attaches, no fault change. strace:
`openat`/`read`/`close` of boot_id replaced by one `fstatfs`. Break-even (run -2 of NSCACHE-SECOND-LOOKUP, by time):
8th lookup (Ivy Bridge) / 10th (N100); the saving is ~7% of the ~136k attach, so the break-even moves down by under
one lookup. Not re-timed (needs a booked timing.sh run; instruction counts only).

## Tests (`crates/boundary/tests/nscache.rs`, clauses s17_3_c_1, s16_1_g_3)
`a_region_on_tmpfs_attaches_without_reading_the_boot_id` (read count from `/proc/thread-self/io`; a stale boot
word is still answered and not rewritten; a reinitialised region records this boot's ID) and
`a_region_on_another_file_system_compares_the_boot_id` (one more read; a stale boot word starts afresh). Each
skips with a printed reason when no suitable directory exists.

## Jobs (mei)
- `nscache-bootid-tmpfs-2-gate2` rc 0: clippy -D warnings, static, then direct tier `affected --run --base
  origin/next`: 17 suites ok, 11585 passed, 0 failed, 1383 ignored; conformance 56/56; peak 14G.
- `nscache-bootid-tmpfs-2-plants2` rc 0: 3 plants, 3 CAUGHT (boot-skip-everywhere, boot-never-skipped,
  reinit-keeps-stale-boot), 0 WARN.
- `nscache-bootid-tmpfs-2-counts2`: the counts above, symbol placement, fault map (text faults 14 = next's), then the
  release profile's `hot_text_products`.
- SUPERSEDED: `-lint` (rc 1 only from a bare `affected` with no `--list`; clippy, records and static passed),
  `-plants` (baseline: old package name), `-gate` (run-1 code; stopped while queued), `-counts` (run-1 code: found
  the +2 faults).
- Run 1's keke jobs (`nscache-bootid-tmpfs-*`) ran on the pre-merge tree: superseded.

## Open / next
- Next: CI. Optionally a booked `validation/probes/nscache/timing.sh` run to restate the break-even by time.
- Not done: making the layout trace attach to an existing region (it would need the tracer to report tmpfs for
  the redirected region and p_nss to re-attach); the code-side fix above makes it unnecessary for this path.
