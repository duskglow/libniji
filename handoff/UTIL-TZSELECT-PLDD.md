# UTIL-TZSELECT-PLDD: tzselect, pldd and the /usr/bin/ld.so link (GitHub #14, the rest)

**State (leg 2 ended, REFRESH).** Branch `task/util-tzselect-pldd` from origin/next `88c0518f6`, role keke `~/w/n2`,
head `1fd1901ea` (WIP commit; all code, records, docs and the test are in it).
Ids reserved (ops reserve): **D-31.34** (tzselect: -c order, -n, missing tables), **D-31.35** (pldd mechanism),
**COMPAT-43** (reserved, UNUSED: the three differences rows give no-compat reasons; report it unused).
Job running at leg end: `util-tzselect-pldd-t2` (kind test: `cargo xtask so && cargo test -p xtask --test
tzselect_pldd`) on the WIP head. `cargo xtask so` takes ~9-10 min on keke.

## Done
* `csys/util/pldd.c` (D-31.35): PTRACE_SEIZE+INTERRUPT all tasks (rescan /proc/PID/task), auxv AT_PHDR/AT_PHNUM,
  bias from PT_PHDR else AT_ENTRY-e_entry, PT_DYNAMIC -> DT_DEBUG -> r_debug -> link_map (skip first), reads by pread
  on /proc/PID/mem, caps (65536 entries, PATH_MAX names), ELFCLASS32 layout by exe class (UNTESTED: no 32-bit
  target on the host; say NOT VERIFIED). Messages prefixed with argv[0] (reference does that; Try line says `pldd`).
  Static target: rc 1, no output (as reference).
* `csys/util/tzselect.c` (D-31.34): libm-free (13-term sine series; haversine h as the sort key). Fixes from replays:
  "\n" on stdout at EOF at a menu (not at text prompts); no try line after `extra: unknown argument`; under byte-order
  collation ASCII letters compare case-folded ("United States" < "US minor..."); no clock lines when the TZ's current
  gmtoff % 60 != 0 (EST5:30:59, EST-24:59:59).
* so.rs (`TZSELECT_TEST`, `PLDD_TEST`), manifest rows (libc-bin tzselect/pldd/ld.so; coexist niji6
  `usr/lib/niji/usr/bin/ld.so`), records D-31.34/35, 3 differences rows, docs/install-packages.md libc-bin row,
  release-notes fragment.
* `xtask/tests/tzselect_pldd.rs`: sessions vs reference (65 cases), help/version, missing tables, -c set + order vs
  in-test haversine, EOF at every prompt, pldd vs reference (listing, pid forms, errors), pldd options, manifest
  ld.so link resolves in both flavours. Last run (t1, before the fixes above) failed only on: pldd argv[0] prefix,
  `-5` (it is `invalid option -- '5'` rc 64, the leg-1 probe row "neg" was misleading), two TZ clock-line cases,
  tzselect --help (bug line). All fixed in 1fd1901ea, not yet re-run (t2 is that run).

## Reference safety (tests)
Reference tzselect sessions must end on a confirmation or EOF at a continent/country/-c menu, never EOF at the zone
menu or at the TZ prompt (see scratch NOTES-PRIVATE.md; copy its section into the hand-back under SECURITY-PRIVATE,
nowhere else). The test's sessions obey this (ONES tail; TZ cases followed by UTC0 and 1).

## Exact next step
1. `~/bin/jobwait util-tzselect-pldd-t2`; read `grep -n "^test \|panicked" ~/logs/util-tzselect-pldd-t2.log` and
   `grep -A12 panicked`. Fix what fails (find the cause; probe the reference black-box if needed).
2. root-system.sh (tools/deb/root-system.sh, after the `ldconfig -p` check ~line 200, `in_root` = `ns timeout 60
   chroot "$r"`): `in_root /usr/bin/ld.so --version` equals `in_root /lib64/ld-linux-x86-64.so.2 --version`;
   `in_root /usr/bin/ld.so --library-path /usr/lib/$ma /tmp/hello` prints `hello from niji`; `in_root
   /usr/bin/tzselect -c +4852+00220` with input `1\n1\n` prints `Europe/Paris` (needs tzdata in the root: check, else
   TZDIR to a copy); pldd smoke if a traceable process can run in the root. Check how that script is run (a job?).
3. docs/plants.md lines (form `id | path | pattern | cmd | msg:<regex>`): (a) tzselect `by_distance` comparator
   reversed (`dist_of[x] < dist_of[y] ? -1 : 1` -> `? 1 : -1`), msg `tzselect_coordinates_list_nearest_first`;
   (b) pldd stops before dlopen'd objects (e.g. cap the walk at 3 entries: `i < MAX_OBJECTS` -> `i < 3`), msg
   `pldd_matches_the_reference`; (c) manifest ld.so system row removed, msg `manifest_ships_ld_so_link`. cmd:
   `cargo xtask so && cargo test -p xtask --test tzselect_pldd` (check docs/plants.md for the exact form).
4. Squash nothing; new commit(s) on top ("Ship tzselect ..." non-WIP subject for the final). clippy (foreground on
   the role), `cargo xtask static`, then gate (`cargo xtask affected --run` under ~/bin/gate 16G) and plants jobs,
   precheck, report json `handoff/reports/UTIL-TZSELECT-PLDD.json` (with "jobs"), handback-check.
5. Hand-back items: challenge (tzselect is a C front end beside zic/zdump, not a `niji-tz` argv[0] command: the
   tz tools already are separate front ends); RULING NEEDED: -c order is great-circle, not the reference's (no
   documented rule to reproduce; recommend keep); INFORMATIONAL: COMPAT-43 reserved unused; pldd ELFCLASS32 path
   untested; SECURITY-PRIVATE section from scratch NOTES-PRIVATE.md. IMPROVEMENT idea: the leg-1 probe summary
   for `pldd -5` disagreed with a direct run; probe scripts should record the exact argv they ran.
