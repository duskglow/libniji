# LD-START-MAPS: the start's mappings at or below the reference's kernel cost

## State after LD-START-MAPS-3 (2026-11-24)

Branch `task/ld-start-maps-3` (LD-START-MAPS-2's head 9e1bfb2e5, origin/next 237dbda92 merged in at f432fe7aa;
code head 5cb29d2c2, later commits handoff only).
Role keke:~/w/n1 (Zen 1 host, 60 vCPU). The train could not merge LD-START-MAPS-2 (conflict in
`crates/boundary/src/ld/image.rs`): next made `image_view` return the image reader built once (`Elf::with_spans`,
D-27.97), while this branch had moved `image_view`'s span reading into `image_spans` (shared with the start mapper's
`image_object`). Resolution: `image_spans` returns the reader (`probe.with_spans(..)`, `None` -> "malformed
program headers") instead of the bare spans; `image_view` and `image_object` both take it, so `image_object` no
longer builds a second reader with `Elf::from_image` (the table check is the probe's, already mapped to
`Refusal::Malformed`). No other file conflicted; no behaviour change intended beyond next's.
Plants: `handoff/LD-START-MAPS-3-plants.txt` (run 1: LD-START-MAPS-2's 16 on the merged head + `merge-with-spans`;
15 CAUGHT, 2 SURVIVED). `give-back-overlap` had been caught on shiki only by a system-level search test and survived
here: new unit test `ld::image::tests::arena_give_back_keeps_the_bytes_still_in_use` (no range in use handed out
twice; given-back bytes zeroed). `merge-with-spans` dropped only the last read-only span (not read by the reader: too
weak), replaced by `merge-with-spans-2` (no spans). Run 2: `handoff/LD-START-MAPS-3-plants-2.txt`.
Jobs on keke (labels `ld-start-maps-3-*`): `lint0` (inherited head clean), `lint1`/`lint2` (clippy + static clean),
`gate` (merged head) and `gate2` (final code head) direct tier, `plants`, `plants2`, `startup` (full-profile
products + `hot_text_products` 37/37; `cargo xtask test` 7311 passed, 1 failed: the #21 test below), probes
`findneed`, `findexec2`, `findglibc`, `walkmaps`.
The measurements below are LD-START-MAPS-2's (shiki, before this merge); this run did not re-measure them.

**Blocker found by this run's gate (keke, Zen 1): GitHub issue #21's formula.** Direct gate `ld-start-maps-3-gate`:
1064 passed, 2 failed, both `niji::heap_meta_products find_over_200000_entries_bounds_its_address_space` (plain and
native pass): "needs 37036 KiB over 3 entries at layout phase 0 > issue #21's 35000" (37292 at phase 4096; the
placement rule beside it, need vs mapped, passes: 256/512 <= 548). It is findutils' `many-dir-entries-vs-OOM`
(Yohane's case): the 200000-entry walk must succeed under the 3-entry run's `ulimit -v` need + 35000 KiB.
Measured on keke with the test's method (fixed layout, node 0, en_US.UTF-8, 4 KiB bisection; probe scripts
`findneed.sh` / `findneed_exec.sh` in the run's scratch, `find` explicit (`ld.so --library-path`) and exec'd (a copy
whose PT_INTERP names the products' loader): identical numbers both ways), KiB:

| products | need, 3 entries | need, 200000 | difference |
|---|---|---|---|
| next de86ab08b (primed release-dyn) | 27512 | 48820 | 21308 |
| this head (merged) | 12392 | 49428 (phase 4096: 49684) | 37036 (37292) |
| glibc 2.39 (system `find`) | 3680 | 33200 | 29520 |

Cause: this branch maps start dependencies (and `ld.so PROGRAM`'s program) from their headers instead of whole-file
views, so the start's address-space peak under a limit fell 15120 KiB. The walk's own need barely moved (+608/+864,
inside the 256 KiB layout bands), and the start cancels out of the difference, so the formula now sees the heap's walk
overhead: the walk maps ~37 MiB (strace, no limit: the first segment's over-mapping kept 4420 KiB, two later segments
of 2304 KiB after trimming their 32 MiB over-mappings, 13 slab chunks of 1792-2048 KiB) where glibc's walk needs
29.5 MiB. On next the 15 MiB start peak was the slack that hid it. Not a loader defect; the loader cannot fix it
without inflating its start again. Every later start-up address-space gain widens the same gap.
Flagged as challenge #8348; coordinator's answer (06:53Z): agreed; finish the loader work and hand back ready; a heap
story HEAP-WALK-ADDRESS-SPACE takes the walk overhead; LD-START-MAPS is held until it lands, then both land. No heap
change and no change to the formula in this branch.

## State after LD-START-MAPS-2 (2026-11-24)

Branch `task/ld-start-maps-2` (LD-START-MAPS's branch, origin/next de86ab08b merged in at 28e2001bf, clean).
Code head 9e0b3ed92; later commits: plants file, this note, the report. Role shiki:~/w/sh2 (Haswell-EP, 26 threads).
Decision in force: D-27.92 (records/decisions/D-27/D-27.92.toml), amended by this run for `ld.so PROGRAM`.

Done this run:
1. Merged origin/next (no conflict; next's loader changes are TLS descriptors and `surplus_bytes`, not the mapper).
   The CI red on train 953 (`init_dynamic_empty_main_stays_within_its_instruction_bound`, batch overshoot) does not
   reproduce on the merged head: `hot_text_products` 36/36, then 37/37 with the new test (empty main 162,618 user
   instructions on this host, inside the bound).
2. `ld.so PROGRAM` (the loader run as a command) was the last start path reading a file through a whole-file view
   (its own reservation, two trims, the view and its unmap). `image.rs::load` now maps the program as a start
   dependency is mapped: `program_from_headers` (one `pread` of the headers into the scratch arena, given back after,
   `map::plan_headers`, `map::map_planned` with `StartMapper`); anything those headers do not settle falls back to the
   old code, moved whole into the cold `program_by_view` (all refusal messages, COMPAT-19). `read_headers` returns the
   arena range as `&'static mut` (open_start binds it as a shared slice; no change there).
3. Test `hot_text_products::init_explicit_run_maps_the_program_without_a_whole_file_view` (traces `ld.so PROGRAM`, no
   `munmap` of the program file's length). The tracer `oracle/start_syscalls.c` takes program arguments (the words
   before the first `NAME=VALUE`).

### Measured (owner rule #8178: `results/kernel-residual/probes/krp.py --only shared,strace`, pages held by a holder)

Shiki (Haswell-EP, kernel of that host), release products (`NIJI_FULL_PROFILE=1 cargo xtask so`), medians of 21, two
runs each, alternating head and next; every program started through an explicitly invoked loader, as krp does.
"next" = origin/next de86ab08b without this branch; "leg 4" = the merged head before change 2 (28e2001bf).
Per start: user / kernel instructions, total, page faults, mmap/munmap (strace).

| program | glibc 2.39 | next | leg 4 (merged) | this head | head - next | head - glibc |
|---|---|---|---|---|---|---|
| empty `main` | 104.9k / 590.6k = 695.6k, 51, 8/1 | 166.9k / 726.0k = 893.0k, 62, 11/1 | 168.6k / 652.0k = 820.6k, 61, 9/0 | 169.0k / 613.2k = 782.2k, 60.5, 9/0 | user +2.1k, kernel -112.8k, total -110.7k | +86.6k (user +64.1k, kernel +22.6k), faults +9.5 |
| `true` | 131.3k / 590.2k = 721.5k, 51, 11/0 | 207.1k / 750.6k = 957.7k, 63, 18/4 | 210.3k / 680.8k = 891.1k, 64, 15/3 | 210.8k / 637.8k = 848.6k, 62, 13/0 | +3.6k, -112.8k, -109.2k | +127.1k (user +79.5k, kernel +47.6k), faults +11 |
| `ls /` | 564.5k / 1,535.8k = 2,100.3k, 102, 35/0 | 780.3k / 1,979.2k = 2,759.4k, 172.5, 47/8 | 782.4k / 1,802.0k = 2,584.4k, 167, 40/7 | 783.0k / 1,739.0k = 2,521.9k, 163, 38/4 | +2.7k, -240.2k, -237.5k | +421.6k (user +218.5k, kernel +203.2k), faults +61 |
| `sed` | 910.0k / 1,598.8k = 2,508.8k, 110, 38/0 | 1,140.8k / 2,174.7k = 3,315.5k, 190, 55/8 | 1,146.8k / 1,963.8k = 3,110.6k, 181.5, 46/7 | 1,147.5k / 1,905.7k = 3,053.3k, 180.5, 44/4 | +6.7k, -268.9k, -262.2k | +544.5k (user +237.5k, kernel +306.9k), faults +70.5 |

Unheld libniji pages (no holder) cost a further 55k to 167k kernel instructions per start (krp `niji` column, leg 4);
glibc's are held by every process on the host.

What each part removes: the dependencies read from their headers and mapped by span (LD-START-MAPS, legs 1-4)
are 68k to 224k of the kernel column and apply to every exec'd dynamic program (Yohane's rows); the program read from
its headers (change 2) is a further 38k to 63k, and applies only where the loader is run as a command (`ld.so
PROGRAM`, krp's own setup, `ldd`-style use), not to an exec'd program. Against Yohane's table (+118k / +620k / +709k
kernel over glibc 2.41 for `true` / `ls` / `sed`), the exec'd share removed here is 68k / 191k / 224k (this host, glibc 2.39).

What is left on the mapping side (strace, `ls /`): the heap's first area (a 32 MiB aligned reservation and two trims:
3 calls where the reference uses `brk`), two sub-page anonymous mappings made and unmapped during the start (720 and
384 bytes), libc's `.bss` mapping of 770 KiB (the reference's is 52 KiB; untouched pages cost nothing, the touched
ones are faults), and the 16 KiB / 6.9 KiB / 68 KiB anonymous areas (thread block, TLS, signal stack). The empty
`main` makes 9 mmap + 2 mprotect + 0 munmap against the reference's 8 + 3 + 1: its mapping calls are at parity, so
its remaining kernel gap (+23k held) is page faults (+9.5: written pages), not mappings.

### Jobs on the code head (shiki)

`ld-start-maps-2-lint1` (clippy + static), `-startup2` (full products, `hot_text_products` 37/37), `-krp3`
(measurement above), `-gate` (direct tier), `-plants` (16 plants: 3 new, 13 of LD-START-MAPS again on the merged
head; `handoff/LD-START-MAPS-2-plants.txt`), `-xtask-test`. Results: gate all steps ok (peak 3.8G); plants
16/16 CAUGHT, no WARN; `cargo xtask test` ok (7265, 4850, 129 and 46 tests passed).
Earlier: `-lint0` (inherited head, clean), `-startup` / `-krp2` (merged head before change 2), `-nextprod` (next's
products for the comparison), `-krp` (failed: wrong product layout, no measurement).

Not planted, with reasons: `arena_give_back(head)` in `program_from_headers` (returns ~0.8 KiB of the 16 KiB scratch
arena; observable only when an explicitly run program's dependencies would fill the arena within those bytes).

## Exact next step

Held (coordinator #8348) until HEAP-WALK-ADDRESS-SPACE lands: it must make find's 200000-entry walk need at least
2.3 MiB less address space with this branch's start (37292 -> under 35000 at every phase, with margin; the walk's slab
chunks and per-segment over-mapping kept, D-3.22/D-3.25). Then: merge next here, re-run the direct gate
(`heap_meta_products` must be green; everything else was green on this head, see the hand-back), land both.
Do not loosen ISSUE_21_ALLOWANCE (ALWAYS FORWARD). Next stories for the remaining gap (kernel, exec'd programs): the
written pages (faults +9.5 on the empty main: link's inline stack tables, libc's `.data`/`.bss` writes), the heap's
first area by reservation and trims, the sub-page anonymous mappings at start.

## Do not redo

Leg 1's attribution; leg 3's PT attribution; `MADV_RANDOM` on the view (rejected); mapping the view without the
arena (brings back DYN-START-SPEED-2's alignment dependence). Kit notes: krp's `--products` needs the dynamic product
layout (`wl_dyn_layout` in validation/wl_lock.sh), not `target/niji`; `--runs 0` with an empty `--kallsyms` file gives
the counts without kernel samples.
