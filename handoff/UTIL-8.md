# UTIL-8: the `locale` command (issue #14) — INCOMPLETE (context checkpoint)

**State.** Branch `task/util-8` (from origin/next `575214f4b`), role mei `~/w/s1`. Three WIP code commits; head = the
commit that adds this note. Builds clean (`cargo xtask so`, job `util-8-so3` rc 0). No transcripts, plants or gate yet.

## On the branch

* `csys/util/locale.c`: front end over `setlocale`/`nl_langinfo` (no operands: the environment listing; keyword and
  category operands with `-k`/`-c`; `-a` (archive name table per D.1.6, directories holding `LC_IDENTIFICATION`,
  built-in `C`, `C.utf8`, `POSIX`, `strcoll` order), `-a -v`, `-m`, `--help`, `--usage`, `-V`, argp-style option errors
  exit 64). Keyword table = the reference's 180 keywords (`locale -k LC_<cat>` probes), each on its public
  `<langinfo.h>` item; value kinds per Appendix D.1.5.
* `xtask/src/so.rs`: `run_dynamic_util` (getent's link steps, shared) builds `target/niji/locale` and the test link
  `target/niji-test/locale` (`LOCALE_TEST`); `transcript/mod.rs` maps `locale` to the test link.
* `tools/deb/system.sh` ships `/usr/bin/locale` in libc-bin (stripped); `check-system.sh` allows it. NOT VERIFIED: no
  package built.

## Compared with the reference on mei (scratch script, not a transcript; 20 cases)

Same: no-operand listings (unset, `LANG`+`LC_TIME`, empty variables, missing locale and its `Cannot set` lines),
`-ck` mixed keyword/category, unknown names (first one ends the run, exit 1), `-a`, `--usage`, `-x`.
Different:
1. **`-a -v` bug (front end, fix first):** `qsort` sorts `s.v` but not the parallel `s.where`, so sources are
   misattributed. Sort `{name, where}` structs instead.
2. **`-m`**: 3 names missing (`MAC_CENTRALEUROPE`, `NF_Z_62-010_(1973)`, `WIN-SAMI-2`): `<code_set_name>` of `.gz`
   charmaps is not read (the reference decompresses with external gzip/bzip2). Recommend: inflate in-process (libniji
   has `core::charmap::inflate`; needs a C-reachable path or a C inflater) — RULING-worthy if a private export.
3. **Built-in C / C.UTF-8 (library, not the front end):** `nl_langinfo` gives empty `_NL_CTYPE_CLASS_NAMES`/`MAP_NAMES`,
   a non-table `_NL_CTYPE_WIDTH`, and different `CLASS_OFFSET`/`MAP_OFFSET` (ref 72/84 and 72/86). A §7 story.
4. **Loaded files, two raw-layout keywords:** `time-era-entries` (ref prints the first bytes of the binary era data,
   e.g. `"S"`), `category` (ref walks 13 strings from the item, reading into the next item: `"i18n:2012;UTF-8;;…"`).
   libniji prints the item's own string. Recommend a differences row (reading past an item is not safe at a file's end).
5. `--help` bug-report lines and `-V` identity: D-31.4 rule (normalise in transcripts, as getent).

## Probe-settled (reference, black box)

`Cannot set <CAT> to default locale: <strerror>` for each failing `setlocale(cat,"")`: LC_CTYPE, LC_MESSAGES, then
LC_ALL (queries) or LC_COLLATE (`-a`, `-m`). Listing: `LC_ALL` non-empty → every category `"<LC_ALL>"`; a set category
variable (even empty) bare; else `"<LANG or POSIX>"`. `-a` lists a directory only when it has `LC_IDENTIFICATION`;
`-v` header `locale: %-15.15s directory: <path>`, 79 dashes, non-empty identification fields as `%9s | %s`, a blank
line between locales. With `LOCPATH` set the reference lists builtins then archive unsorted (not matched). `-m` lists
file names minus `.gz`/`.bz2`, hidden files too, plus `<code_set_name>`; a missing directory: `locale: [error] cannot
read character map directory `…': <strerror>` exit 1. `-a -m`/`-a NAME`: `-a` wins. Byte items signed (0xff → -1),
empty grouping → -1, `conversion_rate` two words `1;1`.

## Next step

Fix item 1; write `validation/transcripts/locale/` (C, C.UTF-8, private `/usr/lib/locale` and charmap roots via
`bind:`; host-archive cases for en_US); record on mei; plants file (`handoff/UTIL-8-plants.txt`: kinds BYTES/BYTE/SARR,
env quoting, first-unknown exit, `-a` filter); `records/pending/UTIL-8-1.toml` (C front end now, folded into
`niji-locale` by UTIL-12) and differences rows for items 2-4; then gate. Read: plan/utilities.md §2.3/§4, spec §31.7,
this note, handoff/UTIL-GETENT.md.
