# PRINTF-NO-HEAP-2 (handoff; run 1, end of leg 1: REFRESH, code done, proofs running/pending)

State: branch `task/printf-no-heap-2` (stacks on task/printf-no-heap; base origin/next 28843dea1), HEAD = the commit
that adds this note (code head `db111d295`). Role mei:~/w/s3 (Skylake-SP). Predecessor note: handoff/PRINTF-NO-HEAP.md.
Decisions in force: D-12.32 (no-heap printf design), D-12.33 (swprintf truncation is E2BIG). No new record needed
(the one-pass spill is an implementation detail of D-12.32; it changes no behaviour).

## Done this leg (do not redo)
- `1dc188f9a` step 1: long fields in one pass. Simple floats: `Capped` (crates/core/src/printf.rs) spills the kept
  text and every later piece to `out` once a piece overflows, when `width <= INLINE_TEXT` (an overflowing text is then
  longer than the width: no padding); a wider field still takes `simple_float_long`. General path: `render_from` builds
  each conversion through `Building` (wraps `Field` + `out`; same fast copy; cold `past()` spills when
  `width <= INLINE_FIELD`); a spilled field just adds `body.len()` to the total. `%Lf` of 2^1000 is one pass now.
  Test rows added to `long_fields_have_their_full_text` (crates/boundary/tests/printf_no_heap.rs): `%f|`, `%-120f|`,
  `%200.2f|`, `%320f|` of +-1e300; `%250Lf|`, `%-320Lf|` of 2^1000; `%2$s%1$.300Lf|`. Tests green on mei
  (printf-no-heap-2-t1: printf_no_heap 3 passed; printf-no-heap-2-t2: `cargo test -p niji --lib printf` 14 passed).
- `db111d295` step 2: `format_at` `#[inline(never)]`. Cause found with nm on the cbench_opt binary: no
  `format_at<ToBuffer>` symbol, the whole general path was inlined into `format_routed<ToBuffer>` (0xab25 bytes),
  which also holds render_simple's loop; perf (instructions:u) of snprintf_s_d_colon: 52% in format_routed<ToBuffer>.
  NOT YET MEASURED: `printf-no-heap-2-cbench-2` (started ~03:47Z on db111d295) is that measurement.
- Plants file `handoff/PRINTF-NO-HEAP-2.plants`: 15 plants (inherited 9, nh-simple-float-spill retargeted to the new
  sink; 6 new on the spill logic). NOT YET RUN.
- #7764 probe (step 3), mei ~/work/printf-no-heap-2/pfnh2.c, ref.txt (counting malloc replacement; each row run
  with allocation allowed, then with every allocation refused):

| row (reference, glibc 2.39) | allocator calls | refused: same result |
|---|---|---|
| C locale `%'f` 1e300; `%If` 1e300 | 0 | yes |
| en_US.UTF-8 `%'f` 1234567.5, 1e300; `%'.2f` -1e300; `%'Lf` 1e300L; `%'e`; `%'g`; `%'d` | 0 | yes |
| en_US.UTF-8 `%If` 1e300; `%I'f` 1e300 | 0 | yes |
| en_US.UTF-8 `%.100000f` (control) | 1 | no: -1, ENOMEM |
| register_printf_specifier('W') itself | 1 | (registration, not a call) |
| after registration: `%W`, `%W %d`, `%d`, `%f` 1e300 | 0 | yes |

  So libniji still allocates where the reference does not: (a) grouped or `I` floats whose text passes 128 bytes
  (`float_text_general` builds `grouped`/`text` in `InlineVec<u8, 128>`, which grows to the heap); (b) any call while
  a conversion is registered (unbounded parse with Hooks; `Custom` outputs are `TryVec<u32>`; vfwprintf collects into
  `CollectWide(TryVec)`). Both need a streaming redesign (grouping into a `Units` sink; handler output straight to
  `out`): FOLLOW-UP, not this run. Routing is right for `%.100000f/e/g` and positional past 48 pieces (reference
  allocates, predecessor probe).

## cbench instructions/call (`cargo xtask bench --compare --counters --runs 3 cbench_opt cbench_stdio`)
before = 28843dea1 (~/work/printf-no-heap/measure-before.tsv), prev = PRINTF-NO-HEAP 91091cabf, after-1 = 1dc188f9a
(printf-no-heap-2-cbench-1, ~/work/printf-no-heap-2/measure-1.tsv). Compare: `python3 ~/work/printf-no-heap-2/cmp.py
~/work/printf-no-heap/measure-before.tsv ~/work/printf-no-heap/measure-after.tsv ~/work/printf-no-heap-2/measure-2.tsv`

| row | before | prev | after-1 |
|---|---|---|---|
| snprintf_f_large | 18443.6 | 52845.4 | 17829.8 (-3.3%) |
| snprintf_s_d_colon | 961.0 | 974.2 | 975.0 (+1.46%) |
| snprintf_ls_line | 2420.9 | 2455.2 | 2457.4 (+1.51%) |
| sprintf_chk_ls_nlink | 760.6 | 762.8 | 764.1 |
| snprintf_d / ld_large / x08 | 489.5 / 549.4 / 530.5 | 490.5 / 550.5 / 531.4 | 490.8 / 550.7 / 531.5 |
| snprintf_e / f2 / g / mixed | 1582.3 / 1110.3 / 1286.3 / 1997.6 | | 1540.3 / 1067.7 / 1244.9 / 1960.0 |
| fprintf_* / stdio rows | | | within +0.1% (fwrite_4k 1087.7 -> 1088.4) |

## Exact next step (leg 2)
1. `~/bin/jobwait printf-no-heap-2-cbench-2`; run the cmp.py line above. If every printf/stdio row is at or below
   before: go on. If s_d_colon/ls_line are still above: `nm -S --size-sort ~/w/s3/target/niji/bench/cbench_opt | grep
   format_routed` and compare with 0xab25; look for what else got inlined (render_from/format_long are separate
   symbols already).
2. Start plants (role busy until the bench ends): `setsid nohup ~/bin/jobrun printf-no-heap-2-plants plants -- sh -c
   '. ~/.cargo/env; cd ~/w/s3 && cargo xtask plants handoff/PRINTF-NO-HEAP-2.plants --dir
   /mnt/zram/claude/printf-no-heap-2/plants-1 --timeout 600' </dev/null >/dev/null 2>&1 &` (nothing chained before
   setsid over ssh: a chained `mkdir` made the ssh call hang this leg).
3. Direct gate `printf-no-heap-2-gate` (`cargo xtask affected --run` under ~/bin/gate 16G), then start-up checks
   (`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test
   hot_text_products`), one build at a time on s3; foreground clippy (`cargo clippy --workspace --all-targets
   --all-features -- -D warnings`) and `cargo xtask static`; `cargo xtask precheck --jobs <labels> --head <sha>`.
4. Report handoff/reports/PRINTF-NO-HEAP-2.json (+ "jobs"), push, `ops handback-check`.

## Open / gaps
- FOLLOW-UP (#7764): (a) and (b) above, with the probe rows.
- INFORMATIONAL: the reference's `%If` of 1e300 in the C locale returned 1 (en_US: 308); libniji not compared.

## Refused
None this leg.
