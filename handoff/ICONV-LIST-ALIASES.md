# ICONV-LIST-ALIASES (runs ICONV-LIST-ALIASES, ICONV-LIST-ALIASES-2): `iconv -l` lists charmap header names

State: branch `task/iconv-list-aliases-2` (continues `task/iconv-list-aliases`, whose run 1 ended on an API 529
before writing a handoff). Base origin/next 0495c4684. Code head ea1b5ee87 (later commits: handoff/ only).

Gate on ea1b5ee87 (shiki `iconv-list-aliases-2-gate2`, direct tier): 15 suites ok, 1837 passed, 0 failed, peak 3.0G.
Run 2 SUPERSEDED `iconv-list-aliases-2-gate1` (on 53ced61: `every_name_iconv_lists_converts` failed on ARABIC7, no `A`;
fixed by ea1b5ee87).

## What the branch does

- `csys/util/charmap-header.h` (new): UTIL-8-2's in-binary gzip inflater and charmap header reader, moved
  unchanged out of `csys/util/locale.c` (rulings #6246/#6261: no new libc.so.6 export) and given an input cap and
  an output cap. `locale.c` and `iconv.c` both include it: one copy of the mechanism. `locale -m` reads as before
  (whole file, 64 KiB of text).
- `csys/util/iconv.c` `-l`: for each installed charmap iconv_open accepts, reads the header the way iconv_open's
  header scan reads it (`crates/legacy/src/global_locale.rs` `find_charmap`: first 4096 bytes of the file, at most
  8 KiB of gzip text; 2 KiB first, on to 8 KiB only when the `CHARMAP` line is not in the first 2 KiB) and lists
  `<code_set_name>` and each `alias` comment line, spelled as iconv_open matches names (letters, digits,
  `,-./:_`, upper case), only when iconv_open accepts it. Same order and `NAME//` format as before.
- Tests: `crates/core/tests/iconv_listing_products.rs` `iconv_listing_holds_the_names_charmap_headers_give`
  (four private charmaps mounted over the charmap directory in a user+mount namespace: plain with
  `<comment_char> %`, gzip with a lower-case alias, gzip with the names after 5 KiB of text, plain with one alias
  past the 4096-byte scan; every name listed and round-trips `Aé`; every listed name round-trips `A`; the far
  alias is listed exactly when a fresh process converts it). `xtask/tests/transcripts.rs`
  `every_name_iconv_lists_converts`: every name the products list round-trips one character in fresh processes,
  `A`, or for a set without `A` the first character the set decodes from its bytes 0x21-0x7e/0xa1-0xfe with `-c`
  (run 2 fix: ARABIC7, KATAKANA, ISO_5427, INIS, NATS and JIS OCR add-on sets have no `A`; the gate failed there).
- Plants: `handoff/ICONV-LIST-ALIASES-plants.txt`, 8 plants, 8 CAUGHT (shiki `iconv-list-aliases-2-plants1`).

## Measured (Haswell build host, 233 gzip charmaps installed; not a timing host)

| | origin/next 0495c46 | branch |
|---|---|---|
| `iconv -l` names | 982 | 1028 (+46, none removed) |
| `iconv -l` instructions:u (perf stat -r 5) | 1,052,960,050 | 1,302,453,716 (+23.7%) |
| `iconv -l` page faults | 3,817 | 4,209 |
| counter run time (loaded host, indicative only) | 0.211 s | 0.268 s |
| `iconv` binary bytes / text | 78,904 / 53,563 | 83,856 / 59,052 (+4,952 / +5,489) |

Where the +250 M instructions go (perf record -c 100000 on instructions:u; sample totals 3538 -> 4397, the same +24% ratio as perf stat): about 230 samples in the
binary's own header inflater (`decode`, `block`, `bits`; it decodes bit by bit), about
485 in libc's `charmap::inflate::block`: ONE extra libc header scan of the whole directory (strace: directory
opens 3 -> 4, opens per charmap 2 -> 4). The scan is triggered by `iconv_open("BIG5-CP950")` (BIG5's header
alias) right after BIG5.gz is loaded: libc's lookup does not match a loaded charmap's alias comment lines, so it
scans; the scan is cached for every later name. A fresh `iconv -f BIG5-CP950` pays the same scan.

Regression proof: the new products test run against origin/next's products fails (`LISTP-SET is not listed by
iconv -l`; shiki `iconv-list-aliases-2-next`).

## Open / next

- Fix-forward options for the cost (not done; `iconv -l` is a rare interactive listing): (1) libc `iconv_open`
  matching a loaded charmap's header alias lines before scanning (also speeds `iconv -f <alias>` in a process that
  already loaded the charmap); (2) a table-driven Huffman decode in `charmap-header.h`.
- Nothing else open. Do not redo: the inflater move, the 2 KiB/8 KiB scan split, the test fix for sets without `A`.

Plan rows: none new (the scan bounds follow `find_charmap`; rulings #6246/#6261).
