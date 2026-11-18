# SL-HEADERS-VIS-FIX: endian.h, sys/types.h and the slice 5/6 sweep; struct timespec through struct stat

**State.** Branch `task/sl-headers-vis-fix` (cut from origin/next 852c1b495, no merge). Code commits 9fa84286d (templates)
and 5a2898433 (generator marker, `sys/pidfd.h`, row removal); head and jobs in `handoff/reports/SL-HEADERS-VIS-FIX.json`.
Decision D-X.127 (reserved for this story). Role: mei `~/w/s3`.

**endian.h and sys/types.h (the two P2 findings of SL-HEADERS-S10-2).** On this base both already match: the P2 rerun
(`header_probe.py --include libc-include`, `probe_compare.py visibility`) gives 0 differences for `endian.h` and
`sys/types.h`, and their `be16toh`/`le64toh` and `__off_t`/`__ino64_t`/`__pid_t` rows in the rerun are byte-identical to
`spec/abi/visibility.tsv`. The endian(3) family (all 24 macros, `htobe16` .. `le64toh`) sits under one `@@if be16toh`
since slice 9 run 3 (landed); the finding came from a tree before that. Nothing changed for them.

**Sweep (slices 5 and 6, plus `sys/pidfd.h`; 28 headers, 40 configurations, 3951 rows).** Before: 50 differences.
Of the class "a name shown in configurations the reference does not show it in":
* `features-time64.h` (13 extra feature-test macros) and `gnu/stubs.h` (13, plus an include error under `_TIME_BITS=64`
  alone): both included `<features.h>`. Now they do not; features-time64.h does the _TIME_BITS check itself.
* `stdlib.h` `WIFCONTINUED`: shown under `_XOPEN_SOURCE` defined empty; now its own `@@if WIFCONTINUED`.
* `sys/pidfd.h` `struct timespec` under `_XOPEN_SOURCE` < 700: see below.
After: 21 differences, all of the other class (names libniji MISSES; not this story's): `alloca` in `alloca.h`,
`_LIBC_LIMITS_H_`, 18 in `ctype.h` (the `*_l` macros, `_tolower`/`_toupper`/`isascii`/`toascii` not shown in C++),
`fwrite_unlocked` macro under fortify (slice 18).

**struct stat.** New core marker `@@hidden-member MEMBER NAME...` (xtask/src/headers.rs, its own commit): where the
member's structure tag is visible in fewer configurations than the aggregate, outside the tag's condition the member is
spelled as the structure's scalar members under the given names. `sys/pidfd.h.in` uses it for `st_atim`/`st_mtim`/`st_ctim`
(-> `st_atime` + `__st_atimensec`, ...) and defines `st_atime` etc. as `st_atim.tv_sec` under `@@when timespec`. Black-box
probe (`statprobe`, 11 compile tests x 9 configurations: timespec complete, `st_atime` macro or member, its type and offset
72, `st_mtime` 88, `st_ctime` 104, size 144/align 8, `stat64`, a program's own `struct timespec`): libniji identical to the
reference. The interim differences row of slice 9 and `handoff/SL-HEADERS-S09/p2-accept.txt` are removed.
A later `sys/stat.h` or `fcntl.h` template that gives `struct stat` must carry the same three `@@hidden-member` lines and
the `st_atime` block (copy from `sys/pidfd.h.in`).

**Plants** `handoff/SL-HEADERS-VIS-FIX/plants.txt`: WIFCONTINUED condition reverted, gnu/stubs.h including features.h,
the generator back to the single struct stat. Run 1 (`sl-headers-vis-fix-plants`, head 5a2898433): 0 CAUGHT, 3
WARN-NO-MSG: their `sh -c` commands make the tool rebuild the products first, and `cargo xtask so` refuses the stale
`libc-include/` (`headers-gen: stale under libc-include/: stdlib.h`) before P2 runs. So P2 never ran on the mutants.
(P2 itself does see each row: the pre-fix baseline `sl-headers-vis-fix-p2base` listed exactly these differences.)

**Run 1 results (mei s3, head 5a2898433).** `sl-headers-vis-fix-lint`: clippy (workspace, all targets, all features,
-D warnings) ok, static ok, headers-gen --check ok (67), header_check 0 failed of 4563 (P1/P5/P8 unchanged), xtask
headers tests 8 passed. `sl-headers-vis-fix-after1`: P2 21 differences (from 50), stat probe identical to the reference.
The FULL gate `sl-headers-vis-fix-gate` (escalated: xtask changed) was stopped (kill-label) when the WIP test below was
added: superseded, never finished.

**WIP (uncommitted-proof).** Commit "WIP: ..." adds the unit test `headers::tests::p2_visibility_of_the_fixed_rows`
(xtask/src/headers.rs): generate() into a temp dir, header_probe.py + probe_compare.py over stdlib.h, gnu/stubs.h,
features-time64.h, sys/pidfd.h; asserts 0 differences, the differences in the panic message. NOT YET COMPILED OR RUN.
Problem left: a `cargo test -p xtask --bin xtask ...` plant still rebuilds products (xtask's sources name
`target/niji`), so the stale check would again catch the mutant before the test. Ways out, pick one:
(a) an integration test target whose own source names none of the product markers (docs/plants.md "Which commands
rebuild the products") that runs `CARGO_BIN_EXE_xtask headers-gen` into a COPY of the tree, or a new `headers-gen
--out DIR` option; (b) RULING NEEDED: a plants option to skip the product build for header-template plants.

**Exact next step.** Sync, compile and run the WIP test on mei (`cargo test -p xtask --bin xtask
p2_visibility_of_the_fixed_rows`); choose (a) or report (b); rewrite plants.txt cmds to that named test (msg regexes
stay); run plants, then the gate (`cargo xtask affected --run --base origin/next`, escalates to FULL), wait; precheck
`--jobs <labels> --head <sha>`; report json `handoff/reports/SL-HEADERS-VIS-FIX.json`; handback-check.

**Not to redo.** The reference stat probe; the P2 baseline counts above; lint on 5a2898433.
**Not this story.** The "missing" class above belongs to the ctype/alloca/limits/fortify owners.
