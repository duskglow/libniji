# ICONV-CJK-STARTUP(-2, -3): take back the start-up cost the CJK tables added

**State.** Branch `task/iconv-cjk-startup-3` (continues -2's `task/iconv-cjk-startup-2`; origin/next 90390071c merged
in 0c713424a), role mei `~/w/s4` (Skylake-SP class). Scripts on mei in `~/work/iconv-cjk-startup-3/` (gc.sh + gr.py:
static getconf link map and reference chains; su.sh: start-up counts; pf.sh + fa.py: per-page fault trace,
`PROG=m3` for the 3-library program; v1.sh, st.sh, cnt.sh: this run's jobs). Decisions in force: D-6.19 (tables
hold offsets), D-6.21 (wide streams call the iconv codecs through the stream's codec value).

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
the merge of next, not from D-6.21 (no pre-merge pair measured in this run). **Zen 1 class not measured** (no
role there in this run).

## Next steps

1. 3-library +1 fault: libc's RW/bss layout puts the start-up writes of `__niji_libc_early_init` and
   `process::secure` on separate pages; a start-up data layout story (cluster those objects) rather than iconv.
2. Zen 1 class: run su.sh / pf.sh there on this head's products; then tighten DYN_TRUE_BOUND from both classes.
