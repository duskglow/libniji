# SYS-ICONV-COVERAGE (issue #6): iconv names with libniji's packages alone — after run 2

**State.** Branch `task/sys-iconv-coverage-2`, code head `d3b1241bc` (handoff commits after it).
Decision: `records/pending/SYS-ICONV-COVERAGE-1.toml` (D-NEW, amends D-6.8; updated this run).
Plants: `handoff/SYS-ICONV-COVERAGE-2-plants.txt`. Per-name before/after table:
`handoff/SYS-ICONV-COVERAGE-2-names.tsv` (run 1's classes kept in `SYS-ICONV-COVERAGE-names.tsv`).
Report: `handoff/reports/SYS-ICONV-COVERAGE-2.json`. Jobs and results: see "Jobs" below.

## What run 2 did

* **Run 1's red fixed.** The clippy arithmetic in `iconv_charmaps_products.rs` now uses saturating
  forms. Run 1's plants baseline failed in `xtask so` because `VENDORS/MICSFT/PC/CP737.TXT` (vendored
  unchanged) ends with a DOS end-of-file byte (0x1A). The generator now reads such a file up to that byte.
* **No fixed charmap cap** (owner principle 11-14). `global_locale.rs`: the 4×256 static arrays are
  replaced by a table of 4-word records that doubles by fallible allocation (`charmap_room`), under
  `LOCK`, read through a new `Objects::words_at` (the existing `tb::words64`, no new `unsafe`).
  `ENOMEM` reaches `iconv_open` from every allocation on the path: `Mapper::map` now returns
  `KResult` (an mmap `ENOMEM` is no longer "file missing"), and the same holds for the parse,
  `TryBox`, the name interning and the table. Other errors still mean "not there" (`EINVAL`).
  Loaded charmaps stay usable, and a later call retries.
* **Tag characters.** The whole code-space test (first run this run) showed the reference's encoders
  write U+E0000..U+E007F as nothing for every generated charset (CP1250: 379 vs 251 `E` lines). The
  charmap codec now does the same, as EUC-JP already did (D-6.15). Without this, 43 of 43 charsets differed.
* **New test** `charmaps_load_past_any_fixed_count_and_fail_cleanly_without_memory`
  (`tests/oracle/iconv_many_charmaps.c`). It loads 300 distinct charmaps in one process, then sets
  `RLIMIT_AS` to its size + 256 KiB and opens more until one fails. That failure must be `ENOMEM`;
  afterwards every earlier descriptor still converts, and the failed name opens once the limit is
  lifted. Measured: `ok first=300 more_opened=64 errno=ENOMEM`.

## Before / after (every reference `iconv -l` name; libniji seeing only the generated charmaps)

| class | names | before (alone) | after (alone, converts as the reference) |
| --- | ---: | ---: | ---: |
| charmap-not-shipped | 382 | 0 | 0 |
| charmap-generated | 281 | 0 | 280 |
| single-byte-no-charmap-name | 229 | 0 | 0 |
| multi-byte-no-charmap-name | 121 | 0 | 0 |
| charmap-not-shipped-differs | 59 | 0 | 0 |
| opens-builtin | 58 | 58 (open) | 52 |
| stateful-or-no-single-byte | 50 | 0 | 0 |
| total | 1180 | 58 | 332 (338 open) |

One process and one process per name give identical results: the cap is gone. "After" is stricter
than "before": the probe line must equal the reference's (open both ways plus a fingerprint).
The 6 built-ins that open but differ are UTF-8, UTF8, ISO-IR-193, UTF-16, UTF16 and CSUNICODE.
Their fingerprints differ in the built-in Unicode codecs, which this story did not touch, so they
are not counted here. ISO88591 is the one generated-class name not in `charmaps.tsv`.

## Stateful encodings (ruling #6225: separate story; 50 names, class `stateful-or-no-single-byte`)

ISO-2022 family (JP/-1/-2/-3, KR, CN, CN-EXT and aliases), UTF-7 (and its aliases), IBM930/933/935/937/939
and the other names in that class. The exact list is in the names table, class `stateful-or-no-single-byte`.

## EBCDIC pages (ruling #6226: IBM CDRA data allowed; prefer IBM's own files)

Sources checked (this run). IBM's own registry file
(`public.dhe.ibm.com/software/globalization/gcoc/attachments/CP00037.txt`) is reachable, but it maps
bytes to IBM GCGIDs, not to Unicode, and lists graphic characters only (no control range). It needs
IBM's GCGID-to-Unicode table as well. ICU's CDRA tables in Unicode form are at
`github.com/unicode-org/icu`, `icu4c/source/data/mappings/ibm-<n>_P100-<year>.ucm` (format `<Uxxxx> \xNN |k`).
**Do not use** the `icu-data` repository's `glibc-*.ucm` files: they are tables taken from glibc.
Next: a `ucm` reader in `xtask/src/charmaps.rs` (precision `|0` round trip, `|1` fallback, `|3`
decode-only), vendored `.ucm` files plus the ICU licence and provenance under
`spec/encoding/ibm/`, rows only where the whole code-space probe equals the reference.

## Jobs (keke, role n2, code head d3b1241bc)

* `sys-iconv-coverage-2-gate3` (`~/work/SYS-ICONV-COVERAGE-2/gate.sh <commit> <epoch>`). Passed
  before hand-back: clippy, static, so, `iconv_charmaps_products` (2 tests: 43 charmaps / 280
  names over the whole code space; the 300-charmap + ENOMEM test), `iconv_codespace_products`,
  `iconv_products`; the after column (above) and round trips (ISO-8859-15, CP1252, ISO-8859-2,
  KOI8-R, CP1250, IBM850: libniji's bytes and round trips equal the reference's); both package
  flavours; check-system. Running at hand-back: `affected --run --base origin/next`. It runs
  everything because `spec/encoding/charmaps.tsv` is a path the selection does not know. After it,
  the FULL_PROFILE `hot_text_products` start-up check.
* `sys-iconv-coverage-2-xtest`: `cargo xtask test` (start-up/layout check: the 8 KiB of static
  charmap arrays are gone), queued to start when gate3 ends.
* `sys-iconv-coverage-2-plants`: rc 0, 7 plants, 4 CAUGHT, 0 SURVIVED, 3 WARN-NO-MSG. All three
  failed in the intended test: `charmap-table-no-growth` and `charmap-table-copy-lost` as
  `charmaps_load_past_any_fixed_count... FAILED`, `charmap-fill-short` as
  `generated_charmaps_convert_like_the_reference ... FAILED` at the names assert. Their msg regex
  used `\|`, which the tool reads as a literal pipe.
* Superseded: `-clippy1` (untyped counter), `-dev1` (the code-space test found the tag
  characters), `-gate` and `-gate2` (package build: the role is not a git tree, and `~/bin/gate`
  drops environment variables, so the commit is now a script argument). `-cs1` is a probe (evidence).

## Next step

1. EBCDIC pages from the CDRA `.ucm` tables (above): the largest part of `charmap-not-shipped`.
2. `single-byte-no-charmap-name` (229): alias groups by probe fingerprint onto existing rows plus data.
3. Multi-byte (WHATWG indexes, D-6.15 pattern). The stateful encodings are their own story (#6225).

Plan rows: D-6.8, D-6.9, D-6.15, the pending D-NEW.
