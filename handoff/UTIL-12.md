# UTIL-12: `niji-locale` and `localedef` (issue #14's blocker)

**State.** Branch `task/util-12` (from origin/next `90390071c`), role mei `~/w/s1`. Head: see `git log -1`.
Leg 3 (refresh 2) done: **LC_COLLATE written** (`crates/locale/src/collate.rs`, commit 057aadf1e + the
handoff commit). `compile()` now writes all 12 categories. `crates/locale/tests/reference.rs`: **all 6 green
on mei** (job util-12-coll4: en_US/de_DE/fr_FR/ja_JP/tr_TR UTF-8, en_US ISO-8859-1): every simple category
byte-equal, LC_CTYPE as before, LC_COLLATE by ORDER: strcoll (multibyte), wcscoll and collseq (bracket
ranges) over a deterministic 4341-string corpus sorted by the reference's file, every adjacent pair same
order under ours (both total preorders, so all pairs agree). Key bytes differ by design (D-31.20).
Workspace clippy, `cargo xtask static`, gate and plants NOT run yet. `cargo test -p niji-locale` ~12 s warm
on mei; runner: `bash ~/w/.scratch/UTIL-12/t.sh <label>` (sync + jobrun + summary; scratch, not in tree).

## Reserved decision ids (ops reserve, leg 3): D-31.19, D-31.20, D-31.21 (rows NOT written yet)

* D-31.19: niji-locale is its own crate (`crates/locale`), not beside the reader in niji-core: changes
  reach few tests; the reader `niji::locale` is the test oracle.
* D-31.20: LC_COLLATE weights are libniji's numbering (place in the order; wide units = place + 2;
  multibyte = per-level rank as a prefix-free code, 1 byte when the level has <= 254 values, else 1/2-byte
  codes with lead bytes >= 2 and trail bytes >= 1, 3 bytes over 64770). Order equal to the reference's,
  bytes not. Plain EXTRAMB/EXTRAWC lists (no range entries, INDIRECT empty); symbol hash items 13-15 empty
  (size 1, 8 zero bytes) since niji's regex reads no names from them.
* D-31.21: transliteration stores each distinct alternative list once (from leg 2: reader-equal, ~half
  the reference's to-table).
Row format: `records/decisions/D-31/D-31.<n>.toml` (read one existing row; developer-flow-reference §10).

## Probe-settled for LC_COLLATE (leg 3; reference keys via python wcsxfrm/strxfrm on mei)

* Weights = places in one order of symbols, characters and elements (the leading symbol list in
  iso14651_t1_common stands outside any order_start). A line for an item already placed moves it
  (tr_TR relists `<CAP>` `<MIN>` after `<RES-1>`); a moved line keeps its section's rules (tr_TR moves
  letters after symbols that have no section; with ruleset 0 = SPECIAL's backward level 2 the order broke).
* Undefined characters share ONE element placed before every character (after the leading symbols),
  wherever UNDEFINED stands (ja_JP's UNDEFINED is the last line; its undefined chars weigh 2, below
  space). Without UNDEFINED: empty levels but the last, which has its own place (en_US: key 1 1 1 1 3b8d).
  collseq: undefined chars have none (absent block or u32::MAX).
* Multibyte: every byte of a sequence that is not an ordered character is its own undefined element
  (reference: an undefined 4-byte char gives 4 elements, `01 03` x4 at level 4).
* `..` alone on a line orders the code points between its neighbours (iso14651_t1's Han range
  U+4E00..U+9FA5, consecutive weights); `..` as a weight = the character itself. `ifdef`/`else`/`endif`
  /`define` handled (DIACRIT_BACKWARD).
* Guesses still open: `...` treated by code point (charmap order for non-UTF-8 not checked); charmap
  chars only in the tables for single-byte charmaps; invalid bytes weigh as undefined;
  `reorder-sections-after` refused with an error (none of the five uses it).

## Settled earlier (legs 1-2; see git history of this file for the full lists)

Lexer comments to end of line; MB_CUR_MAX from the charmap header; byte tables −128..255 rules;
CLASS32/TOUPPER32 by code point; class bitmaps appended to the previous item; translit last-wins with the
charmap filter; NONASCII_CASE/MAP_TO_NONASCII; ERA BC years; magics and item counts of all 12 files.

## Exact next step (leg 4; the last refresh is used: hand back from leg 4)

1. Write the three D rows (D-31.19..21) and commit with the collate test comment fix.
2. The `localedef` program. Decide the cheapest route that ships in libc-bin:
   (a) freestanding static Rust like ldconfig (`crates/ldconfig-bin` is a 16-line staticlib over
   `niji-boundary` feature `ldconfig`; entry `niji_machine_ldconfig_entry` in csys/src/machine/entry.S;
   built by `run_ldconfig` in xtask/src/so.rs ~2491); a `localedef` feature would make boundary depend
   on niji-locale (check the dependency direction first), or
   (b) a C front end `csys/util/localedef.c` (argv, files, gzip via a pipe is NOT allowed: rulings #6246
   say inflate in-process) calling a Rust staticlib entry `niji_localedef_compile` (needs a global
   allocator over malloc in a no_std staticlib), built like `run_dynamic_util` (so.rs ~2549).
   Minimum options for issue #14: `-i`, `-f`, `-c`, NAME with `/` = directory form (12 files, LC_MESSAGES/
   SYS_LC_MESSAGES), `--no-archive`; exit 4 + `[error] ...` on missing source/charmap; --help paths.
3. Archive writer (D.1.6; `niji::locale::archive` reads) for NAME without `/`; or defer to a follow-up
   story with a RULING/INFORMATIONAL line if time is short (bash's suite needs only the directory form).
4. Manifest row `system libc-bin usr/bin/localedef`; transcript vs reference (UTIL-1 harness);
   plants in `docs/plants.md` form on collate.rs/ctype.rs/cats.rs (e.g. drop the reorder ruleset keep,
   the undefined placement, the Codes lead offset) run via `cargo xtask plants`; gate
   (`cargo xtask affected --run`); precheck; handback-check.
5. RULING NEEDED to raise: shipping Devuan's locale sources/charmaps in `locales` (R5: "Devuan's data
   compiled by libniji's localedef"; they come from glibc's source package; not committed to this tree).

## Budget notes

Leg 1: 80 calls, ~190k. Leg 2: ~115 calls, ~150k. Leg 3: ~78 calls, ~175k (format and design reading
took most; the writer passed after four test rounds). Wall deadline 2026-11-22T08:02:37Z: leave >= 45 min
for gate + plants + precheck; start plants/gate by ~07:00 at the latest.
