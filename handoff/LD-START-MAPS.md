# LD-START-MAPS: the start's mappings at or below the reference's kernel cost

## State after leg 4 (2026-11-23, final)

Branch `task/ld-start-maps`, base origin/next e3b0caf60. Code head 961e52335 (later commits: D-27.92's text,
this note, the report). Role mei:~/w/s4 (Skylake-SP). Decision in force: D-27.92
(records/decisions/D-27/D-27.92.toml). Code: `image.rs` `open_start` / `read_headers` / `image_object` /
`image_spans` / `arena_give_back` / `StartMapper::{store_file, map_span}`; `map.rs` `plan_headers`, `span_first`,
`Mapper::{maps_span, map_span, store_file}`, cold `tail_from_file`; `link.rs` start search via `open_start`.

Leg 4: gate on 1616cd635 RED, `hot_text_products::init_dynamic_start_system_calls_stay_within_their_bounds`:
"three: 52 calls besides 8 openat, over the bounds 50" (DYN-START-RING). Cause: the per-object `pread` of the
dynamic array (one call per dependency). Fix 961e52335: the start's mapper makes the reservation of a movable,
page-aligned dependency whose PT_LOADs leave no gap by mapping its file over the span with the first segment's
protection (`Mapper::map_span`, one mmap fewer per such object); the first segment keeps it, each later one maps
over its part. Gapped, over-aligned (libc: 2 MiB), fixed, or read-only-first-with-zero-fill objects reserve apart;
dlopen's mappers unchanged. Model test extended (spanned space equals the whole-file space, one call fewer where
gapless; a gapped shared object reserved apart).

### Jobs on the code head 961e52335 (mei)

- `ld-start-maps-gate2`: direct tier, all green (13 steps, `test -p niji` included).
- `ld-start-maps-plants3`: 13 plants, 13 CAUGHT, no WARN (`/mnt/zram/claude/ld-start-maps/plants-3/results.txt`;
  plants file `~/work/ld-start-maps/plants.txt`, copy in the scratch dir). 4 new: span-gap, span-in-place,
  span-prot, start-span-offset.
- `ld-start-maps-startup3`: `NIJI_FULL_PROFILE=1 cargo xtask so`, `hot_text_products` 33/33, kit counts.
- `ld-start-maps-xtask-test`: `cargo xtask test` (workspace), started last; see the hand-back.
- Superseded: `ld-start-maps-gate` (1616cd635, red as above), `-plants` / `-startup` (old head, stopped),
  `-plants2` (a `|` in a plant's replacement split the line: baseline failed, no plant applied), `-startup2`
  (stopped: `cargo xtask test` first would have held the counts past the deadline; its one failure is below).
- `ld-start-maps-basebins`, `-basebins-fast`: the merge base's products in a RAM copy, for layout-bins.

Layout-bins (`target/product/{fast,release}/layout-bins.tsv`): identical to the merge base's (0 lines differ).
DYN_TRUE_BOUND not tightened: the passing test prints no count, none measured.

### Measured (kit, release, Skylake-SP, median of 101; `ld-start-maps-meas-base` vs `ld-start-maps-startup3`)

| program | user | kernel | faults | syscalls | maps |
|---|---|---|---|---|---|
| n0 | 159,047 -> 161,576 | 730,939 -> 626,081 | 58 -> 57 | 29 -> 28 | 14 -> 11 |
| n3 | 250,528 -> 265,243 | 1,122,981 -> 903,807 | 70 -> 70 | 59 -> 58 | 35 -> 26 |
| t | 216,900 -> 219,364 | 730,456 -> 627,444 | 58 -> 57 | 29 -> 28 | 14 -> 11 |
| glibc 2.39 g0 / g3 / true (same run) | 101,855 / 138,761 / 128,175 | 586,781 / 920,185 / 588,150 | 51 / 67 / 51 | 30 / 64 / 30 | 12 / 27 / 12 |

Per start: kernel -105k (n0), -219k (n3); n3 is now 16k below glibc's kernel count, n0 +39k over. Faults -1 (n0).
User +2.5k (n0), +14.7k (n3): image-mode reading (leg 3 PT attribution) and `span_first`'s pass over the headers
(about +0.75k per small library over leg 3). Target "kernel and faults <= the reference" NOT reached for n0
(kernel +39k, faults +6, the stack inline tables and anonymous areas per leg 1's attribution).

### Not planted, with reasons

`arena_give_back`'s zeroing (every consumer writes a scratch range before reading it: equivalent).
`kind_of`'s past-32 fallback (only the 300-PT_LOAD program reaches it; its extra pages hold nothing read).

### Open / INFORMATIONAL

- `catalogs_products::catalogs_equal_the_reference` failed in `cargo xtask test` (startup2, stopped): "catalog
  file replaced on disk after it was loaded": libniji `[new]`, reference `[old]`, name-service cache on. No message
  catalog code on this branch; run on the merge base's products in job `ld-start-maps-basecat` (see hand-back).
- `cargo clippy -p niji-boundary --all-targets` alone fails in ffi.rs/selftest.rs (feature set); `cargo xtask
  clippy` is clean. `jobrun ... measure` kind failed in 1 s for the kit (used `bench`).
- Kit: `meas.sh` needs an ABSOLUTE output dir.

## Exact next step

Coordinator: collect `ld-start-maps-xtask-test` (and `-basecat`); land if green. Next story for the n0 gap: the
stack inline tables (`link`'s `Table<_, INLINE>`) and the anonymous areas (thread block, TLS, descriptor area).

## Do not redo

Leg 1's attribution; leg 3's PT attribution; `MADV_RANDOM` on the view (rejected); mapping the view without the
arena (brings back DYN-START-SPEED-2's alignment dependence).
