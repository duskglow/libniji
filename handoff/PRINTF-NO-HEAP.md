# PRINTF-NO-HEAP (handoff; run 1, end of leg 2)

State: branch `task/printf-no-heap` (base origin/next 28843dea1). Role mei:~/w/s3 (Skylake-SP host).
Code is DONE and committed (e592c348a "printf: produce long fields, floats and long formats without the heap");
records D-12.32 (design) and D-12.33 (swprintf E2BIG) added after it (ids reserved via ops reserve).

## Done (do not redo)
- Reference probe: scratch `pfnh.c` (mei ~/work/printf-no-heap/, outputs ref.txt, ref2.txt). Record in the doc
  comment of `crates/boundary/tests/printf_no_heap.rs`. Leg 2 added rows, all 0 allocations on the reference:
  W seq 100, W %m, `%'d `x100, `%*d `x50, `%+.3d|`x100, `%.5000a`, `%.300a`, `%m`.
- libniji AFTER: every `Ref::None` row makes 0 allocator calls and gives the same result under refusal
  (job printf-no-heap-t4: 3 tests pass: printf_no_allocation_where_the_reference_makes_none,
  swprintf_result_does_not_fit_is_e2big, long_fields_have_their_full_text). Only `pos 16+`/`%.100000f/e`
  (reference allocates too) still allocate (pos 100: 10 calls; pos 16, 24 now 0 since they fit the 48-piece parse).
- Design (D-12.32): core printf.rs `Field` counts past 256 units; `field()` + cold `long_field()` re-produce long
  fields to `Out` via `Stream`; floats via `float_units`/`ToUnits` (128-byte flush buffer); simple-format floats
  `Capped` + cold `simple_float_long`; bounded `Parsed` (`more`, `positional`, `bounded`; `parse_from`,
  `render_from`); boundary `format_at` -> cold `format_long` (sequential: parts; positional: `Parsed<48>`);
  `conv::wide_units` (UNSAFE(slice)) replaces the wide format copies; `%m` via inline `ErrorText`;
  `InlineVec::extend_inline` (mem.rs). swprintf truncation: E2BIG (D-12.33).
- Jobs started on mei on code head e592c348a: `printf-no-heap-plants` (9 plants, file ~/work/printf-no-heap/plants.txt,
  dir /mnt/zram/claude/printf-no-heap/plants-1), `printf-no-heap-gate` (direct tier, 266 targets + 137 conformance).

## Next steps (leg 3)
1. `~/bin/jobwait printf-no-heap-plants printf-no-heap-gate` on mei; read `~/bin/test-summary ~/logs/printf-no-heap-gate.log`
   and the plants verdicts (all CAUGHT expected; list any WARN). Fix reds (oracle `printf_diff` may show the swprintf
   errno change: the reference gives 7 there too, so it should agree now).
2. cbench printf rows, before (origin/next) and after: `cargo xtask bench --compare --counters <printf benches>`
   (find names: `grep -n printf benchmarks/` / docs/benchmarks.md). Hot rows (%d, %s, %x, short %f) must not be slower.
3. The head after e592c348a only adds records + a doc comment (D-NEW -> D-12.33 in boundary printf.rs). Sync, run
   `cargo xtask static` (records check) and `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   on the final head; if precheck wants the gate on the final head, rerun the gate (warm).
4. `cargo xtask precheck --jobs <labels> --head <sha>`; write handoff/reports/PRINTF-NO-HEAP.json (impact: correctness
   rows fixed = the 19 before-rows + every swprintf row; coverage: 3 tests, 9 plants; perf from step 2), push,
   `ops handback-check`.

## Open / gaps (for the report)
- Still allocating (reference unknown or allocates): registered conversions; positional formats past 48 pieces;
  long floats with `'` in a locale with separators, or `I`. INFORMATIONAL.
- Wide `%100000d` into 256 units writes nothing past the buffer and reports E2BIG; unchanged.

## Refused (leg 1)
`... ; rm -f ~/.claude/projects/.../tool-results/bh377ycbk.txt` (deleting a harness output file, combined with a
probe rerun): refused by the auto-mode classifier; the probe was then rerun alone, the file left in place.
