# SL-HEADERS-VIS-FIX: endian.h, sys/types.h and the slice 5/6 sweep; struct timespec through struct stat

**State.** Branch `task/sl-headers-vis-fix` (cut from origin/next 852c1b495, no merge). Head, jobs and results:
`handoff/reports/SL-HEADERS-VIS-FIX.json`. Decision D-X.127 (reserved for this story). Role: mei `~/w/s3`.
Commits: 9fa84286d (templates), 5a2898433 (generator marker `@@hidden-member`, `sys/pidfd.h`, interim row removed),
8e69892cf + 55167f138 (`headers-gen --out DIR`, test target `xtask/tests/header_p2_fixed_rows.rs`), fd7eceefb (shared
tooling, its own commit: the plants tool judges xtask test targets), caf0d8c6f (plants).

**endian.h and sys/types.h (the two P2 findings of SL-HEADERS-S10-2).** On this base both already match: P2
(`header_probe.py --include libc-include`, `probe_compare.py visibility`) gives 0 differences for `endian.h` and
`sys/types.h`; their `be16toh`/`le64toh` and `__off_t`/`__ino64_t`/`__pid_t` rows are byte-identical to
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

**struct stat.** Core marker `@@hidden-member MEMBER NAME...` (xtask/src/headers.rs): where the member's structure tag
is visible in fewer configurations than the aggregate, outside the tag's condition the member is spelled as the
structure's scalar members under the given names. `sys/pidfd.h.in` uses it for `st_atim`/`st_mtim`/`st_ctim`
(-> `st_atime` + `__st_atimensec`, ...) and defines `st_atime` etc. as `st_atim.tv_sec` under `@@when timespec`. Black-box
probe (11 compile tests x 9 configurations: timespec complete, `st_atime` macro or member, its type and offset 72,
`st_mtime` 88, `st_ctime` 104, size 144/align 8, `stat64`, a program's own `struct timespec`): libniji identical to the
reference. The interim differences row of slice 9 and `handoff/SL-HEADERS-S09/p2-accept.txt` are removed.
A later `sys/stat.h` or `fcntl.h` template that gives `struct stat` must carry the same three `@@hidden-member` lines and
the `st_atime` block (copy from `sys/pidfd.h.in`).

**P2 as a test.** `headers-gen --out DIR` writes the generated headers below DIR only (no `libc-include/`, no
abi-gen). `cargo test -p xtask --test header_p2_fixed_rows` runs P2 on that output for stdlib.h, gnu/stubs.h,
features-time64.h, sys/pidfd.h: 0 differences (about 110 s on the Skylake-SP host with 8 probe jobs).

**Plants tool (shared tooling, fd7eceefb).** `affected::Repo::load` leaves the xtask crate out, so every xtask test
target was "unknown" to `cargo xtask plants` and rebuilt the products, which refuse a mutant template's stale
`libc-include/` before the test runs (run 1 and the first run of this leg: WARN-NO-MSG for that reason). The tool now
judges `xtask/tests/<name>.rs` by the same text rule (`affected::names_products`); naming a tests/ directory stays unsure.

**Plants** `handoff/SL-HEADERS-VIS-FIX/plants.txt` (6): WIFCONTINUED row, gnu/stubs.h including features.h, the single
struct stat (P2 plant the brief asks for), the xtask rule (two), headers-gen --out. Results in the report json.

**Results (mei s3).** Final tested head e5b548342 (only this note and the report json after it).
`sl-headers-vis-fix-plants4` (8193fe14d code): 6 CAUGHT of 6, each on the baseline build; results
`handoff/SL-HEADERS-VIS-FIX-plants.txt`. `sl-headers-vis-fix-gate2` (FULL, escalated: xtask changed): 32 suites,
12171 passed, 0 failed; conformance 140/0. `sl-headers-vis-fix-precheck` (--head e5b548342): GREEN, 31 checks.
Superseded: plants2 (killed: rebuilt products, before the tool fix), plants3 (headers-gen-out SURVIVED: P2 fell back to
the system headers; the test now requires each probed header), lint2 (did not compile: `gen` reserved), lint3, lint4
(older heads; clippy and static green there and again in precheck), run 1's gate (stopped) and plants (0 judged).

**Exact next step.** Land. A later `sys/stat.h` or `fcntl.h` template that gives `struct stat` copies the
`@@hidden-member` lines and the `st_atime` block from `sys/pidfd.h.in`.
**Not to redo.** The reference stat probe; the P2 baseline counts above; the sweep.
**Not this story.** The "missing" class above belongs to the ctype/alloca/limits/fortify owners.
