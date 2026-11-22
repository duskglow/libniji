# ICONV-CJK-STARTUP(-2, -3, -4): take back the start-up cost the CJK tables added

**State.** Branch `task/iconv-cjk-startup-4` (= -3's `task/iconv-cjk-startup-3` head 6863fc274 plus this note; no
code change in run 4; origin/next 90390071c merged in 0c713424a; next fa79409c7 merges cleanly, not merged), role keke
`~/w/n2` (Zen 1 class). Run 3's scripts on mei in `~/work/iconv-cjk-startup-3/` (gc.sh + gr.py: static getconf link
map; su.sh: start-up counts; pf.sh + fa.py: per-page fault trace). Run 4's on keke in `~/work/iconv-cjk-startup-4/`
(ab.sh / ab2.sh: heap A/B release / fast; z1.sh + su.sh: Zen 1 start-up counts; seq.sh: gate then `cargo xtask
test`). Decisions in force: D-6.19 (tables hold offsets), D-6.21 (wide streams call the iconv codecs through the
stream's codec value).

## Run 4: the CI red (heap address space) is not this branch

CI red on train tip ff5ae1612 (with ZIV-TEST-ISOLATION), Skylake-SP class, fast profile, unpinned, among 7,058
tests: `heap_meta_products::the_heap_keeps_address_space_near_its_use`, refill +39492 KiB over start, bound 39236
(zero margin, ruling #7441); malloc1 +4420 and alloc +28996 as on next. The branch changes no heap code. A/B on the
Zen 1 class host, oracle `heap_address_space.c` linked exactly as the test links it, the two sides interleaved:

| products | runs | start | malloc1 | alloc | refill | realloc |
|---|---|---|---|---|---|---|
| merge base 90390071c, release | 20 pinned node 0 + 10 unpinned | 7056 | +4420 | +28996 | +39236 | +41288 |
| this branch, release | 20 pinned node 0 + 10 unpinned | 7056 | +4420 | +28996 | +39236 | +41288 |
| merge base 90390071c, fast | 20 pinned node 0 + 10 unpinned | 7704 | +4420 | +28996 | +39236 | +41288 |
| this branch, fast | 20 pinned node 0 + 10 unpinned | 7704 | +4420 | +28996 | +39236 | +41288 |

Every run identical, both sides (120 runs). Next's fast products were built by `WL_PROFILE=fast wl_prep.sh` in
`~/wl/src/90390071c…/target/niji`. The host's installed `~/wl/bin/wl_prep.sh` failed at its cache copy because
it still names `target/liella/` (the repo's validation/wl_prep.sh does not), so the A/B used the build tree's copy
(profile marker `fast`). The empty cache dir that call left was removed. Refill is exactly at its bound, so any extra slab (256 KiB) during the step fails
it. This branch does not add that slab on this host. Not run on the Skylake-SP class: this run has no role there.
The +256 KiB therefore belongs to the heap line (INFORMATIONAL in the report).

## Done

* Runs 1-2: see D-6.19 and -2's report (handoff/reports/ICONV-CJK-STARTUP-2.json): offsets in the tables, the
  crates-2 red explained, the ld.so `fini_all` stack probes moved cold (-4 faults).
* Run 3, merge (0c713424a): next's CP949/UHC kept in cjk.rs; cjk_tables.rs regenerated with `encodings-gen`, not
  hand-merged. The generator now emits one index constant per set and `Which::set` indexes `SETS` through them
  (next put CP949 third in the generator but last in `Which::ALL`). CP949 holds offsets like the others:
  **libc.so.6 RELATIVE 1,438 after the merge (full profile), the same as before it** (fast profile: 2,225, both
  before and after this run's code change).
* Run 3, getconf (fbf258b23, D-6.21): `putwc`/`fgetwc`/`encoded_len` called `iconv::Conv::convert` by name for
  `,ccs=` streams, so any program reaching the wide stream path (perror) linked every built-in codec. A `,ccs=`
  codec now holds `Conv::convert` as a value (`ConvertFn`, set in `Codec::charset`); equality ignores it. Test
  `wide_stderr_links_no_cjk_codec` (iconv_reloc_products.rs): static PIE of perror + fputwc(stderr) keeps no
  `cjk` section; a `w,ccs=BIG5` control must keep them. Plants 2/2 CAUGHT (handoff/ICONV-CJK-STARTUP-3-plants.txt).
* The 3-library +1 fault, attributed (pf.sh, 3 runs identical, ASLR off): anon 10 -> 11. In 21.5,
  `process::secure` and the write before it share one anonymous page; in this head `__niji_libc_early_init` and
  `process::secure` write to two pages two apart (libc's RW/bss layout, D-6.21 does not touch it). Not fixed.

## Results (mei, Skylake-SP class, full profile, su.sh, median of 101, ASLR off; user instructions / faults)

| program | 21.5 | 21.6 | this head (fbf258b23) |
|---|---|---|---|
| empty main | 741,245 / 51 | 742,653 / 56 | 722,754 / 51 |
| 3 libraries | 984,141 / 65 | 985,550 / 71 | 962,283 / 66 |
| clock_gettime | 763,337 / 54 | 764,745 / 59 | 744,867 / 54 |
| BIG5 round trip | 46,956,779 / 415 | 763,347 / 72 | 743,847 / 69 |

Static getconf text: merged head before the change 1,486,604 B, after 579,796 B (fast profile, same link);
full profile 549,396 B (21.5: 903,524, -2's figure). The instruction drop against 21.5 (-18.5k) comes mostly from
the merge of next, not from D-6.21 (no pre-merge pair measured in this run).

**Zen 1 class (run 4, keke, release profile, su.sh, median of 101, ASLR off; merge base 90390071c's primed
products vs this head):**

| program | merge base 90390071c | this head | glibc 2.39 (host) |
|---|---|---|---|
| empty main | 725,801 / 57 | 722,382 / 51 | 101,055 / 48 |
| 3 libraries | 965,274 / 72 | 961,860 / 66 | 136,284 / 64 |
| clock_gettime | 747,942 / 61 | 744,525 / 55 | 103,194 / 50 |
| BIG5 round trip | 746,571 / 73 | 743,480 / 69 | 137,094 / 62 |

libc.so.6 RELATIVE 1,606 -> 1,438. The two classes agree on this head within one fault (Skylake-SP: 51, 66, 54,
69). `hot_text_products` (release, Zen 1): 33 passed; the dynamic start-up program reads 58 against
DYN_TRUE_BOUND's 67, so the non-blocking FAULT-COUNT REPORT "more than 4 under the bound" fires. The bound is the
higher of the two classes per profile (D-X.103's rule) and the Skylake-SP level of that program on this head is not
measured, so the bound is left as it is (next step 2). Fast profile, Zen 1: not read in this run (the gate captures
a passing test's output; run `cargo test -p niji --test hot_text_products -- --nocapture` on fast products).

## Next steps

1. 3-library +1 fault: libc's RW/bss layout puts the start-up writes of `__niji_libc_early_init` and
   `process::secure` on separate pages; a start-up data layout story (cluster those objects) rather than iconv.
2. DYN_TRUE_BOUND (hot_text_products.rs): measure the dynamic start-up program on the Skylake-SP class on this
   head (fast and release), then tighten [73, 67] to the higher class per profile (Zen 1: fast not read, 58 release).
3. Heap refill +256 KiB on the Skylake-SP class (CI, unpinned, under load): heap line, not this branch (A/B above).
4. Static programs linked WITHOUT `--gc-sections` still carry the CJK tables: the heap oracle (malloc, realloc,
   snprintf; no iconv) links `cjk_tables::CODES` (340,096 B) and `CELLS` (198,212 B), because they share one
   libc.a member (a codegen unit of the core crate) with `realloc` (`~/work/iconv-cjk-startup-4/gc4.sh` +
   chain.py: map inclusion chain). That is libc.a's member granularity, not an iconv reference (getconf links
   with `--gc-sections`, where run 3's fix holds). A libc.a granularity story (the tables in a member of their own,
   or more codegen units for the archive build), measured on static text size.
