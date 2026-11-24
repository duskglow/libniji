# LOCALE-ADDRESS-SPACE: handoff (leg 1 of run 1: investigation done, no code yet)

State: branch `task/locale-address-space`, base origin/next `fb12f4d3e`; build role keke `~/w/r1` (Zen 1, 60 vCPU),
products built there at the base (job `locale-address-space-so0`, rc 0). Brief: `~/state/briefs/LOCALE-ADDRESS-SPACE.md`.
Scripts (keke `~/work/locale-address-space/`, copies in the run's scratch `work/`): `measure.sh <role> <out> <runs>`
(least `ulimit -v` of `find <3 files>` per library and LANG, fixed layout and ASLR; `smaps_at_exit.c` preload dumps
status + smaps to fd 3), `smaps_sum.py A B [-m]` (per-mapping diff), `trace.sh <role> <out> [k]` (strace mmaps,
`k` = with stacks), `refill.sh <role> <out>` + `sizes.py` (gdb on libniji: size of each heap slow-path allocation
and each `Heap::refill`, i.e. each size class's first slab).

## Measured at the base (keke, glibc 2.39 host, `/usr/bin/find`, 3 files on tmpfs; job `locale-address-space-m0`)

| | C | en_US.UTF-8 | C.UTF-8 |
|---|---|---|---|
| libniji need (KiB; fixed and 3 ASLR runs, spread 0) | 8716 | 12416 (+3700) | 9304 (+588) |
| reference need | 3736 | 3736 | 3736 |
| libniji anon at exit (KiB) | 5900 | 8528 (+2628) | 6476 (+576) |
| reference anon at exit | same | +0 | +0 |

The reference's need does not move with LANG: under the limit its setlocale fails and `find` runs in "C" (the
archive, 2988 KiB, is mapped in both libraries without a limit). Compare at-exit address space (VmSize, smaps), not
the 3-file need, for the reference.

## Where libniji's extra anonymous memory goes (smaps + stacks)

1. **Heap size classes (+2048 KiB at exit: one segment growth step).** Every size class's first block takes a 256 KiB
   slab. The en_US setlocale leaks, through the heap, four objects in classes the C run never uses:
   `intern_name_locked` name 31 B (class 32), `LocaleView::new` items 5920 B (6144) and `Tables` 288 B (288),
   `intern_locked`'s boxed `LocaleView` 2944 B (3072); plus gettext's `intl::record_for` 240 B and `intl::intern` 46 B
   (the missing findutils catalog under a non-C LC_MESSAGES). C uses 16 slabs' room; en_US needs 17, so `find`'s
   `fdopendir` (`DirState`, 32776 B) grows the segment by 2 MiB (rss +20 KiB only). ~9.2 KiB of data costs
   4 x 256 KiB of slabs.
2. **Loader state (+576 KiB, any non-C LC_CTYPE incl. C.UTF-8).** `locale::publish_global` ->
   `compat_ctype_objects` -> `niji_dl_sym` (6 names @GLIBC_2.2.5) creates the `dlopen` `Loader` (one 540224 B
   NORESERVE map: in-line tables for `CAP` = 512 objects) and `image_view` maps one page per start-up object (10).

## Design decided (implement next)

* `crates/core/src/mem.rs`: trait `Permanent` (data kept for the process: `slice<T: Copy>(n, fill)`, `value<T>(v)`),
  default methods = today's TryVec/TryBox leak; `pub struct OnHeap; impl Permanent for OnHeap {}`.
* `crates/core/src/locale/view.rs`: `LocaleView::new_in(locale, handle, reuse, perm: &impl Permanent)` placing the
  items (`perm.slice(total, Item::Word(0))`) and `Tables` (`perm.value`); `new` = `new_in(.., &OnHeap)` (all test
  callers unchanged).
* `crates/legacy/src/global_locale.rs`: `pub trait Objects: Permanent`; `intern_locked` uses `new_in(.., objs)` and
  `objs.value(view)`; `intern_name_locked` uses `objs.slice(len + 1, 0u8)` and copies the name in.
* `crates/boundary/src/locale.rs`: `impl Permanent for Glue`: when `heap::routed()` (make it `pub(crate)`) keep the
  default (a replacement malloc still sees setlocale's allocations: D-X.156 unchanged); else a bump arena of
  lazily mapped page chunks (16 KiB first, doubling to 1 MiB; a request larger than a chunk gets its own page-rounded
  chunk), its state under a `RawMutex` taken only inside global_locale's LOCK (document; fork covered by LOCK).
  Unsafe (writing `T` / making the slice) with `// UNSAFE(slice)` comments as tb.rs does; new rare code `#[cold]`.
* gettext's two leaked records (`intl::record_for`, `intl::intern`) may take the same arena through
  `niji_legacy::intl::Host` (Catalogs) if cheap; else note them.
* Loader state (item 2): NOT in this story (loader redesign or a COPY-only lookup in the executable's dynsym);
  record the cost and recommendation in a pending decision record (`records new decision D-NEW --series D-7
  --story LOCALE-ADDRESS-SPACE`) and as RULING NEEDED/next step.

## Exact next step

1. Implement the design; `cargo xtask clippy` + `static` on r1 (sync with `sync_role.sh`).
2. `cargo xtask so` job, rerun `measure.sh` (expect en_US anon +576 only, no heap growth) and a 200k-entry need.
3. Test: in `crates/core/tests/heap_meta_products.rs`, a same-run comparison: VmSize at exit of `find d3` in
   en_US.UTF-8 minus C, products vs the system's loader (the reference), allowed over the reference's = the loader
   state (measured, stated) + a stated margin; skip when no archive/en_US. Plant: `TryVec`/`TryBox` leak restored in
   `intern_locked` (docs/plants.md line, `cargo xtask plants`).
4. Gate (`cargo xtask affected --run`), plants, `cargo xtask test`, full-profile hot_text_products, precheck,
   report json, handback-check.
