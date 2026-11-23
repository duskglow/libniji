# LD-START-MAPS: the start's mappings at or below the reference's kernel cost

## State after leg 3 (2026-11-23)

Branch `task/ld-start-maps`, base origin/next e3b0caf60, HEAD 1616cd635 (plus this note). Role mei:~/w/s4
(Skylake-SP). Decision in force: D-27.92 (records/decisions/D-27/D-27.92.toml). Code: `image.rs` `open_start` /
`read_headers` / `image_object` / `image_spans` / `arena_give_back` / `StartMapper::store_file`; `map.rs`
`plan_headers`, `Mapper::store_file`, cold `tail_from_file`; `link.rs` start search via `open_start`.

Leg 3 commits: 5238cc7f2 (each PT_LOAD's image-span kind worked out once per object: the attributed +7.2k
user/object), 09768b1b8 (tests; D-27.92 corrected: a dynamic-step refusal ENDS the search with
`<path>: object file has no dynamic section`, status 127, as the whole-file check's refusal does; it is not
skipped), 1616cd635 (SAFETY comment shortened: the unsafe audit reads a fixed distance up).

### Jobs started on HEAD 1616cd635 (mei), not yet collected

- `ld-start-maps-gate` (direct tier, 14 targets, `cargo xtask affected --run --base origin/next`; not FULL)
- `ld-start-maps-plants` (9 plants, `~/work/ld-start-maps/plants.txt`, dir `/mnt/zram/claude/ld-start-maps/plants-1`;
  copy in the scratch dir `plants.txt`). Commit `<dir>/results.txt` as `handoff/LD-START-MAPS-plants.txt`.
- `ld-start-maps-startup` (waits for the gate, then `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so`,
  `hot_text_products`, copies `target/product/fast/layout-bins.tsv` to `~/work/ld-start-maps/layout-bins-head.tsv`)

Clippy (`cargo xtask clippy`) and `cargo xtask static` clean on the role at 1616cd635. New tests passed in
job `ld-start-maps-newtests` (on 09768b1b8's tree before the comment-only commit): map 7/7, ldso_search 4/4, no skips.

### Measured (kit, release, Skylake-SP, median of 101; `ld-start-maps-meas-base` vs `ld-start-maps-meas-new4`)

| program | user | kernel | faults | syscalls | maps |
|---|---|---|---|---|---|
| n0 | 159,047 -> 161,463 | 730,939 -> 628,434 | 58 -> 57 | 29 -> 28 | 14 -> 11 |
| n3 | 250,528 -> 262,973 | 1,122,981 -> 920,475 | 70 -> 70 | 59 -> 61 | 35 -> 29 |
| t | 216,900 -> 219,260 | 730,456 -> 632,543 | 58 -> 57 | 29 -> 28 | 14 -> 11 |
| glibc 2.39 g0 / g3 / true (this run) | 101,855 / 138,675 / 128,175 | 592,425 / 940,487 / 587,565 | 51 / 67 / 51 | 30 / 64 / 30 | 12 / 27 / 12 |

Per start: kernel -102k (n0) and -203k (n3); n3's kernel count is now below glibc's (920k vs 940k), n0 is +36k
over glibc's. User cost +2.4k (n0) and +12.4k (n3), about +3.3k per extra library (was +7.2k before 5238cc7f2):
image-mode reading (`image_spans` 1.4k, `image_tables` 0.75k, `open_start` 0.6k per object, PT tables in
`~/work/ld-start-maps/pt-base`, `pt-new3`). This goes in the report as a `cost` row. Faults: -1 (n0), level (n3);
the +6 fault gap to glibc is the stack and anonymous areas (leg 1 attribution), not this story's mappings.
Target "kernel and faults <= the reference" is NOT reached for n0 (kernel +36k, faults +6): COMPLETE with
"target not reached", next: the stack inline tables (`link`'s `Table<_, INLINE>`) and the anonymous areas.

Kit note: `meas.sh` needs an ABSOLUTE output dir (it `cd /`s before writing); a relative one fails at the first row.

### Not planted, with reasons

`arena_give_back`'s zeroing: every consumer of a scratch range writes it before reading (span_slots, read_image,
pread), so a delete-zeroing plant would be equivalent; the zeroing keeps `scratch`'s documented invariant.
`kind_of`'s past-32 fallback: only objects with more than 32 PT_LOADs reach it (map.rs's 300-load program), whose
extra pages hold nothing read by address, so a plant there would survive for that reason.

## Exact next step

1. `~/bin/jobwait ld-start-maps-gate ld-start-maps-plants ld-start-maps-startup` (mei). Red gate: fix, rerun
   only the failing suite. A SURVIVED plant: strengthen the test, not the plant.
2. Layout-bins: compare `~/work/ld-start-maps/layout-bins-head.tsv` with the merge base's (e3b0caf60; build it
   only if no copy exists) and say what moved; tighten DYN_TRUE_BOUND if hot_text_products shows room (never loosen).
3. `cargo xtask precheck --jobs ld-start-maps-gate,ld-start-maps-plants,ld-start-maps-startup,ld-start-maps-meas-new4 --head <sha>`.
4. `handoff/reports/LD-START-MAPS.json` (impact: perf kernel n0 730,939 -> 628,434 and n3 1,122,981 -> 920,475;
   faults n0 58 -> 57; cost user n0 +2.4k / n3 +12.4k; coverage 2 tests, 9 plants; correctness: a dependency
   whose PT_DYNAMIC is outside every PT_LOAD is refused, its `_DYNAMIC` would not be mapped), push,
   `ops handback-check`, hand back with the per-start numbers in the first line (owner 11-21).
   INFORMATIONAL: `cargo clippy -p niji-boundary --all-targets` alone fails in ffi.rs/selftest.rs (feature set;
   `cargo xtask clippy` is clean); `jobrun ... measure` kind failed in 1 s for the kit (used `bench`).

## Before counts and attribution (leg 1, kept for reference)

Base (release, kit form, median of 101; job `ld-start-maps-meas-base`): n0 159,047 user / 730,939 kernel / 58
faults / 29 syscalls / 14 maps; glibc g0 101,855 / 588,762 / 51 / 30 / 12. Leg 1 attribution (job
`ld-start-maps-attr-base4`, logs `~/work/ld-start-maps/attr-base/`): the whole-file view was about 100k of the
142k kernel gap (arena munmap 37.9k, view mmap 22.2k, arena mmap 8.7k, view faults ~30k); the rest is `link`'s
stack frame (inline per-object tables, +7 faults), libc RELRO/data faults, the path store (16 KiB), the thread
block and TLS (70,736 B) and the main thread's descriptor area (69,632 B).

## Do not redo

- Leg 1's attribution; leg 3's PT attribution of the user cost (above).
- `MADV_RANDOM` on the view (more faults, rejected by reasoning); mapping the view without the arena (brings
  back DYN-START-SPEED-2's alignment dependence).
