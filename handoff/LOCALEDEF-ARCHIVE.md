# LOCALEDEF-ARCHIVE: locale archive writer, the #7534 gaps, a freeing arena

**State.** Branch `task/localedef-archive` (from origin/next), role keke `~/w/n2`. Leg 3 ended in a REFRESH (last
one of the run) at ~145k context / 110 calls, with the plants and gate jobs started (below). Records D-31.23..26 are
written (`records/decisions/D-31/`); no COMPAT (two RULING NEEDED, below). Probe scripts and logs:
`~/work/localedef-archive/` on keke (`e2e.sh`, `rss.sh`). Sync: `bash ~/src/liella-ops/tools/sync_role.sh --story
LOCALEDEF-ARCHIVE --base origin/next ~/w/LOCALEDEF-ARCHIVE keke n2` (rc 0 = synced).

## Done in leg 3

* Commit `e8c2bcbca`: prog tests (13, all pass: archive default/lock, names + charmap codeset alias, list, delete,
  add/--replace/missing, stdin, outside-section line, category error with/without -c); category error message now
  `<file>:<line>: <CAT>: syntax error: <what>` (`Output::failed`); core reader: locrec 0 = absent
  (`crates/core/src/locale/archive.rs`, unit test `lookup_skips_a_deleted_name`, 4 pass); reference.rs temp-dir
  race fixed (en_us_utf8 and generated_charmaps both used pid+en_US.UTF-8 and removed each other's dir: a FLAKE on
  next too, INFORMATIONAL); spec §31.8 i_3/g_2/g_3/g_4/f_2 + stubs regenerated; probe transcripts in
  `spec/behaviour/utilities.glibc.txt`; D-31.24/25/26; three differences rows. `cargo xtask records check` ok.
* Next commit (leg-3 end): D-31.23 with RSS; `products_localedef_writes_the_library_compile` in reference.rs (runs
  `target/niji/localedef`: en_US/de_DE dir files == library compile, archive names); unsafe-audit fix in
  ldconfig.rs (UNSAFE line directly above the allow); `handoff/LOCALEDEF-ARCHIVE.plants` (8 plants).
* E2E (job `localedef-archive-e2e`, keke, rc 0): libniji localedef wrote en_US de_DE fr_FR ja_JP tr_TR + a
  six-POSIX-category source (`copy "en_US"` x6, the RELEASE-SNAPSHOT-1122C L1 case) into an archive (`--prefix`) and
  as directories: six -> 12 files, rc 1 (warnings). The reference `localedef --list-archive` lists our archive
  (same 6 names). Under `unshare -r -m` with the archive bind-mounted on /usr/lib/locale, libniji `locale -a` lists
  the 6 and `LC_ALL=<loc>.UTF-8 locale -k <12 categories>` gives 180 lines, no stderr, byte-identical to the
  directory form via LOCPATH, for all 6 (setlocale(LC_ALL) non-NULL for the six-category case).
* RSS after (same script, KiB): en_US 109552, de_DE 111576, fr_FR 111560, ja_JP 49936, tr_TR 112540 (before 224640,
  344160, 344160, 234240, 283680): -51 % to -79 %.

## Done in leg 2

* Commit `125135975` (tested): defaults. `undefined_categories_get_the_references_defaults` in
  `crates/locale/tests/reference.rs` PASSES for UTF-8 and ISO-8859-1 (source with only LC_MEASUREMENT, reference
  `localedef -i <path> -c`). Finding: the reference's undefined-category output is NOT the POSIX locale in places:
  LC_CTYPE without print/graph/cntrl/punct for ASCII (widths -1, no outdigits), LC_TIME wide strings and alt month
  names empty, LC_MONETARY "not available" numbers 0xff (a source's `-1` gives 0x7f) and mon_grouping "\x7f",
  LC_ADDRESS country_ab2/ab3 spaces. Decision taken (D-31.24, to write): libniji writes the POSIX values there;
  the test skips exactly those items and checks ASCII classes/widths against XBD 7.3.1 (`posix_ctype`). Matched:
  `week 7;19971130;7`, NUMERIC grouping empty, collate code-point form. Needs a differences row + RULING NEEDED
  (recommend: no compat flag, a degenerate default has no use). Arena clippy fixes done.
* Commit `c6d6bbbe3` (WIP, clippy clean, NOT tested beyond unit tests): `crates/locale/src/archive.rs` (Archive
  parse/names/contains/remove/add/write; 3 unit tests pass), prog.rs body rewritten (archive default, list/add/
  delete/--replace, `--no-archive` name normalized, stdin via `Host::read_stdin`, `<stdin>` in messages, -c and
  outside-section statuses via `compile_text` + `Output::outside`), `Host::lock` (flock on the archive dir in
  `crates/boundary/src/localedef.rs`), prog test mock gained `stdin`/`locked`.

## Done in leg 1 (now compiled, in the two commits above)

* Freeing tool arena in `crates/boundary/src/ldconfig.rs` (D-31.23): power-of-two classes 16 B..64 KiB with
  intrusive free lists from 1 MiB chunks; bigger blocks are own mappings (munmap/mremap). RSS not yet measured.
* `source.rs`: lines outside a category recorded in `Source::errors` and skipped. `lib.rs`: `Output`,
  `compile_output`, `compile_text`; `compile()` stays strict. `defaults.rs`: the built-in defaults source.

## Measured before (base = origin/next 8e1837914, job localedef-archive-so-base, keke)

Peak RSS of `target/niji/localedef -i <loc> -f UTF-8 <dir>`: en_US 224640 KiB, de_DE 344160, fr_FR 344160,
ja_JP 234240, tr_TR 283680 (wall 1.0-1.45 s). Script: `sh ~/work/localedef-archive/rss.sh <localedef> <outdir>`.

## Probe-settled (reference localedef, keke; transcript material for tests)

Archive (`--prefix=P` uses `P/usr/lib/locale/locale-archive`):
* Missing `P/usr/lib/locale`: `cannot create temporary file: P/usr/lib/locale/locale-archive.XXXXXX: No such file or
  directory`, status 1. Compile into archive: silent, status 0 (1 with warnings), replaces an existing name silently.
* Stored names: NAME's codeset normalized (`de_DE.UTF-8` -> `de_DE.utf8`, `xx_XX.ISO-8859-1` -> `xx_XX.iso88591`
  whatever the charmap). NAME without codeset: stored as given AND with the charmap's normalized codeset inserted
  before `@` (`en_US` + ISO-8859-1 -> `en_US`, `en_US.iso88591`; `zz_ZZ@euro` + UTF-8 -> `zz_ZZ@euro`,
  `zz_ZZ.utf8@euro`), both sharing one record. `--no-archive` with `nn_NN.UTF-8` writes directory `nn_NN.utf8`
  (normalized too: libniji today uses NAME as given; fix). Use `niji::locale::name::{parse, archive_form}`.
* `--list-archive`: names sorted bytewise, one per line, status 0; with FILE operand reads that file (extra operands
  ignored); missing FILE: `cannot open locale archive "F": No such file or directory`, status 1; missing default
  archive (prefix): nothing, status 0.
* `--delete-from-archive N...`: exact names only (no normalization); a missing one: stderr `locale "N" not in
  archive`, status still 0. No operands: status 0.
* `--add-to-archive DIR...`: stdout `Adding DIR` per operand; name = DIR's last component normalized; existing name
  without `--replace`: stderr `locale 'fr_FR.utf8' already exists`, status 255; missing DIR: stderr `stat of "DIR"
  failed: No such file or directory: ignored`, status 0. No operands: status 0.
* Format (D.1.6): header 14 u32 (magic, serial 0, namehash off/used/size, string off/used/size, locrectab off/used/size,
  sumhash off/used/size); reference sizes 907 / 0x1d4c / 420 / 2003 (preallocated, file 3-6.5 MB). Name slot index =
  hash % namehash_size; hash = `h = len(name); for each byte c: h = rotl32(h, 9) + c` (black-box fit, 12 of 12
  observed names; textbook rotate-add family; `/tmp`-free script hashfit.py in scratch). Collision step NOT observed:
  writer should pick a prime table size >= 2n+1 where every name's hash % size is distinct (grow until so), so any
  reader finds each name at its first probe. A deleted name keeps its slot with locrec_offset 0: libniji's reader
  (`crates/core/src/locale/archive.rs` `entry`) must treat locrec 0 as absent (today it builds a Record at 0; fix +
  unit test). Identical category images are stored once (locrec slots share offsets).
Gaps:
* No `-i` (and `-i -`): source from stdin; messages name it `<stdin>` (`<stdin>:4: syntax error: ...`).
* Lines outside a category: `<file>:<line>: syntax error: not inside a locale definition section`, skipped, output
  written WITH OR WITHOUT -c, status 1. Error inside a category (`decimal_point "." junk junk`): `<file>:2:
  LC_NUMERIC: syntax error`; without -c status 4 no output; with -c output written, status 1. libniji plan: a failed
  category gets its default (under -c), status 1; without -c the "no output file produced" line, status 4.
* Undefined category: `[warning] No definition for <CAT> category found`, status 1, file written with defaults.
  Reference default values (`locale -k` of p1/stdin-out): tel_int_fmt="+%c %a%t%l", country_ab2="  " (2 spaces),
  country_ab3="   ", category=";UTF-8;;;;;;;;;;;" (identification), height 297 width 210, measurement 1, all
  monetary numbers -1, crncystr "-", mon_decimal_point ".". File sizes: ADDRESS 131, COLLATE 92, IDENTIFICATION 104,
  MEASUREMENT 23, MONETARY 270, NAME 62, NUMERIC 50, PAPER 34, TELEPHONE 48, TIME 1928, SYS_LC_MESSAGES 48.
* `--big-endian`: the reference writes big-endian files, status 0. Plan: a differences row (libniji is x86-64 only;
  no libniji reads them) + COMPAT-NEW or the stated reason; RULING NEEDED in the hand-back.

## Exact next steps (leg 3)

Code, tests, records, E2E and RSS are DONE (leg 3); do not redo them. Head `3cc67e6ff` + this handoff commit.
Jobs started on keke at 00:15Z on head 3cc67e6ff (role n2): `localedef-archive-plants` (8 plants, dir
`/dev/shm/claude/localedef-archive/plants-1`; the arena plant runs `cargo xtask so`, ~10 min) and
`localedef-archive-gate` (direct tier). Clippy (workspace, all targets/features, -D warnings) and `cargo xtask
static` passed on the role at that code (static after the last code edit; job localedef-archive-lint2).

1. `~/bin/jobwait localedef-archive-plants localedef-archive-gate` (one call, timeout 600000; exit 3 = call again).
   Check the gate log head for `FULL (escalated` (then brief step 4's rule). Plants: every one CAUGHT; list any
   WARN line. A red: fix, rerun only the failing suite.
2. Start-up checks (core `locale/archive.rs` `entry` changed: one `filter(r != 0)`), AFTER the gate (one build per
   role): job `localedef-archive-test` = `cargo xtask test`; job `localedef-archive-hot` =
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`. Diff
   `target/product/fast/layout-bins.tsv` against origin/next's if cheap (state it if not done).
3. `cargo xtask precheck --jobs localedef-archive-gate,localedef-archive-plants,localedef-archive-test,localedef-archive-hot --head <sha>`
   on the synced final head; report `handoff/reports/LOCALEDEF-ARCHIVE.json` (+ `"jobs"`), handoff rewrite,
   push, `ops handback-check`. Wall deadline 2026-11-23T02:58Z: if ~45 min remain with only jobs left, hand back
   PENDING JOBS.

Hand-back material:
* RULING NEEDED: (a) D-31.24 undefined-category defaults are POSIX values where the reference's differ, no compat
  option (recommend: keep, no option). (b) D-31.26 `--big-endian` refused, no compat option (recommend: keep).
* INFORMATIONAL: reference.rs tests `en_us_utf8` and `generated_charmaps_compile_like_the_reference` shared one
  temp dir (pid + locale) and could remove each other's output: a flake on next, fixed here.
* IMPACT: correctness (archive writer + 3 options; 12 files always; stdin; -c; setlocale(LC_ALL) on a six-category
  source), perf/size (RSS -51..-79 %, measured), coverage (8 prog tests + 1 products + 1 core unit, 8 plants).
* IMPROVEMENT idea: the plants file format splits on ` | `, which is easy to trip with Rust closures (`|&r|`).

