# PRINTF-NO-HEAP-2 (handoff; run 1, end of leg 2)

State: branch `task/printf-no-heap-2` (stacks on task/printf-no-heap; base origin/next 28843dea1). Code head
`ef49de906`; HEAD = the commit that adds this note and the report. Role mei:~/w/s3 (Skylake-SP).
Predecessor note: handoff/PRINTF-NO-HEAP.md. Decisions in force: D-12.32 (no-heap printf design), D-12.33
(swprintf truncation is E2BIG). No new record (the one-pass spill and the cold call are implementation details).

## Commits (this story)
- `1dc188f9a` long fields in one pass: `Capped` (simple floats) and `Building` (general path) spill to `out` once a
  piece overflows, when the width fits the inline room; `%f`/`%Lf` of 1e300 / 2^1000 is one pass.
- `918d47cdf` plants file `handoff/PRINTF-NO-HEAP-2.plants` (15: 9 inherited, 6 on the spill logic).
- `db111d295` `format_at` `#[inline(never)]` (gained little: s_d_colon 975 -> 972.7, `%d` 490.8 -> 491.7).
- `ef49de906` `format_routed` calls the general path through `format_general` (`#[cold] #[inline(never)]`).
  Cause, from disassembly of base and branch cbench_opt (`~/work/printf-no-heap-2/dis-*.s`, script dis.sh):
  the only source change reaching `format_routed<ToBuffer>` (40 KB: four render_simple instantiations inlined)
  is the general path; as a plain call LLVM kept the sret pointer in `%rdi` for it and saved it late on the
  other paths and merged exits (bare `"%s"`: +1 mov, +1 jmp). Callee lists of both builds otherwise identical.

## cbench instructions/call (`cargo xtask bench --compare --counters --runs 3 cbench_opt cbench_stdio`)
before = 28843dea1 (predecessor's measure-before.tsv; a fresh base rerun `printf-no-heap-2-basebench`, cbench_opt
--runs 1 in RAM, agrees within 0.3), prev = PRINTF-NO-HEAP 91091cabf, after = ef49de906 (`printf-no-heap-2-cbench-3`,
~/work/printf-no-heap-2/measure-3.tsv).

| row | before | prev | after | vs before |
|---|---|---|---|---|
| snprintf_d / ld_large / x08 | 489.5 / 549.4 / 530.5 | 490.5 / 550.5 / 531.4 | 487.5 / 547.5 / 527.6 | -0.4 / -0.35 / -0.55% |
| snprintf_s_path (`"%s"`) | 229.2 | 229.2 | 231.2 | **+0.87%** |
| snprintf_s_d_colon | 961.0 | 974.2 | 941.2 | -2.06% |
| snprintf_ls_line | 2420.9 | 2455.2 | 2379.4 | -1.71% |
| sprintf_chk_ls_nlink | 760.6 | 762.8 | 699.6 | -8.02% |
| snprintf_mixed / f2 / g / e | 1997.6 / 1110.3 / 1286.3 / 1582.3 | | 1924.7 / 1060.3 / 1237.3 / 1532.3 | -3.2 .. -4.5% |
| snprintf_f_large (`%f` 1e300) | 18443.6 | 52845.4 | 17812.5 | -3.42% |
| fprintf_ls_line / du_line | 1877.2 / 842.4 | | 1877.2 / 842.3 | 0.00% |
| stdio snprintf_d / mixed | 489.5 / 2014.2 | | 487.5 / 1941.3 | -0.4 / -3.6% |
| stdio fprintf_d | 633.7 | 633.9 | 639.5 | **+0.92%** |
| stdio fputs_32 / putc | 125.0 / 23.2 | | 125.0 / 23.2 | 0.00% |
| stdio fwrite_4k | 1087.7 | 1089.1 | 1088.1 | +0.04% (no printf code on it) |

OPEN (RULING NEEDED): `"%s"` +2.0 and fprintf_d +5.8 instructions per call. Both are register-allocation and
layout effects in `format_routed` (ToBuffer, ToStream), not added work; each attribute change moved them between
rows (fprintf_d was 634.2 before ef49de906). A third attribute tweak is a rut (RUT RULE); the robust fix is
structural: a small hot dispatcher with the render_simple instantiations out of line, so the simple paths no
longer share one 40 KB function with the rest (follow-up story; measure with
`cargo xtask bench --compare --counters --rows snprintf_s_path,fprintf_d,... cbench_opt cbench_stdio`).

## #7764 probe (mei ~/work/printf-no-heap-2/pfnh2.c, ref.txt; counting malloc replacement, each row run with
allocation allowed and again with every allocation refused)

| row (reference, glibc 2.39) | allocator calls | refused: same result |
|---|---|---|
| C locale `%'f` 1e300; `%If` 1e300 | 0 | yes |
| en_US.UTF-8 `%'f` 1234567.5, 1e300; `%'.2f` -1e300; `%'Lf` 1e300L; `%'e`; `%'g`; `%'d` | 0 | yes |
| en_US.UTF-8 `%If` 1e300; `%I'f` 1e300 | 0 | yes |
| en_US.UTF-8 `%.100000f` (control) | 1 | no: -1, ENOMEM |
| register_printf_specifier('W') itself | 1 | (registration, not a call) |
| after registration: `%W`, `%W %d`, `%d`, `%f` 1e300 | 0 | yes |

FOLLOW-UP: libniji still allocates where the reference does not: (a) grouped or `I` floats whose text passes 128
bytes (`float_text_general` builds in `InlineVec<u8, 128>`, which grows); (b) any call while a conversion is
registered (unbounded parse with Hooks; `Custom` outputs `TryVec<u32>`; vfwprintf collects in `CollectWide`).
Both need streaming (grouping into a `Units` sink; handler output straight to `out`). Routing is right for
`%.100000f/e/g` and positional past 48 pieces (the reference allocates, predecessor probe).
INFORMATIONAL: the reference's `%If` of 1e300 in the C locale returned 1 (en_US: 308); libniji not compared.

## Proofs on ef49de906
- Plants `printf-no-heap-2-plants`: 15 CAUGHT, 0 SURVIVED, 0 WARN (handoff/PRINTF-NO-HEAP-2-plants.txt).
- Bench `printf-no-heap-2-cbench-3` (table above).
- Direct gate `printf-no-heap-2-gate`: 20 suites ok, 11873 passed, 0 failed; conformance 137/0; peak 15G.
- `printf-no-heap-2-xtask-test`: 28 suites ok, 12572 passed, 0 failed; conformance 147/0.
- clippy (`--workspace --all-targets --all-features -D warnings`) and `cargo xtask static`: clean on ef49de906.
- **RED** `printf-no-heap-2-hot-text` (NIJI_FULL_PROFILE so + hot_text_products): 32 passed, 1 failed:
  `init_zero_fault_calls_hold_at_every_text_phase`: "pre 0x2000: strings 1 faults, want 0". The base 28843dea1
  passes the same command (`printf-no-heap-2-base-hot-text`, RAM copy: 33/33), so this branch causes it (start-up
  text layout: the bigger printf code shifts what shares the `strings` window's pages). Not chased (wall time).
  Next: diff `target/product/fast/layout-bins.tsv` and the release hot-text windows against the base; look for
  new printf functions placed in or next to the start-up text (cold paths such as `format_long`, `render_from`,
  `Capped::overflow`, `simple_float_long`, `format_general` should be in the cold text).
- precheck on ca72b43db: RED only for printf-no-heap-2-hot-text (other jobs count: prose-only difference).
- SUPERSEDED (older heads or diagnostics): printf-no-heap-2-cbench-1 (1dc188f9a), printf-no-heap-2-cbench-2
  (db111d295), printf-no-heap-2-basebench (the base revision, diagnostic), printf-no-heap-2-t1/t2 (1dc188f9a).

## Next step
Ruling on the two rows; then the structural follow-up above and the #7764 follow-ups (a), (b).

## Refused
None.
