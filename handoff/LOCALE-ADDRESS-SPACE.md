# LOCALE-ADDRESS-SPACE: handoff (run 1, leg 2: fix, test and plants done)

State: branch `task/locale-address-space` (base origin/next `fb12f4d3e`), build role keke `~/w/r1` (Zen 1, 60 vCPU).
Brief: `~/state/briefs/LOCALE-ADDRESS-SPACE.md`. Decision: pending record
`records/pending/LOCALE-ADDRESS-SPACE-1.toml` (D-NEW, series D-7; CI assigns the id). Plants file:
`handoff/LOCALE-ADDRESS-SPACE-plants.txt`. Scratch scripts: keke `~/work/locale-address-space/` (`measure.sh`,
`smaps_sum.py`, `trace.sh`, `refill.sh`; outputs `m0` = base, `m1` = after).

## What was done

* `niji::mem::Permanent` (default: the heap leak; `OnHeap`), `LocaleView::new_in`, `global_locale::Objects:
  Permanent` (view, items, `Tables` and name go through it), and in `boundary/src/locale.rs` the products' arena
  (`perm_place`: private page chunks, 16 KiB first, doubling to 1 MiB, own `RawMutex` taken only inside the interning
  lock; with a replacement malloc the default heap path, D-X.156). The unsafe placement helpers are in `tb.rs`
  (`uninit_slice_at`, `uninit_at`, `filled`; category `slice` is only permitted there).
* Test `heap_meta_products::a_utf8_locale_costs_no_more_address_space_than_the_references` (same run: products vs the
  system loader, en_US.UTF-8 minus C, allowance = reference's + products' C.UTF-8 cost + 64 KiB).

## Measured (keke, find 4.9.0, 3 files on tmpfs; VmSize at exit KiB, fixed layout)

| | C | en_US.UTF-8 | C.UTF-8 |
|---|---|---|---|
| libniji before (m0) | 12896 | 18512 (+5616) | 13472 (+576) |
| libniji after (m1) | 12896 | 16480 (+3584) | 13488 (+592) |
| reference | 3736 | 6724 (+2988) | 7152 (+3416) |

The least `ulimit -v` (3 ASLR runs and fixed, spread 0) barely moves: libniji C 8716, en_US 12416 -> 12408, C.UTF-8
9304 -> 9272; the reference 3736 for all (under the limit its setlocale fails and it runs in C). de_DE is not
installed on keke (only en_US.utf8, C.utf8). Left over: +596 KiB anon = dlopen loader state (528 KiB table map +
44 + 4 KiB image views) + 16 KiB arena chunk: see the record (loader change, its own story).

## Next step

Plants (`locale-address-space-plants`), then on the final head: static + records check, direct gate
(`cargo xtask affected --run`), `cargo xtask test`, full-profile `hot_text_products`, precheck, report json,
handback-check.
