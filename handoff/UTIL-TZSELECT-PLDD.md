# UTIL-TZSELECT-PLDD: tzselect, pldd and the /usr/bin/ld.so link (GitHub #14, the rest)

**State (leg 1, REFRESH).** Branch `task/util-tzselect-pldd` from origin/next `88c0518f6`, role keke `~/w/n2`.
Done: probes of the reference (all below), `csys/util/tzselect.c` written (NOT yet compiled or tested).
Not started: pldd.c, so.rs, manifest rows, tests, docs, plants, gate. No jobs run yet.
Scratch (outside the tree): `~/w/.scratch/UTIL-TZSELECT-PLDD/` (local: probe scripts `probe1-5.sh`, `pt.c`, fit
scripts, reference transcripts under `ref/p1..p3`, tab files `ref/tab/`); keke `~/work/util-tzselect-pldd/`
(same probes, `p1..p4` outputs, `pt` target binary). Probe scripts copy with `scp -o BatchMode=yes F claude@<keke>:work/util-tzselect-pldd/`.

## Challenge to the brief (flag it in the hand-back)
zic and zdump are NOT one `niji-tz` executable chosen by argv[0]: they are separate C front ends
(`csys/util/{zic,zdump}.c`, `run_dynamic_util` in `xtask/src/so.rs`). tzselect and pldd follow that shape:
`csys/util/tzselect.c`, `csys/util/pldd.c`, built by `run_dynamic_util(root, cc, "<name>", <NAME>_TEST, cdebug)`
beside zic/zdump in `run_iconv`, test links `target/niji-test/{tzselect,pldd}`.

## tzselect, probed (keke, Ubuntu 24.04, tzdata 2026f; transcripts in scratch ref/p1..p3)
* Tables from `$TZDIR` (default /usr/share/zoneinfo): iso3166.tab + zone1970.tab; zonenow.tab not needed. Missing:
  `<argv0>: line 179: <path>: No such file or directory` + `<argv0>: time zone files are not set up correctly`, rc 1
  (libniji drops `line 179:`: differences row).
* Prompts on stderr; value on stdout once confirmed. Flow: `Please identify a location so that time zone rules can
  be set correctly.` then `Please select a continent, ocean, "coord", or "TZ".`; menu = continents present in
  zone1970.tab (prefix map America->Americas, Atlantic/Indian/Pacific -> "... Ocean"), sorted, then `coord - I want
  to use geographical coordinates.` and `TZ - I want to specify the timezone using the Posix TZ format.`
* Country menu header `Please select a country whose clocks agree with yours.`: countries of zones with that
  prefix, sorted by strcoll (C.UTF-8 bytes: Åland last; en_US: first). Zone menu `Please select one of the following
  timezones.`: comments of every zone1970 line holding the code (any continent), file order. One zone: no menu.
* Info block: `\nThe following information has been given:\n\n\t<country>\n[\t<comment>\n]\nTherefore TZ='<tz>'
  will be used.\nSelected time is now:\t<%a %b %e %H:%M:%S %Z %Y>.\nUniversal Time is now:\t<same, UTC>.\nIs the
  above information OK?\n` + menu Yes/No. Same C-format times under en_US. Yes: value + newline on stdout, rc 0.
  No: restart from "Please identify". coord: `\tcoord <COORD>\n\t<label>\n`; TZ: `\tTZ='<v>'\n`.
* Menu answers: `#? ` prompt (no newline); EOF -> rc 1 silently; empty line -> menu again (no header); accepted:
  blanks around, `+`, leading zeros (`+1 01 " 1 " \t1 00001 +01`); refused with `Please enter a number in range.`:
  `0 12 abc 1. 0x1 1e0 -1 1a "1 2" +3 03(out of range)`.
* Layout (also probed on bash select directly, probe5): colw = max(digits(n)+2+chars(item)) + 2; cols =
  COLUMNS/colw (COLUMNS positive int else 80; 0/-5/abc -> 80), min 1; rows = ceil(n/cols); rows==1 -> one per line;
  column-major; column 0 numbers `%*d` with digits(rows), other columns digits(n); pad to column start with tabs
  (stops of 8) then spaces; no trailing pad. Width counts characters, not bytes (synthetic TZDIR probe).
* LANG=C: names converted with transliteration (`Côte d’Ivoire` -> `C?te d'Ivoire`), sorted after conversion.
* -c COORD: header `Please select one of the following timezones, echo listed roughly in increasing order of
  distance from <COORD>.` (typo kept verbatim), then menu of LIMIT (default 10) labels = country names joined ", "
  (code US stays `US`, every other code mapped) + ` - <comment>`. Always a menu, even with 1 entry. ISO 6709 DDMM,
  DDMMSS, DD, decimals accepted; garbage COORD lists as from +0000+00000. ORDER: not reproduced by haversine, law of
  cosines, chord, weighted Euclid/L1, deg/rad or DD.MM misparses (fit*.py): libniji uses exact great-circle order
  (D-31.27 pending id: reserve `D-31` with `ops reserve`; differences row; RULING NEEDED in hand-back).
* -n: `-n abc` gives a garbled list in the reference, `-n 0` a sed error and TZ='': libniji refuses LIMIT that is
  not a positive number (usage error, rc 1; same differences row).
* Options: `--help` (stdout rc 0, processed in order), `--version`, `-x` -> `<argv0>: illegal option -- x` +
  `<argv0>: try '<argv0> --help'` rc 1; `-c` w/o arg -> `option requires an argument -- c` + try line; operand ->
  `<argv0>: extra: unknown argument` rc 1; `--` ends options.
* TZ answer: prompt `Please enter the desired value of the TZ environment variable.\nFor example, AEST-10 is
  abbreviated AEST and is 10 hours\nahead (east) of Greenwich, with no daylight saving time.\n`; bad ->
  `'<v>' is not a conforming Posix timezone string.` and prompt again. Accepted/refused table: scratch
  ref/p3/tz*.tz + tz*.err (41 strings); tz_valid() in tzselect.c encodes it.

## pldd, probed (scratch probe4.sh; keke ptrace_scope 1)
* Output: `<pid>:\t<readlink /proc/pid/exe>` then each link_map l_name of the r_debug chain after the main
  program, in chain order: `linux-vdso.so.1`, `/lib/x86_64-linux-gnu/libc.so.6`, `/lib64/ld-linux-x86-64.so.2`,
  then dlopen'd `/lib/.../libm.so.6`, `libz.so.1` (names as the loader recorded them, not realpaths). rc 0.
* PID parsing: `+N`, ` N`, `0N` accepted (decimal); `abc`/`-5` -> `pldd: invalid process ID 'abc'` rc 1;
  0 or absent -> `pldd: cannot open /proc/<pid>: No such file or directory` rc 1; not traceable (pid 1, 2, own
  shell, a sibling without PR_SET_PTRACER) -> `pldd: cannot attach to process <pid>: Operation not permitted` rc 1;
  static target -> rc 1, no output. No operand or two: `Exactly one parameter with process ID required.` + `Try
  \`pldd --help' or \`pldd --usage' for more information.` rc 1. Bad option: `pldd: invalid option -- 'x'` /
  `pldd: unrecognized option '--bogus'` + Try line, rc 64. `--usage`: `Usage: pldd [-?V] [--help] [--usage]
  [--version] PID`. `--help`/`-?`: argp text (see keke p4/help.out; drop the bug-report lines as gencat does).
  `-V`: `pldd (libniji) 2.39` (house form).
* Mechanism (decision record to write): PTRACE_SEIZE+INTERRUPT the tasks for permission and a stopped snapshot,
  read /proc/PID/auxv (AT_PHDR, AT_PHNUM) -> PT_DYNAMIC -> DT_DEBUG -> r_debug -> link_map chain via /proc/PID/mem,
  bounded walk (loaded data is untrusted: cap entries, cap string length), detach.

## /usr/bin/ld.so
Reference: libc-bin owns `/usr/bin/ld.so -> ../lib64/ld-linux-x86-64.so.2`. Rows to add to tools/deb/manifest.tsv:
`system libc-bin usr/bin/ld.so link ../lib64/ld-linux-x86-64.so.2` and coexist `niji6
usr/lib/niji/usr/bin/ld.so` (both rows only allow libc6/libc6-dev, so two rows). Check in tools/deb/root-system.sh
after the `ldconfig -p` check (line ~200): `in_root /usr/bin/ld.so --version` equals the /lib64 path's, and
`--library-path /usr/lib/$ma /tmp/hello` prints `hello from niji`; tzselect/pldd smoke there too.

## Exact next step
1. Write csys/util/pldd.c; add both to so.rs (`TZSELECT_TEST`, `PLDD_TEST`) and manifest (`usr/bin/tzselect`,
   `usr/bin/pldd` libc-bin, debug) + ld.so rows; docs list of shipped commands (grep `zdump` in docs/).
2. Test `xtask/tests/tzselect_pldd.rs` (pattern of tz_tools.rs, `mod test_link;`): scripted sessions vs
   /usr/bin/tzselect with the two time lines masked (continents 1-9, US zone, No then Yes, junk answers, TZ good
   and bad, coord menu, -c labels as a set with -n 400, -c order checked against an in-test haversine, LANG=C
   Africa, COLUMNS 40/200, missing TZDIR, options); end of input at every prompt exits 1 (libniji only).
   Before the hand-back read scratch `NOTES-PRIVATE.md` (not for the tree) and copy its section into the hand-back.
   pldd: target with PR_SET_PTRACER_ANY + dlopen, reference pldd vs ours identical; errors identical.
3. Plants (docs/plants.md form): -c order broken (sort comparator), pldd stops at the loader entry, manifest ld.so row
   removed. Then clippy, static, gate, plants, precheck, report json, handback-check.
