# TZ-TOOLS: `zic` and `zdump` in libc-bin (GitHub #14, the rest)

**State.** Branch `task/tz-tools-2` (TZ-TOOLS-2, on `task/tz-tools` `6dc74ea1c`, from origin/next `7e5d8b3fa`),
role keke `~/w/n1`. The jobs, the code head and the results are in `handoff/reports/TZ-TOOLS-2.json` and the
TZ-TOOLS-2 section below. TZ-TOOLS (first run): code head `dedddfd3d`, `tz-tools-gate5`, `tz-tools-plants5`
(8 caught, zone-name-check survived, keep-noops and expiry-mark did not compile: both fixed in TZ-TOOLS-2).

## TZ-TOOLS-2 (coordinator 11-21: zone-name-check survived; zic's names are a security boundary)

* Why the plant survived: the only hostile-name case also held a bad link name, which the link-name check (not
  planted) refused before anything was written. The new tests below catch it.
* `zic` output (D-31.18): every name from the source is walked from a descriptor of `-d` (`mkdirat`, then
  `openat(O_NOFOLLOW)`), so no symbolic link below `-d`, from this run, an earlier one or anyone, is followed for
  a write; files are written at a short private name and renamed (a 255-byte component now works: the old
  temporary name added `.zic-tmp`); a Link line's hard link is first made at a private name at the top of `-d`
  and renamed into place, and `EPERM` (a directory target) is an error, `link from <path> failed`, before any
  directory is made (probed). Absolute link targets stay absolute in the symbolic link (probed). Name errors use
  the reference's messages and order: `empty file name`, `begins with '/'`, `contains '.' component`, `'..'`,
  `'//'`, `ends with '/'` (probed). The path limit applies to the name alone (4095 bytes fit, 4096 give
  `Can't remove <path>: File name too long` after `-d` is made; probed). `-l`, `-p`, `-t` keep the old path code.
* Tests: `zic_handles_zone_and_link_names_as_the_reference_does` (54 names as zone and link name, 7 link
  targets: status, stderr and every entry left, including symbolic link contents);
  `zic_never_writes_through_a_symbolic_link_below_the_output_directory` (libniji only: 3 source link chains,
  4 links already in the tree; nothing outside `-d` changes).
* Plants: keep-noops and expiry-mark rewritten to compile (`if (0 && ...)`), messages naming their tests;
  zone-name-check moved into the name check; new link-to-directory, walk-follows-links, name-length-limit,
  component-length-limit (15 plants).
* Not compared with the reference, on purpose: what it does with links already in the tree or with link chains.

## On the branch

* `csys/util/zic.c`: the zic(8) input language (Rule, Zone, Link; Leap and Expires in the `-L` file, and the
  obsolescent `#expires` comment with the reference's warning), options `-d`, `-b fat|slim` (fat default), `-L`,
  `-l`, `-p`, `-t`, `-v`, `--version`, `--help`; `-r` is refused (exit 1, not implemented). Output: TZif version 2,
  3 when the footer needs a weekday shift or an hour outside 0..24. Fat: a 32-bit block with a transition at the
  lowest 32-bit time when earlier ones exist, transitions of rules that last forever up to 2037, indicator arrays
  only when an entry is set, and (observed) a transition at the highest 32-bit time when the footer quotes an
  abbreviation. Slim: an empty 32-bit block; the transitions stop once every perpetual rule has taken effect in
  the last line. An expiring leap table ends the data at the expiry with no footer. Links: a hard link, else a
  relative symbolic link with the reference's warning (EPERM: an error). Errors: `"FILE", line N: MESSAGE`, exit
  1, nothing written. Output below `-d` only, never through a symbolic link (D-31.18, TZ-TOOLS-2 above).
* `csys/util/zdump.c`: `-v`, `-V`, `-i`, `-c`, `-t`, `--help`, `--version` over `localtime_r`/`gmtime_r`,
  sampling every twelve hours (zdump(8)) and narrowing each change to the second; the `fewer than 3 characters`
  warning; names padded to the longest.
* `xtask/src/so.rs`: both built like getent (`target/niji/{zic,zdump}`, test links `target/niji-test/`).
* `tools/deb/manifest.tsv`: libc-bin ships `/usr/sbin/zic` and `/usr/bin/zdump` (stripped, debug). No manual
  pages (the other libc-bin utilities ship none either).
* `xtask/tests/tz_tools.rs`: corpus (fat, slim, `-L`), zdump over every installed zone, malformed and hostile input.

## Proof (keke, Ubuntu 24.04 reference, tzdata 2026e `tzdata.zi`, 598 zone files incl. hard-linked aliases)

| check | result |
|---|---|
| zic fat (default), decoded: transitions with indicators, first type, leaps, footer, both blocks | 593 / 598 identical; 198 byte-identical |
| zic `-L leapseconds` | 593 / 598 identical |
| zic `-b slim` | 590 / 598 identical |
| the remaining zones, read through libniji's zdump (`-V -c 1800,2100`) on both files | same reading (test asserts) |
| zdump `-v -c 1800,2100` and `-i -c 1800,2100`, libniji (its localtime) vs reference, every installed zone | 497 / 497 identical |
| zdump on the host C library, full default range `-v`, `-V`, `-i`, `-i -t 0,2000000000` (dev loop) | 598 / 598 identical |

Not byte-identical (format freedom, RFC 9636): the order of the type table, unused types, abbreviation order.
Decoded differences left (listed in the test with reasons): fat/`-L` Asia/Tbilisi and Europe/Lisbon (the reference
keeps one transition that changes nothing visible), Europe/Warsaw (standard-time indicator of the 1918 change);
slim also America/Ciudad_Juarez, America/Ojinaga, America/Indiana/Winamac, Asia/Gaza, Asia/Hebron (handover to the
footer one transition earlier or later). The listed zones are read (libniji's zdump, 1800-2100) against the
reference's fat file, which lists every source transition: all read the same. America/Ojinaga: libniji's slim file
reads CST from 2022-10-30 as the source gives; the reference's slim file reads CDT until 2022-11-06. Tried and rejected: keeping every no-op line start (361/598).

## Not done

* The coexist flavour ships no libc-bin utilities today (getent, iconv, locale are system-only); zic/zdump follow.
* The swapped trixie root building Debian's tzdata package with this zic: not run (run budget); `-b fat` with
  `-d` and `-L` is what that build uses and is covered by the corpus. Lintian not run: the deb build exports HEAD
  and rebuilds release products (one product build per role); the manifest lint (`deb_manifest`) is in the gate.
* `zic -r`; zic's diagnostics for abbreviations made by `%s` (only literal formats are checked); `zic -v`'s
  extra warnings (non-portable name bytes, components over 14 bytes and the others zic(8) lists): `-v` is accepted
  and ignored. `-t` with an absolute path still lands under `-d` (the old path code, unchanged; not probed).
* zdump checks a malformed `-c`/`-t` even when no listing is asked for (the reference ignores it then).
* Fuzzing: the hostile-input test (11 cases: huge years and offsets, 300 types, link loop, escaping names, NUL,
  1 MiB line, 5000-letter abbreviations); no random mutation run.

## Next step

Land after `tz-tools-2-gate1` and `tz-tools-2-plants1` (15 plants, all must be CAUGHT); then UTIL-12 (localedef).
Plan rows: plan/utilities.md §2.3, §4 (tz tools); spec §31.9, §31.10; D-31.18.
