# HEADERS-STAGE2-GAPS-H: handoff (run 1, end of leg 2)

Branch task/headers-stage2-gaps-h from origin/next 0f7aa0180; role keke:~/w/n1. No decision rows (black-box probes
only). Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-H/` (probe*.py, test1.sh, test2.sh, mkroot.sh, pkg.sh,
ybuild/); keke `~/work/headers-stage2-gaps-h/` (same scripts; `excal/` = Devuan excalibur root, libc6-dev 2.41,
headers only, made by mkroot.sh = job headers-stage2-gaps-h-root41c; `ulsrc/` = util-linux 2.41.5 source excerpts).

## Done (pushed)
* Item 1 (leg 1, 730f215ea): `<arpa/ftp.h>` FTP_NAMES tables (fragment, header_check s13.d/30, conformance program).
* Item 2 sweep (leg 1): results kept below.
* Leg 2: item 3 code in the WIP commits after eac0709db: `headers/sched.h.d/010-sched-attr.frag` + `@@fragments` in
  `headers/sched.h.in`; `headers/slices/s08.tsv` sched.h uapi cell `linux/sched/types.h,linux/sched.h` (P8 both
  orders); header_check fragment `spec/tools/header_checks/s08.d/30-sched-attr.py` (units "sched_attr declared
  sched.h" / "sched_attr absent sched.h"); spec section 24.3 clause text (what <sched.h> gives); 4 sched plants
  appended to handoff/HEADERS-STAGE2-GAPS-H-plants.txt; conformance/c/s13_arpa_ftp_names.c now has
  `/* NIJI-HEADERS: yes */`.

## Two leg-1 defects found by plants1 / pkg2 (fixed in leg 2, need re-proof)
* plants1: 4 CAUGHT, 2 SURVIVED (s2h-swap, s2h-spelling). Cause: the conformance program built against the host's
  stock `<arpa/ftp.h>` (same tables), not libniji's. Fix: the NIJI-HEADERS directive (conformance/run.sh). plants1 is
  SUPERSEDED: rerun all plants (10 lines now) as plants2.
* pkg2: `=== PKG inetutils stock rc=0 libniji rc=2`, ftpd.c `typenames undeclared`. Cause: `libc-include/` is a
  TRACKED generated tree and leg 1 never committed the regenerated `libc-include/arpa/ftp.h` (the deb build copies
  the committed tree). Fix: commit the headers-gen output (next step 1). pkg2 SUPERSEDED.

## 2.41 probe results (probe4.py --root excal; p4-root.tsv on keke; settled, do not redo)
* struct sched_attr, sched_setattr, sched_getattr only under _GNU_SOURCE (C: gnu, gnuc11; C++: g++ default); none
  in c89/c99/c11/gnu17/posix08/xopen7/_DEFAULT_SOURCE/c++11 -U_GNU_SOURCE (tag `struct sched_attr *` ok everywhere).
* Under GNU <sched.h> includes <linux/sched/types.h> (its guard and linux/types.h names appear); sizeof 56 align 8;
  members/offsets/types = kernel VER1 (u32 size 0, policy 4, u64 flags 8, s32 nice 16, u32 prio 20, u64 runtime 24,
  deadline 32, period 40, u32 util_min 48, util_max 52; u64 = unsigned long long).
* New macros under GNU: SCHED_NORMAL 0, SCHED_EXT 7, SCHED_FLAG_RESET_ON_FORK 1, RECLAIM 2, DL_OVERRUN 4,
  KEEP_POLICY 8, KEEP_PARAMS 16, UTIL_CLAMP_MIN 32, UTIL_CLAMP_MAX 64, KEEP_ALL 24, UTIL_CLAMP 96 (int); SCHED_FLAG_ALL
  undefined; redefining KEEP_ALL the kernel's way after it: no warning; as 0x18: warning (kernel spelling).
* Prototypes: C `int (__pid_t, struct sched_attr *, unsigned int)` / `(..., unsigned int, unsigned int)`, not const;
  C++17 noexcept types; NULL attr warns (-Wnonnull) for both; no warn_unused_result, no access attribute.
* Combos: own struct sched_attr + <sched.h> (GNU, either order) = error; linux/sched/types.h or linux/sched.h with
  <sched.h> either order = ok, no warnings; non-GNU: own struct ok.
* util-linux 2.41.5 (read, owner ruling 11-26): configure AC_CHECK_FUNCS sched_setattr; chrt.c/uclampset use
  struct sched_attr (all members incl. util_min/max) and both functions from <sched.h> when HAVE_SCHED_SETATTR;
  schedutils/sched_attr.h defines its own struct only without HAVE_SCHED_SETATTR.

## Jobs running at the end of leg 2 (keke)
* headers-stage2-gaps-h-test2 (`sh ~/work/headers-stage2-gaps-h/test2.sh`, synced tree = WIP commit with the
  sched code, before the spec/plants edits): headers-gen ok (242 headers; ftp.h 3 FTP_NAMES, sched.h 2
  sched_setattr); **xtask headers unit tests: 28 passed, 1 FAILED:
  `headers::slices::s19::tests::diagnostic_attributes_match_the_reference`** (attr_probe.py check: libniji vs the
  2.39 reference; most likely the nonnull on sched_setattr/getattr, which 2.39 does not declare). Then header_check
  (hc2.log), probe4 libniji vs 2.41 diff, conformance s13 (conf2.log). Read `~/logs/headers-stage2-gaps-h-test2.log`.

## Next leg: exact steps
1. `~/bin/jobwait headers-stage2-gaps-h-test2`; read its log tail. Run the s19 test alone for its message:
   `cargo test -q -p xtask diagnostic_attributes_match_the_reference` (jobrun test kind) and see how attr_probe.py
   treats names absent from the reference (spec/tools/attr_probe.py, diagattrs.tsv); fix by the table's own
   mechanism (an additions row, or an exclusion naming R3), never by loosening the check.
2. Fix any header_check / probe4 DIFF it shows (probe4 diff must be 0 lines; the names lines must be empty).
3. Copy the regenerated generated files back: `rsync -a keke:~/w/n1/libc-include/ <worktree>/libc-include/`
   (scp/rsync over ssh claude@ keke address), plus any other file headers-gen/abi-gen rewrote (git status shows);
   commit "Regenerate libc-include" (never hand-edit them).
4. Sync, then start plants2 (`cargo xtask plants handoff/HEADERS-STAGE2-GAPS-H-plants.txt --dir
   /dev/shm/claude/headers-stage2-gaps-h/plants-2 --timeout 600`) and pkg3: pkg.sh with NIJI_DEB_COMMIT=<new head>,
   SOURCE_DATE_EPOCH=<its commit time>, pkgs `inetutils util-linux` (pkg.sh writes `echo inetutils > pkgs`: make it
   take P). pkg3 must run alone on the role (no headers-gen/so beside it). Report a further stop (header, name, unit).
5. clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`), `cargo xtask static`,
   `cargo xtask precheck --jobs <labels> --head <sha>`, report json handoff/reports/HEADERS-STAGE2-GAPS-H.json,
   handback-check. Budget is tight: extend-budget once if only waits remain.

## Sweep (item 2) results (leg 1)
* `<arpa/ftp.h>`: only switch FTP_NAMES. `<arpa/telnet.h>` *_NAMES: 0 DIFF. tftp/inet/nameser: no switch defines
  anything. `<protocols/routed.h>` RIPCMDS -> `ripcmds` (5), `<protocols/timed.h>` TSPTYPES -> `tsptype` (25).
* GAP (report, not chased): libniji's `<protocols/routed.h>`, `<protocols/timed.h>`, `<protocols/rwhod.h>`,
  `<protocols/talkd.h>` lack their protocol names entirely (RIPVERSION, struct rip; TSPVERSION, struct tsp;
  WHODVERSION, struct whod; TALK_VERSION, NAME_SIZE, struct osockaddr ...). inetutils talk/talkd need talkd.h:
  likely the package build's next stop. Recommend one follow-up story for the four headers.

## Not to redo
The FTP tables' probes; the arpa/protocols sweep; telnet regression; the 2.41 probes above; the excalibur root.
