# ICONV-CJK-STARTUP: take back the start-up cost the CJK tables added (leg 1 of the run)

**State.** Branch `task/iconv-cjk-startup` (from origin/next 83f91cb05), role keke `~/w/n2` (Zen 1 host).
Code commit f4b9f1482, record commit 9b050869f (D-6.19, reserved). Scripts on keke `~/work/iconv-cjk-startup/`
(su.sh start-up counts, relsites.py relocation attribution, chk.sh, ic.c a BIG5 round-trip program); 21.5 and 21.6
products extracted there as p215/, p216/; this head's full-profile products as hd02/.

## What the branch does

* Generator `xtask/src/encodings_cjk.rs`: all sets' tables concatenated per kind (`ROWS`, `CELLS`, `SKIP1`,
  `SKIP_ROW`, `PAGES`, `CODES`, `FOUR`, `FOUR_ENC`) and `SETS: [Set; 5]` of (start, count) spans; `cjk.rs` reads
  them through `part()`; `Which::set()` indexes `SETS` (no match on addresses).
* `iconv/mod.rs`: `NAMES` / `LATIN1_MORE` are build-time only; const fns pack `NAME_BYTES` and `NAME_ROWS`
  (offset, length, codec); `builtin()` / `builtin_names()` read those.
* New test `crates/core/tests/iconv_reloc_products.rs`: no RELATIVE word inside, or pointing into, the cjk_tables
  objects, `NAME_ROWS`, `NAME_BYTES`. Plants `handoff/ICONV-CJK-STARTUP-plants.txt` (3).

## Attribution (step 1, relsites.py, 21.5 -> 21.6)

All 71 new RELATIVE are ICONV-CJK's: 40 descriptor slices (8 x 5), 10 in compiler lookup tables of the five
descriptors' addresses (`Which::set`'s match, two copies), 21 in the new `NAMES` rows (`&[u8]` each; 18 to
.rodata strings, 3 to strings merged into the start-up read-only section).

## Results (keke, Zen 1, chk1 job on f4b9f1482, full profile, median of 101, ASLR off)

| program | 21.5 | 21.6 | head |
|---|---|---|---|
| empty main | 740,804 / 51 | 742,212 / 56 | 739,510 / 55 |
| 3 libraries | 983,610 / 65 | 985,019 / 71 | 982,318 / 70 |
| clock_gettime | 762,925 / 55 | 764,333 / 60 | 761,632 / 59 |
| BIG5 round trip (ic.c) | 46,997,067 / 415 (charmap) | 762,976 / 72 | 760,565 / 72 |

RELATIVE 1,502 (21.5) / 1,573 (21.6) / 1,438 (head; 12 relocated pages, was 13). Instructions: below 21.5 on
all three. **Faults: target not reached** (+4/+5 over 21.5 remain; only -1 vs 21.6); not attributed: no per-page
fault trace yet, and 21.6 also landed VALGRIND-DBG-RODATA etc. (ld.so text +14,976 B), so they may not be
ICONV-CJK's. getconf: text 903,524 (21.5) / 1,340,252 (21.6) / 1,340,708 (head): **not fixed**; likely cause
(by reading, NOT VERIFIED by a link map): printf's wide conversions reach the locale codec dispatch
(`Codec::decode/encode` in iconv/mod.rs match `Codec::Cjk`), so `--gc-sections` keeps cjk.rs and its tables.

Checks on f4b9f1482 (chk1, rc 0): clippy, static, `-p niji --lib iconv` 17 ok, full-profile so,
hot_text_products 33 ok, iconv_reloc_products ok. `affected` escalates to FULL (xtask changed): gate = precheck +
`iconv-cjk-startup-crates` (static, `cargo test -p niji`, `cargo test -p xtask`; started on f4b9f1482 because the
sync was BUSY; code identical to 9b050869f).

## OPEN: plants baseline failed (keke:iconv-cjk-startup-plants rc 2)

The plants tool's baseline builds **ungrouped** (`layout ungrouped`); there iconv_reloc_products fails:
`relocated word at 0x48dd28 points into ..._4niji5iconv10NAME_BYTES` (log
`/dev/shm/claude/iconv-cjk-startup/plants-1/logs/baseline-1.log`, products in `.../plants-1/src/target/niji`).
The traced full-profile build passed. Not yet known which word that is.

## Exact next step

1. `python3 ~/work/iconv-cjk-startup/relsites.py <21.6 libc> /dev/shm/claude/iconv-cjk-startup/plants-1/src/target/niji/libc.so.6`
   or `readelf -sW` around 0x48dd28 in that libc: name the site holding the word (a slice of a name somewhere,
   or a false hit of the test's range check). Fix the cause (or the test if it is a false hit, with evidence).
2. `~/bin/jobwait iconv-cjk-startup-crates`; rerun plants (new dir plants-2); sync; precheck
   `--jobs iconv-cjk-startup-chk1,iconv-cjk-startup-crates,iconv-cjk-startup-plants-2`; report json; handback-check.
3. Faults/getconf: hand back COMPLETE "target not reached" for faults with next: a per-page fault trace
   (perf record -e page-faults -d, 21.5 vs head) and a getconf follow-up (CJK dispatch reachable only from
   iconv_open/setlocale), unless budget allows.
