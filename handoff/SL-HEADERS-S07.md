# SL-HEADERS-S07: slice 7, POSIX files and processes (handoff after run 3, SL-HEADERS-S07-3)

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
  alone (1 of 1, same as the reference build). xz-utils does not build: it includes `<pthread.h>` (slice 8). sqlite3: see the report file.
* Decision (pending, `records/pending/SL-HEADERS-S07-3-1.toml`, D-NEW series D-X), differences row after `bsd_signal` for the reserved names.
* Plants: `handoff/SL-HEADERS-S07/plants.txt` (18; 7 new for this run's generator rules and `header_s07.py`).

**Open.** P7 for xz-utils and sqlite3 waits for slice 8 (`pthread.h`, `signal.h`); P2/P3 rows of the five SL-HEADERS-S07-TAIL headers go with them.
Pending ids: the two `COMPAT-NEW` rows of S07-2 and this run's `D-NEW` get ids from CI; the differences rows say "pending".

**Things not to redo.** abi-gen, the table reruns, the earlier generator additions. Never run `abi_extract.py` without flags (drops two `sched_*attr` rows).

**Next step.** SL-HEADERS-S07-TAIL after slices 8 and 14; re-run `smoke.sh --niji-headers xz_utils sqlite3` and `undef-compare.sh` when `pthread.h` lands.
Plan rows: plan/headers.md section 3 (P-list), section 4 slice 7.
