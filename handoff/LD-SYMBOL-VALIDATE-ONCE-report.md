# LD-SYMBOL-VALIDATE-ONCE report

**Result:** relocations (`GLOB_DAT`, `JUMP_SLOT`, `64`) bind through a quick walk over tables validated once when
the object's view is built, with versions compared by index; everything the validation does not vouch for keeps the
exact walk. Per imported symbol 1,525 -> 996 user instructions (reference 643, eager). The brief's 300 is not
reached; what bounds it is below. Decision record **D-27.96**.

## Commits (all mine, on origin/next 3bf4b7342)
* d60bce3ea Bind relocations over symbol tables validated once at map time (leg 1; not compiled in leg 1).
* aea01b407 Keep the quick symbol walk exact on rare tables; fix its version test (leg 2: review against the
  exact walk found a Bloom shift >= 32 and a unique definition's version-name error as divergences; the version
  test's fixture did not link).
* 0898cf520 Record D-27.96.

## Measurements (Zen 1 host keke, release products, `LANG=C.UTF-8`, median of 7; the audit's probe 5 method)
Per imported function never called (`-z now`; slope 0 -> 200 imports), user instructions:
before 1,525.0 (imp0 159,842, imp200 464,840) -> after 995.7 (160,615 / 359,746); reference 642.7.
Lazy binding: libniji binds eagerly either way (before 1,514.9, after 995.6; reference lazy 36.8).

The nine distribution binaries (each run by its loader explicitly; outputs identical to the reference's in both):

| binary | ref iu | niji iu before | niji iu after | ratio before -> after | ik after / ref | pf ref / before / after |
|---|---|---|---|---|---|---|
| true | 129,073 | 225,888 | 207,271 | 1.75 -> 1.61 | 850,996 / 613,873 | 50 / 61 / 61 |
| cat | 326,486 | 288,011 | 258,251 | 0.88 -> 0.79 | 1,185,028 / 1,204,605 | 78 / 91 / 91 |
| ls | 563,296 | 820,879 | 662,068 | 1.46 -> 1.18 | 1,972,166 / 1,562,352 | 102 / 142 / 135 |
| sort | 877,918 | 1,195,464 | 1,039,596 | 1.36 -> 1.18 | 2,335,493 / 2,011,198 | 198 / 227 / 228 |
| date | 352,984 | 332,619 | 294,334 | 0.94 -> 0.83 | 1,233,118 / 1,187,620 | 83 / 99 / 99 |
| wc | 339,180 | 314,730 | 277,966 | 0.93 -> 0.82 | 1,259,012 / 1,184,482 | 86 / 98 / 98 |
| sed | 908,798 | 1,213,814 | 1,002,301 | 1.34 -> 1.10 | 2,163,400 / 1,639,589 | 110 / 152 / 154 |
| grep | 586,023 | 650,467 | 557,371 | 1.11 -> 0.95 | 1,833,206 / 1,733,998 | 100 / 133 / 133 |
| printf | 330,649 | 290,521 | 260,936 | 0.88 -> 0.79 | 1,132,367 / 1,148,270 | 78 / 88 / 88 |

Kernel instructions and faults do not move (this story changes only user-space lookup work). Cycles were measured
too (logs below) but the host was running other builds, so they are noisy; user+kernel cycle ratios after: true
1.35, cat 0.99, ls 1.21, sort 1.12, date 1.09, wc 1.02, sed 1.23, grep 1.14, printf 1.16 (before 1.23, 1.05, 1.25,
1.11, 1.03, 1.03, 1.25, 1.09, 1.06): not a verdict either way; instructions are the measure (owner 11-05).

Empty dynamic main (hot_text_products' own count, 21 runs): fast 167,584 (bound 173,500), release 159,904 (bound
162,300): inside the bounds, not more than 10% under, so `DYN_START_INSTR` is unchanged (the empty main makes few
symbolic lookups). Release layout bins (the products step's WINDOW-BIN-BYTES lines): identical before and after.

## Where the remaining 996 per import go (exact: single-step count, trace_start.py and a per-instruction variant)
Whole process imp200 - imp0 = 199,145 steps / 200 = 995.7 (equal to perf's count). Per import:
* `reloc::quick_target` 503 exclusive (the walk, inlined): about 430 distinct instructions each run once or twice
  per import (no loop dominates; 155 are moves: spills of the views' fields and the definition's copies between
  the walk's layers; 6-instruction bounds checks on each table read; the bucket's `div`).
* `Dynamic::hashed_string` 133 (name scan and serial GNU hash, ~15 per byte).
* relocation pass: `pass1` 126 exclusive (entry decode, dispatch, field), `Loads::inside` 28, `Arch::apply` 11.
* about 196 that grow with the probe's ~25 distinct versions, not with imports: `check_versions` (requirement
  checks: `Verdefs::has_hashed` 66, `Vernauxes::next` 26, `Dynamic::once` 6), each version's first name compare in
  the walk (`quick_version_named` 77), the pass's version table (`fill_wants` 18).
So a reference costs about 800 that scale with imports. To reach 300: the walk would need to drop to ~150 (no
copies of the definition, the per-object fields kept in registers, reads the validation already vouches for not
checked again), the hash to ~50 (two bytes a step) and the relocation decode to ~60. Each is a separate change; none
alone gets there.

## Correctness
The quick walk returns the exact walk's answer or hands the reference to it (`Probe::Slow`): a view that failed
validation, SysV-only objects, a name offset or version index the validation does not cover, a canonical PLT entry,
a version index >= 64 or unknown to the pass's table, a unique definition outside the start's scope-order
registry or whose version name reads with an error. Interposition (scope order), copy relocations, protected
symbols and `STB_GNU_UNIQUE` follow from that equivalence; no new reference probe was needed (the existing tests
that settled them against the reference ran in the gate). New tests: `reloc_quick_versions_bind_by_index_each_to_its_own`
(s27_1_c_1), `reloc_quick_unterminated_string_table_is_the_exact_walks_error` (no spec clause; loaded data is
untrusted).

## Jobs (keke, role r5)
On the final code (aea01b407; 0898cf520 and later add only records/ and handoff/):
* `ld-symbol-validate-once-gate`: direct tier (35 targets), 11 suites ok, 993 passed, 0 failed.
* `ld-symbol-validate-once-plants`: version-index-skip CAUGHT, validation-skip CAUGHT, quick-bloom (see hand-back).
* `ld-symbol-validate-once-lint`: `cargo clippy --workspace --all-targets --all-features -D warnings`,
  `cargo xtask static`, `cargo xtask records check`: clean.
* `ld-symbol-validate-once-xtasktest`: `cargo xtask test` (start-up check, §9): products step and layout contracts
  passed; first test set 7,189 run, 7,189 passed (2,920 s); the second set (4,878 product tests) was still running
  at hand-back (PENDING JOBS).
* In-loop check: `cargo xtask precheck` (job `-precheck`, on 0898cf520 because the role was busy with the job
  above; e8d2302ed and later add only handoff/): WAIT, no red (static, placeholders, head, clippy ok; `-after` and
  `-hot1` count, prose-only difference). `ops handback-check` on e8d2302ed: ok.
* `ld-symbol-validate-once-hot1`: `cargo xtask so` + `hot_text_products` (fast) and `NIJI_FULL_PROFILE=1` the same
  (release): 33 passed each.
* `ld-symbol-validate-once-after`: niji-ld tests (6 targets, 71 tests), release products, probe 5 and the nine
  binaries (the after columns above).
Earlier runs (not on the final code): `-before`, `-before2` (the base's measurements: the before columns),
`-ldcheck1` and `-ldcheck2` (rc 101: the version test's fixture did not link, then its library hid `vf@V1`; fixed in
aea01b407), `-ldcheck3` (green, uncommitted tree), `-trace1`, `-trace2` (the single-step profiles).

## Leftovers / next
* next: the three changes above toward 300 per symbol (walk without copies and re-checks, a two-byte hash step, a
  leaner relocation decode), measured by the same probe; or design C (a persistent resolution cache keyed by build
  ids: near zero per symbol, but a new on-disk trust boundary that needs a ruling).
* `check_versions` and the walk each find a requirement's definition by name; sharing that (one table per object
  pair) would remove ~half of the per-version cost.
