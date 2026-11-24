# LD-START-MAPS: the start's mappings at or below the reference's kernel cost

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
head; `handoff/LD-START-MAPS-2-plants.txt`), `-xtask-test`. Results: see the report json and the hand-back.
Earlier: `-lint0` (inherited head, clean), `-startup` / `-krp2` (merged head before change 2), `-nextprod` (next's
products for the comparison), `-krp` (failed: wrong product layout, no measurement).

Not planted, with reasons: `arena_give_back(head)` in `program_from_headers` (returns ~0.8 KiB of the 16 KiB scratch
arena; observable only when an explicitly run program's dependencies would fill the arena within those bytes).

## Exact next step

Coordinator: collect the jobs; land if green. Next stories for the remaining gap (kernel, exec'd programs): the
written pages (faults +9.5 on the empty main: link's inline stack tables, libc's `.data`/`.bss` writes), the heap's
first area by reservation and trims, the sub-page anonymous mappings at start.

## Do not redo

Leg 1's attribution; leg 3's PT attribution; `MADV_RANDOM` on the view (rejected); mapping the view without the
arena (brings back DYN-START-SPEED-2's alignment dependence). Kit notes: krp's `--products` needs the dynamic product
layout (`wl_dyn_layout` in validation/wl_lock.sh), not `target/niji`; `--runs 0` with an empty `--kallsyms` file gives
the counts without kernel samples.
