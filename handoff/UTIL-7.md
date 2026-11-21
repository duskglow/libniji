# UTIL-7: the `iconv` command (issue #14)

**State.** Branch `task/util-7` from origin/next `575214f4b`; role mei `~/w/s2`. Commits: `7ac2b16f2` (front end, build,
packaging, transcript cases), `441c1fe30` (recorded outcomes, `--verbose` fix, pending record, plants), then this
note and `handoff/reports/UTIL-7.json`. Jobs on mei: `util-7-check1` (clippy, static, so, transcripts, the two new
tests), `util-7-plants1` (`handoff/UTIL-7-plants.txt`, 10 plants), `util-7-gate1` (direct tier). Results are in the
report json and the hand-back.

## On the branch

* `csys/util/iconv.c`: a thin front end over libniji's `iconv(3)` (spec §31.6). It uses getent's option parser
  pattern (permuted options, `--`, unique long prefixes, the ambiguity list, exit 64). Each operand is read whole and
  converted with its own `iconv` calls, and the output is collected and written at exit (also after an error).
  `-c` adds `IGNORE` to the target's suffixes. `-l` lists the built-in names and every charmap stem `iconv_open`
  accepts, sorted, once each, as `NAME//` (`NAME/` when the name holds `/`).
* `csys/util/iconv-names.h`: the built-in names. `crates/core/tests/iconv_cmd_names.rs` holds the header equal to
  `niji::iconv::builtin_names()`, a new 4-line accessor in `crates/core/src/iconv/mod.rs` (no converter change).
* `xtask/src/so.rs`: `run_getent` is now `run_dynamic_util(name)`; it builds `target/niji/iconv` (shipped) and
  `target/niji-test/iconv` (this tree's loader). The transcript harness maps `iconv` to the test link.
* Packaging: `tools/deb/system.sh` ships `/usr/bin/iconv` in libc-bin (stripped like getent), and
  `tools/deb/check-system.sh` allows the path. There is no PKG-MANIFEST yet. The coexist flavour ships no
  utilities, so nothing changes there. NOT VERIFIED: neither package flavour was built in this run.
* `validation/transcripts/iconv/`: 49 cases recorded from the reference on mei. They cover conversion, long and
  attached options, `-`, the locale default, illegal, unconvertible and truncated input, `-c`, `//TRANSLIT`,
  `//IGNORE`, unknown charsets (from, to, both, with a suffix), a missing file, the stop after an error, a read
  error, `-o` (written, kept after an error, empty, naming the input, a directory, `/dev/full`), `--verbose`, `-s`,
  every option error, 5 charmap or EUC-JP cases, and input split across two files after every byte.
* `xtask/tests/transcripts.rs`: `the_committed_iconv_transcripts_hold`, and `every_name_iconv_lists_converts`
  (runs `-l`, then converts with every listed name). Both skip with a message when `cargo xtask so` has not
  built the test link.
* `records/pending/UTIL-7-1.toml` (D-NEW, series D-31).

## Probe-settled (reference, black box)

* Messages: illegal input; incomplete character at the end of the buffer; conversion from, to, or both not
  supported, followed by the Try line; `cannot open input file`; `cannot open output file: <err>` (no name);
  `error while closing output file: <err>`; `error while reading the input: <err>`.
* Exit statuses: 1 for these errors, 0 for `-c`, 64 for an option error.
* The position counts bytes within the current file. With `//IGNORE` it is the end of the input.
* Each file is converted on its own: an incomplete end is an error. A missing file continues to the next operand.
  An illegal sequence or a read error stops.
* `--verbose` names each operand on stderr, and nothing for implicit stdin. `-s` changed nothing visible.
* `-l` on the reference prints 1180 `NAME//` lines.

## Open

* **Charmap aliases are not listed by `-l`** (for example `CP1251`'s `% alias` names). The front end cannot read
  gzip, and only libniji's Rust charmap code can. Options: (a) a libc-internal listing entry point (a new
  interface; ruling); (b) a Rust front end; (c) leave as is (every listed name converts, but not every name that
  converts is listed). The spec's libniji rule says "exactly the names it can convert", so (c) falls short of it.
* Not covered by transcripts until SYS-ICONV-COVERAGE lands: conversions through every charmap (the plan's
  corpus). Only KOI8-R, CP1252, SHIFT_JIS, EUC-KR and EUC-JP are here; `crates/core/tests/charmap_data.rs`
  covers the codecs. A pipe that delivers standard input one byte at a time is also not covered: the harness
  writes stdin whole. The front end reads to EOF before converting, so a split cannot change the result.
* Unprobed messages: an `iconv_open` failure other than `EINVAL` (`failed to start conversion processing: <err>`
  is a guess), and E2BIG growth under `//IGNORE` (`reserve` makes it rare).
* `--help` and `--version` are not compared (identity strings, §31.1).

## Next step

Read the three mei jobs (`~/bin/jobwait util-7-check1 util-7-plants1 util-7-gate1`). Then the RULING on the
aliases. Read: plan/utilities.md §2.3/§4/§5, spec §31.6, docs/transcripts.md, this note.
