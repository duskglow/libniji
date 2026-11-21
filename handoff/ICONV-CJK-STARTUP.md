# ICONV-CJK-STARTUP: take back the start-up cost the CJK tables added (hand-back, leg 2)

**State.** Branch `task/iconv-cjk-startup` (from origin/next 83f91cb05), role keke `~/w/n2` (Zen 1 host).
Commits: f4b9f1482 (code), 9b050869f (record D-6.19, `records/decisions/D-6/D-6.19.toml`), 6fce3b3e6 (test scope
fix, leg 2), then handoff/report commits. Scripts on keke `~/work/iconv-cjk-startup/` (su.sh start-up counts,
relsites.py relocation attribution, chk.sh, ic.c a BIG5 round-trip program); 21.5 / 21.6 products as p215/, p216/.

## What the branch does

* Generator `xtask/src/encodings_cjk.rs`: all sets' tables concatenated per kind (`ROWS`, `CELLS`, `SKIP1`,
  `SKIP_ROW`, `PAGES`, `CODES`, `FOUR`, `FOUR_ENC`) and `SETS: [Set; 5]` of (start, count) spans; `cjk.rs` reads
  them through `part()`; `Which::set()` indexes `SETS` (no match on addresses).
* `iconv/mod.rs`: `NAMES` / `LATIN1_MORE` are build-time only; const fns pack `NAME_BYTES` and `NAME_ROWS`
  (offset, length, codec); `builtin()` / `builtin_names()` read those.
* Test `crates/core/tests/iconv_reloc_products.rs`: no RELATIVE word inside the cjk_tables objects, `NAME_ROWS`,
  `NAME_BYTES`, and none outside `.got` pointing into them. Plants `handoff/ICONV-CJK-STARTUP-plants.txt` (3).

## Attribution (relsites.py, 21.5 -> 21.6)

All 71 new RELATIVE are ICONV-CJK's: 40 descriptor slices (8 x 5), 10 in compiler lookup tables of the five
descriptors' addresses (`Which::set`'s match, two copies), 21 in the `NAMES` rows (`&[u8]` each).

## Leg 2: the plants baseline red

The plants tool builds the dev profile ungrouped. There `builtin` reads `NAME_BYTES` with
`add NAME_BYTES@GOTPCREL(%rip),%r10`; ld cannot rewrite a GOTPCRELX binop in a shared object, so one `.got` slot
(0x48dd28) holds the table's address. Same build: 21 other GOT RELATIVE slots to statics (`locale::classes::C_TABLE`,
`iconv::aliases::TEXT`, `niji_boundary::heap::HEAP`, ...), so a non-LTO codegen property, one word per table, not
per row; the release profile (fat LTO) has none for these tables. The test now leaves `.got` words out of
"points into" (commit 6fce3b3e6, reason in its comment). plants-2 baseline: ok.

## Results (keke, Zen 1, full profile, median of 101, ASLR off; user instructions / faults)

| program | 21.5 | 21.6 | head |
|---|---|---|---|
| empty main | 740,804 / 51 | 742,212 / 56 | 739,510 / 55 |
| 3 libraries | 983,610 / 65 | 985,019 / 71 | 982,318 / 70 |
| clock_gettime | 762,925 / 55 | 764,333 / 60 | 761,632 / 59 |
| BIG5 round trip (ic.c) | 46,997,067 / 415 (charmap) | 762,976 / 72 | 760,565 / 72 |

RELATIVE 1,502 / 1,573 / 1,438 (head; relocated pages 12, was 13). Instructions below 21.5 on all three rows.
**Faults: target not reached** (+4/+5 over 21.5 remain, -1 vs 21.6); not attributed (21.6 also landed other
layout changes, ld.so text +14,976 B). Skylake-SP class not measured. **getconf not fixed:** text 903,524 (21.5) /
1,340,252 (21.6) / 1,340,708 (head); likely cause, NOT VERIFIED (no link map read): printf's wide conversions reach
the locale codec dispatch (`Codec::decode/encode` match `Codec::Cjk`), so `--gc-sections` keeps cjk.rs and its tables.

## Jobs (keke)

On the final code (a5b0f172f): chk3 (clippy -D warnings, static, iconv_reloc_products on the full-profile
products) rc 0; crates-2 (static, `cargo test -p niji`, `cargo test -p xtask`); plants-3 (the 3 plants).
SUPERSEDED: chk1 (f4b9f1482: full-profile so, hot_text_products 33 ok, iconv lib 17 ok; no tested-content record),
crates (rc 0 on f4b9f1482, before the test edit), plants (baseline red, fixed by 6fce3b3e6), chk2 (stopped:
mis-chained), plants-2 (desc-ptr, span-cut CAUGHT; name-off SURVIVED under a wrong cmd, fixed by a5b0f172f).
`affected` escalates to FULL (xtask changed): FULL is the train's; `cargo xtask test` (full gate) not run for that
reason. plants-3 and crates-2 waited ~30 min in keke's slot queue (all 24 cores busy).

## Next step (follow-up stories)

1. Faults: a per-page fault trace of an empty main (`perf record -e page-faults -d`), 21.5 vs head, both classes.
2. getconf: confirm with a link map (`-Wl,-Map`) why the static getconf keeps cjk.rs; make CJK dispatch reachable
   only from iconv_open / setlocale (e.g. a function pointer installed there), measure the static size.
