# ICONV-CJK-STARTUP(-2): take back the start-up cost the CJK tables added

**State.** Branch `task/iconv-cjk-startup-2` (continues `task/iconv-cjk-startup`, base origin/next 83f91cb05; next
has moved 58 commits since, not merged), role keke `~/w/n2` (Zen 1 host). Scripts and kept products on keke in
`~/work/iconv-cjk-startup-2/` (pf.sh + fa.py: per-page fault trace; gc.sh + gr.py: getconf link map and section
reference chains; v1.sh: the verification run; hd00 = head before the loader fix, hd01 = final code).

## Done (both runs)

* Run 1 (f4b9f1482, D-6.19): iconv tables hold offsets, not addresses; libc.so.6 RELATIVE 1,573 -> 1,438; test
  `iconv_reloc_products`; plants 3 CAUGHT (handoff/ICONV-CJK-STARTUP-plants.txt).
* Run 2, the crates-2 red (#7438): **deterministic, not a flake.** The build tag (`source_hash_tag`) hashes
  everything under `crates/`, including the test file a5b0f172f changed. crates-2 tested products built at
  f4b9f1482 (its static step printed "libm.so.6 was not built from this tree"), while `xtask ldso-trace` built
  its prefixed interpreter with the new tag. That interpreter refused the stale libc.so.6 (D-27.22); the trace
  ended in `Versions::def_name_from` -> `io::fail_with` -> `exit` (the 77 names). Commit 20255f55d: `ldso-trace`
  first checks that libc.so.6 carries this tree's tag and says the products are stale (unit test
  `ldso_trace_takes_only_products_of_this_tree`). `build_native_ld` has the same gap; not changed here.
* Run 2, faults attributed (perf page-faults with data addresses, empty main, ASLR off, 3 runs identical):
  21.5 54 / 21.6 59 / hd00 58 / hd01 54 under perf. 21.6's +5 = **stack +4**, all touched by ld.so's
  `fini_all` at exit, and **anon +1** (libc's `__niji_libc_early_init` touches one more anonymous page; layout,
  not attributed further). Cause of the +4: `dlopen::fini_exit` (Finis + `exit_set`'s CAP = 512 tables, >24 KiB)
  was inlined into `fini_all`, and that frame's stack probes touch every page at every exit, `dlopen` or not.
  Commit f5f1ddbad moves the work after the `LOADER` test into one `#[cold] #[inline(never)]` function.
* Run 2, getconf, verified with a link map and section reference chains (gc.sh): not printf's wide conversions.
  The chain is `perror` -> `boundary::report::emit` -> `legacy::stdio_compat::write_text` -> `putwc` (the wide
  stream path) -> `iconv::Conv::convert` -> `cjk::four_cp` / the CJK tables (11 cjk sections kept). **Not fixed.**

## Results (keke, Zen 1, full profile, su.sh, median of 101, ASLR off; user instructions / faults)

| program | 21.5 | 21.6 | hd00 (run 1) | hd01 (final) |
|---|---|---|---|---|
| empty main | 740,923 / 51 | 742,331 / 56 | 739,630 / 55 | 739,574 / 51 |
| 3 libraries | 983,759 / 65 | 985,168 / 71 | 982,467 / 70 | 982,414 / 66 |
| clock_gettime | 763,044 / 55 | 764,452 / 60 | 761,751 / 59 | 761,695 / 55 |
| BIG5 round trip | 46,997,122 / 415 | 763,031 / 72 | 760,620 / 72 | 760,564 / 68 |

Instructions below 21.5 on every row; faults at 21.5 on empty main and clock_gettime; **3 libraries +1 over 21.5**
(the anon page, not attributed). getconf text 903,524 (21.5) / 1,340,708 (final), unchanged. Skylake-SP class
**not measured** (no role there in this run). hot_text_products: dynamic start-up-only program 58 faults (release),
bound 67 (bound not tightened: one host class measured).

## Jobs (keke, final code d338c5077; later commits are handoff/ only)

chk0 (clippy -D warnings + static on the inherited head) ok; v1 (clippy -D warnings, static, xtask unit test,
niji-ld dlclose / dlopen_process_objects / init_order, full-profile so, iconv_reloc_products, fault trace, su.sh
counts) rc 0; htp (full-profile hot_text_products, 33 ok, the prefixed trace included) rc 0; plants-2.
SUPERSEDED: plants (exit-skip pointed at init_order, which tests the order algorithm only: SURVIVED; tag-nul
CAUGHT); v1's hot_text_products section (its output pipe through `head -80` cut the run short; htp reran it).
`affected` escalates to FULL (xtask changed): FULL is the train's; `cargo xtask test` not run for that reason.

## Next steps

1. getconf / every static program that can reach `perror`: make the CJK codecs reachable only from where a CJK
   codec is chosen (iconv_open, setlocale of a CJK charset), e.g. `Conv::convert` dispatching CJK through a
   function pointer those install; measure static getconf text (target: 21.5's ~903 KB).
2. The 3-library +1 fault: trace pie3 with pf.sh (swap m0.c for m3.c) and attribute the anon page.
3. Skylake-SP class: run v1.sh's su.sh / pf.sh lines there; then tighten DYN_TRUE_BOUND from both classes.
