# SL-HEADERS-S07: slice 7, POSIX files and processes (handoff after run 5, SL-HEADERS-S07-5)

**Run 5 (merge only). OPEN at budget end: gate job `sl-headers-s07-5-gate2` (head c3c4a479f) had passed clippy, static parts and xtask unit tests (the rows test) when the budget ran out; PLANTS NOT RUN yet (`cargo xtask plants handoff/SL-HEADERS-S07/plants.txt`, the file was rewritten for the new files: plants s7-kernel-int, s7-anon-alias, s7-opaque-dir, s7-stat-old-gen/guard/member, s7-enum-param removed with their code; s7-stat-old-hidden, s7-no-packed-chars, s7-enum-tag, s7-slice-row added; paths of s7-lfs-excl moved to functions.rs). Then `ops handback-check`.** origin/next had moved the header list into `headers/slices/*.tsv` (slice 7 is now `headers/slices/s07.tsv`, 24 rows, p1 1), split `xtask/src/headers.rs` into `xtask/src/headers/*.rs` with a general aggregate emitter, and added `@@if` to `endian.h.in`. Merged with both intents: differences, PROVENANCE, reserved.tsv rows of both sides; `endian.h.in` takes next's `@@if be16toh`; generated files regenerated, never merged by hand. Kept from run 4 in the new layout: the `+gnu64` redirects (`lfs`, `extern_lines` in `headers.rs`/`headers/functions.rs`), no `packed` on an all-char structure (`utsname`, `aggregates.rs`). Dropped as covered by the new emitter: `kernel_int`, the anonymous-union alias skip (the table's union is now emitted as the reference has it), `anonymous_struct`, DIR incomplete type. Pre-2008 `struct stat` times now use `@@hidden-member st_atim st_atime st_atimensec` (and mtim, ctim) in the stat, fcntl and file templates (slice 9's pidfd.h uses the same marker with `__st_*` names). `sys/resource.h.in` declares `@@enum __rlimit_resource` (the rows test redeclares the table's prototypes, so the enumeration parameters of prlimit keep their tag; run 4's unsigned-int spelling is gone). `sys/mman.h.in` no longer owns MAP_FAILED (macros.tsv types it now). Checks: `header_check.py` 0 failed of 7359 (run 4: 0 of 3245), `header_s07.py` 0 of 56, `header_p10.py` 0 failed. Shared generator change in this run, named for the hand-back: `xtask/src/headers/functions.rs` (lfs redirects) and one condition in `aggregates.rs` (packed).

**State.** Branch `task/sl-headers-s07-3` (continues `task/sl-headers-s07-2`). Head: see `handoff/reports/SL-HEADERS-S07-3.json`. Scope is the
24 generated headers: `fcntl.h sys/fcntl.h sys/stat.h sys/statvfs.h sys/statfs.h sys/vfs.h dirent.h sys/dir.h sys/time.h sys/times.h utime.h
sys/resource.h sys/utsname.h sys/select.h poll.h sys/poll.h sys/mman.h sys/uio.h sys/file.h sys/sysmacros.h paths.h libgen.h sys/xattr.h
sys/sendfile.h`. Say "slice 7" in commits. `sys/wait.h wait.h sys/param.h spawn.h sys/ioctl.h` are SL-HEADERS-S07-TAIL (ruling #6379), no partial state here.

**Start of run.** Clippy and static were green on the inherited head. The queued S07-2 gate and plants had finished rc 0 (keke).

**Done this run.**
* P2 (28 of 1839 rows differ after the fixes, all accepted) and P3 (18 of 369, all accepted): the accepted rows, grouped by reason, are
  `handoff/SL-HEADERS-S07/p2p3-accepted.txt` (`probe_compare.py ... --accept` reports 0 unaccepted for both).
  * (a) 17 rows `_STRUCT_TIMESPEC`, `_SIGSET_NWORDS`, and (c) 9 rows `__off_t __ino64_t __pid_t` in strict configurations of `sys/uio.h sys/xattr.h
    sys/sendfile.h`: reserved names, one differences row, no compat option (ruling #6443; C 7.1.3).
  * (b) 4 rows `struct timespec` visible in strict configurations: FIXED. `struct stat` and `struct stat64` have `st_atime`/`st_atimensec` (and mtime,
    ctime) members below POSIX 2008 (names settled by compiling against the reference), timespec members from it on. This also fixes `st_atime`
    being an undeclared name in strict modes.
  * (d) `getumask` 2 rows: omitted by ruling #6377 (row in force).
  * P3 real bug, FIXED: `fallocate prlimit preadv2 pwritev2 scandirat versionsort lstat` (and `lockf fseeko ftello mkstemp mkostemp mkostemps ftruncate
    truncate pwrite`, in slice 3/6 headers; regenerated) bound to the 32-bit name under `_GNU_SOURCE` + `_FILE_OFFSET_BITS=64`. The generator reads the
    `+gnu64` call rows of `callsites.tsv` (`lfs` in `xtask/src/headers.rs`, `extern_lines`).
  * 16 P3 rows (`open openat open64 openat64 poll ppoll`) are the `_chk` redirections of `_FORTIFY_SOURCE`: slice 18 owns them (not a difference to record: the
    work is not done yet). They stay in the accepted file under that reason.
* Checks green on the final tree: `header_check.py` 0 of 3245 (P1, P5), `header_p10.py` 0 failed (P10), new `spec/tools/header_s07.py` 0 of 53 (stat members and
  64-bit redirects), P6 `conformance/run.sh --niji-headers` 22 passed 0 failed (116 programs not covered: headers of later slices), clippy and static.
* P7 (new `smoke.sh --niji-headers`, `niji-cc` with `NIJI_SMOKE_HEADERS`, `undef-compare.sh`): zlib builds and passes its test against libniji's headers
  alone (1 of 1, same as the reference build). xz-utils does not build: it includes `<pthread.h>` (slice 8). sqlite3 first failed on `isalpha()`: only `_ISalnum` and `_ISupper` were declared (the visibility probes see macros, not enumerators): FIXED (ten enumerators in `headers/ctype.h.in`, checked by `header_s07.py`), but sqlite3 was NOT rebuilt after the fix (context budget): rerun `smoke.sh --niji-headers sqlite3` (needs `NIJI_FULL_PROFILE=1 cargo xtask so`). `undef-compare.sh` on the first run: every object that differs differs only by `_chk` symbols the reference build calls (`__fprintf_chk`, `__memcpy_chk`, ...: the distribution compiler's default fortify, slice 18); 21 objects identical.
Other enumerators the visibility probes cannot see may be missing in other headers: audit (INFORMATIONAL).
* Decision D-X.118 (`records/decisions/D-X/D-X.118.toml`), differences row after `bsd_signal` for the reserved names.
* Plants: `handoff/SL-HEADERS-S07/plants.txt` (18; 7 new for this run's generator rules and `header_s07.py`).

**Open.** P7 for xz-utils and sqlite3 waits for slice 8 (`pthread.h`, `signal.h`); P2/P3 rows of the five SL-HEADERS-S07-TAIL headers go with them.
Ids (run 4, reserved by the coordinator): D-X.117 (S07-2 decision), D-X.118 (S07-3 decision), COMPAT-32 (getumask, #6377), COMPAT-33 (bsd_signal, #6378); the four records moved from records/pending to records/decisions/D-X and records/compat, the differences rows name them. Static clean, gate green (12099 passed, 0 failed).

**Things not to redo.** abi-gen, the table reruns, the earlier generator additions. Never run `abi_extract.py` without flags (drops two `sched_*attr` rows).

**Next step.** SL-HEADERS-S07-TAIL after slices 8 and 14; re-run `smoke.sh --niji-headers xz_utils sqlite3` and `undef-compare.sh` when `pthread.h` lands.
Plan rows: plan/headers.md section 3 (P-list), section 4 slice 7.
