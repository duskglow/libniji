# HEAP-ADDRESS-SPACE (GitHub issue #21): handoff

State: branch `task/heap-address-space-3` (continues -2 and -1), head = this handoff's commit. Build role mei
`~/w/s1`; scripts in mei `~/work/HEAP-ADDRESS-SPACE-3/` (`base.sh`: next's products in a RAM copy + the issue
oracle and the probe's relro map against them; `relmap.sh`/`relmap.py`: link the start-up probe with a map and sum
`.data.rel.ro` by input section). Spec: `~/state/briefs/HEAP-ADDRESS-SPACE.md` "## The change".
Decisions in force: D-3.19, D-3.14, ruling #6325; pending records `records/pending/HEAP-ADDRESS-SPACE-3-1.toml`
(D-3 series: growth with use, amends D-3.14/D-3.19) and `-2.toml` (D-X series: the GOT alignment).

## Done

- Runs -1/-2: segments mapped as far as used (grow in place, doubling, at most 8 slabs per step), large table
  generation 0 one page, nearby-class spill before mapping, `locate_in` sends a window's unmapped part to the large
  table; rt tests updated by cause; regression test `the_heap_keeps_address_space_near_its_use`.
- Run -3: the STARTUP-DATA-WINDOWS blocker fixed forward (ruling #6325): `entry.S` carries an empty `.got` input
  section aligned to 256, so the output GOT (200 bytes, start-up slots +0x30..+0xc0) cannot straddle a page whatever
  relro precedes it; `cargo xtask so` checks it (`check_group_end`). Products build: all 12 contracts hold,
  7 written start-up pages (was 8). Job `heap-address-space-3-so1`.
- Issue test runs on mei (job `heap-address-space-3-hmp1`, 5 passed): +836 / +28996 / +32836 / +34888 KiB, same
  as keke. rt heap tests on the synced head: `heap-address-space-3-rt1`, 27 passed. Lint (clippy, static, records
  check, the so.rs unit test): `heap-address-space-3-lint1` rc 0.

## Running at hand-back (mei, detached)

- `heap-address-space-3-gate` (direct tier, `cargo xtask affected --run`), `heap-address-space-3-plants2`
  (handoff/HEAP-ADDRESS-SPACE-3-plants.txt, 6 plants, dir `/mnt/zram/claude/HEAP-ADDRESS-SPACE-3/plants-2`; run -3 first try `-plants` refused: .S plant without a label anchor),
  `heap-address-space-3-base1` (evidence: next's products; its log prints the issue steps on next, the RLIMIT_AS
  run, and the probe's relro).

## Not done yet (exact next steps, in order)

1. Read the three jobs. Plants: an unexpected SURVIVED on `no-spill` or `window-tail-invalid` means the products
   test does not reach that path: add a targeted rt test, never loosen. Gate red: fix by cause.
2. Ruling #6325 step (1), not finished: compare `.data.rel.ro` by input section, branch
   (`/mnt/zram/claude/HEAP-ADDRESS-SPACE-3/brmap/pf.map`, 44904 B, 462 sections) vs next (`base1`'s
   `base-1/map/pf.map`): `python3 ~/work/HEAP-ADDRESS-SPACE-3/relmap.py <map> .data.rel.ro`, then diff. Most relro is
   anonymous (`anon.<hash>`), so compare section counts and size histograms; the start-up cluster is unchanged.
3. "Fails on next": `base1`'s log must show the steps above the test bounds (expected +16448 / +32836 / +49220 /
   +67652) and the RLIMIT_AS run failing; quote it in the report.
4. Start-up checks on the final head (brief, 2026-11-14): `cargo xtask test`, and
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`, each its
   own jobrun label, after the gate (one product build per role).
5. Counters: `cargo xtask bench --counters cbench_malloc` and `cbench_startup` on the branch and in the base copy;
   the limits in `bench/instr_limits.tsv` are for the Zen 1 CPU (keke), so run them on a Zen 1 role or let CI check.
6. Then the remaining gap: returning empty segments' address space (lock-free `find_seg` readers need a design and a
   decision record); findutils `find` over 200000 entries under the issue's `ulimit -v` formula (not reproduced).

## Process notes

Run -3: one wasted call (a stray `cat` waiting on stdin, with a heredoc; killed by PID). The keke base-map job
waited in keke's full queue and was cancelled (`kill-label`) in favour of the same build on mei.
