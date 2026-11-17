# SL-HEADERS-S07: slice 7, POSIX files and processes (handoff after run 2, SL-HEADERS-S07-2)

**State.** Branch `task/sl-headers-s07-2` (continues `task/sl-headers-s07`). Head: `handoff/reports/SL-HEADERS-S07-2.json`. Scope is the
24 generated headers: `fcntl.h sys/fcntl.h sys/stat.h sys/statvfs.h sys/statfs.h sys/vfs.h dirent.h sys/dir.h sys/time.h sys/times.h utime.h
sys/resource.h sys/utsname.h sys/select.h poll.h sys/poll.h sys/mman.h sys/uio.h sys/file.h sys/sysmacros.h paths.h libgen.h sys/xattr.h
sys/sendfile.h`. Say "slice 7" in commits. Clippy and static clean on the head.

**Scope change (ruling 11-17, #6379).** `sys/wait.h`, `wait.h`, `sys/param.h`, `spawn.h`, `sys/ioctl.h` move to story SL-HEADERS-S07-TAIL, held
until slices 8 and 14 land. No partial state for them is on this branch (no template, no table row, no `header_check.py` entry; only the
comment in `header_check.py` and this note name them). Their P10 `W*` run goes with them.

**Red gate cause (fixed).** `cargo xtask stubs` reported `abi_struct_statx_timestamp` with neither stub nor test: the four
`struct statx_timestamp` rows of `layouts.tsv` were never run through `cargo xtask abi-gen`. Regenerated `layout.rs` and
`appendix_c_layouts.rs` (commit "regenerate abi layout"). The `cargo xtask test` red had the same first step (`stubs`); a rerun on
the fixed tree was started and stopped at the `cargo test` stage to sync later edits, so `xtask test` on the final head is the gate's job.

**Done this run.**
* Rulings 3: `getumask` stays omitted, `bsd_signal` not declared (symbol exported). Differences rows (after `__STDC_ISO_10646__`) and
  pending records `records/pending/SL-HEADERS-S07-2-{1,2,3}.toml` (decision D-NEW series D-X; two COMPAT-NEW, registered, not implemented).
  The rows say "COMPAT-NEW (pending id)": replace with the assigned ids when CI assigns them.
* Proofs on the 24 headers (keke): P1 and P5 0 failed of 3245; P10 (`spec/tools/header_p10.py`: `FD_*` over 0..1023, `major`/`minor`/
  `makedev` over 20003 device numbers, -O0 and -O2, against the system's headers) 0 failed; P8 `sys/xattr.h` with `linux/xattr.h`, both orders,
  with and without `_GNU_SOURCE`: 0 failed (`--uapi target/niji/uapi`).
* Fixes the proofs drove: `endian.h` macros only under `__NIJI_MISC` (sys/types.h leaked them into strict modes); `paths.h` includes nothing;
  `major/minor/makedev` expand in place (`gnu_inline` always-inline `gnu_dev_*`, address still binds to the library); `FD_SET/CLR/ISSET` index through
  `__fdelt_chk` (also in `sys/types.h`); `sys/xattr.h` defines `XATTR_CREATE/REPLACE` only if the kernel header has not.
* Plants: `handoff/SL-HEADERS-S07/plants.txt` (11).

**Open (not green).**
* P2: 32 differences of 1839 rows (`header_probe.py --include libc-include --headers <24> --out D`, `probe_compare.py visibility`). (a) `_STRUCT_TIMESPEC`
  and `_SIGSET_NWORDS` (17 rows: reference-internal single-underscore names libniji does not define; propose accept with a differences row, no option:
  reserved names, not interfaces); (b) `struct timespec` visible in strict configurations in `fcntl.h sys/fcntl.h sys/file.h sys/stat.h` (4 rows: `struct stat`
  members always use it; needs the generator to give `struct stat` pre-2008 timestamp members without the name); (c) `__ino64_t __off_t __pid_t` missing
  in strict configurations of `sys/uio.h sys/xattr.h sys/sendfile.h` (9 rows: R-H3 reserved typedef names, emitted only where a prototype needs them);
  (d) `getumask` (1 row, accepted by ruling 3).
* P3: 32 differences of 369 rows. The `*_chk` and `+gnu64`-with-fortify rows (`open`, `openat`, `poll`, `ppoll`, `lstat`, `fallocate`, `prlimit`, `preadv2`,
  `scandirat`, `versionsort`, ...) are slice 18 (fortify); `getumask addr` (ruling 3). Not classified one by one.
* P6 (`cargo xtask so`; `conformance/run.sh --niji-headers`) and P7 (zlib, xz-utils, sqlite3) were NOT run (budget). P3 and P2 above ran on the tree before the
  last two template fixes only for `sys/xattr.h` (not in the diffs).

**Things not to redo.** abi-gen, the two table reruns, the generator additions (`xtask/src/headers.rs`). Never run `abi_extract.py` without flags (drops two
`sched_*attr` rows).

**Next step.** Classify P2 (a)-(c) and P3 (accept file with differences rows, or fix: (b) is generator work); P6, P7; then SL-HEADERS-S07-TAIL.
Plan rows: plan/headers.md section 3 (P-list), section 4 slice 7.
