# PRINTF-NO-HEAP (handoff; run 1, end of leg 1)

State: branch `task/printf-no-heap` (base origin/next 28843dea1), WIP commit on top. Role mei:~/w/s3.
NOT shippable yet: `Field` now truncates past 256 units, and `render` does not yet re-produce long fields.

## Reference probe (done; scratch `pfnh.c`, mei ~/work/printf-no-heap/ref.txt)
Counting + refusing malloc replacement, `-fno-builtin -fno-printf-return-value`. Reference: 0 calls and the same
result under refusal for every row of `crates/boundary/tests/printf_no_heap.rs` marked `Ref::None` (see its doc
comment, which is the probe record). Reference ALLOCATES (ENOMEM under refusal): `%.100000f/e/g` (1 call), 16+
positional args (16: 1 call, 64: 4). 12 positional: 0. Sequential up to 100: 0. swprintf: E2BIG (7) on EVERY
truncation (`L"hello"` into 4 too), not only %100000d; spec silent -> match it (change `ToWideBuffer`'s
`Errno::EOVERFLOW` to `E2BIG` in boundary printf.rs `vswprintf`; check tests asserting 75; D-row needed via
`ops reserve --story PRINTF-NO-HEAP D-12`).

## libniji BEFORE (job printf-no-heap-before2 on mei, base code + new test)
Allocating where the reference does not: %f 1e308 (3 calls), %f -1e308, %Lf 2^1000 (5), %Lf 2^13000 (8), %.400e (5),
%.10000f (10), %.1100f 1e-300 (7), %'f 1e308 (10), %+.300f (5), %.500d (2), %.100000d (10), %.100000x, %#.600o,
%+0600.500d, %'.500d, %.50000s (2), %ls 10000 chars (7), %.*d 100000 (10), pos 12 (2); EVERY swprintf row (the
format is copied into a TryVec: boundary printf.rs `vswprintf` and `vfwprintf`). Zero already: seq 8..100, widths,
%100000s, %s long, %300c, %p, %La.

## Design (in progress)
1. DONE (WIP commit): core printf.rs `Units<U>` trait (Field, Stream, TryVec impl), `Field<U>` = non-spilling
   inline 256 units that counts past its room (`complete()`, `len()` = total), `Stream` (to `Out`, inserts
   `zeros` pad zeros after `prefix` units), `fill`, `zeros`. `Unit::from_text/from_wide/from_byte` take
   `out: &mut impl Units<Self>`. failinject gained `calls()` (selftest only).
2. NEXT: in `render`, move the per-letter body production (the `match letter` arms d..B, f..A, c, s, p, m) into
   `fn field<U: Unit>(c: &Conv, letter: u8, prec: Option<usize>, args: &[Arg], env: &mut dyn Env, body: &mut impl
   Units<U>, zero_ok: &mut bool) -> Result<usize /*prefix_len*/>`; keep `n` and `_` arms in render. After it: if
   `!body.complete()` call a `#[cold] #[inline(never)] long_field` that writes pad spaces (right-justified), then
   `field` again into `Stream { prefix: prefix_len, zeros: pad if zero_ok&&!left }`, then `if zeros left
   zero_run`, then pad spaces if left; add its count to `total`. Integer precision zeros: replace the
   `for _ in digits.len()..min { U::from_text(env, b"0", ...) }` loop with `zeros(body, min - digits.len())`.
3. Floats: in `field`, fast path (no group-with-separator, no i18n, not hex): format into `Text` through a capped
   sink (stores only while `text.len()+b.len() <= INLINE_TEXT`, else sets `over`, no spill); if `over` (or
   precision >= INLINE_TEXT), cold `long_float`: a byte sink buffering 128 bytes and flushing with
   `U::from_text(env, buf, None, body)` (flush BEFORE a put that does not fit; never split one put: the radix
   arrives as one put), the `+`/space first; prefix = `(neg||plus||space) as usize`. Route `%'f` to the fast path
   when `env.thousands()` is empty or grouping inactive (C locale). Grouped long floats in a grouping locale stay
   allocating: record as a gap.
4. legacy `printf_registry.rs` `push_text`: call `U::from_text(env, text, None, body)` on the TryVec directly
   (Field now truncates).
5. Positional: make `Parsed<const N: usize = INLINE_PIECES>` with `bounded`/`full` flags; in bounded mode a piece
   or class push past N sets `full` and returns Err; boundary `format_at` parses bounded Parsed<16>, on `full`
   calls a cold path with an unbounded `Parsed<48>` (covers up to ~23 positional args; the reference itself
   allocates from 16). Generic `render`/`parse_into` over N.
6. Wide: add `conv::wide_units(&[i32]) -> &[u32]` (UNSAFE(slice), same size/alignment) and drop the TryVec copies
   in `vswprintf` and `vfwprintf`.
7. Then: test (all rows must pass), `swprintf_result_does_not_fit_is_e2big`, plants in docs/plants.md (one of the
   8 rows allocating again: e.g. make `zeros` unused/Field spill; `%d` using a heap table), cbench printf rows
   before/after (`cargo xtask bench --compare --counters` printf rows), clippy, static, direct gate, precheck.

## Not to redo
The reference probe (record in the test's doc comment). The before table above.

## Refused
`... ; rm -f ~/.claude/projects/.../tool-results/bh377ycbk.txt` (deleting a harness output file, combined with a
probe rerun): refused by the auto-mode classifier; the probe was then rerun alone, the file left in place.
