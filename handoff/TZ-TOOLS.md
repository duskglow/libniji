# TZ-TOOLS: `zic` and `zdump` in libc-bin (GitHub #14, the rest)

**State.** Branch `task/tz-tools` (from origin/next `7e5d8b3fa`), role keke `~/w/n3`. Code head `dedddfd3d`; this
note and `handoff/reports/TZ-TOOLS.json` are the commit after it. Jobs on the code head: `tz-tools-gate5` (direct
tier, `--base origin/next`), `tz-tools-plants5` (`handoff/TZ-TOOLS-plants.txt`, 11 plants). Superseded:
`tz-tools-gate` (clippy: host collections in the new test), `tz-tools-gate2` (the test compared program paths;
zic and zdump name themselves by argv[0] as the reference does), `tz-tools-plants`, `tz-tools-plants2` (stopped /
baseline red for the same reasons), `tz-tools-gate3`, `tz-tools-plants3` (stopped: the slim cut dropped a Gaza/Hebron
transition the footer cannot give), `tz-tools-gate4`, `tz-tools-plants4` (stopped: zdump lacked the reference's
invalid-option line; fixed in the code head); development runs `tz-tools-t1` to `-t3`.

## On the branch

* `csys/util/zic.c`: the zic(8) input language (Rule, Zone, Link; Leap and Expires in the `-L` file, and the
  obsolescent `#expires` comment with the reference's warning), options `-d`, `-b fat|slim` (fat default), `-L`,
  `-l`, `-p`, `-t`, `-v`, `--version`, `--help`; `-r` is refused (exit 1, not implemented). Output: TZif version 2,
  3 when the footer needs a weekday shift or an hour outside 0..24. Fat: a 32-bit block with a transition at the
  lowest 32-bit time when earlier ones exist, transitions of rules that last forever up to 2037, indicator arrays
  only when an entry is set, and (observed) a transition at the highest 32-bit time when the footer quotes an
  abbreviation. Slim: an empty 32-bit block; the transitions stop once every perpetual rule has taken effect in
  the last line. An expiring leap table ends the data at the expiry with no footer. Links: a hard link, else a
  relative symbolic link with the reference's warning. Errors: `"FILE", line N: MESSAGE`, exit 1, nothing written.
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
* `zic -r`; zic's diagnostics for abbreviations made by `%s` (only literal formats are checked).
* zdump checks a malformed `-c`/`-t` even when no listing is asked for (the reference ignores it then).
* Fuzzing: the hostile-input test (11 cases: huge years and offsets, 300 types, link loop, escaping names, NUL,
  1 MiB line, 5000-letter abbreviations); no random mutation run.

## Next step

Land after the gate and plants; then UTIL-12 (localedef). Plan rows: plan/utilities.md §2.3, §4 (tz tools);
spec §31.9, §31.10.
