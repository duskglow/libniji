# HEADERS-STAGE2-GAPS-I: handoff (run 1, end of leg 2)

Branch task/headers-stage2-gaps-i (stacks on HEADERS-STAGE2-GAPS-H 012838ad7; no merge done). Role shiki:~/w/sh1
(stock Ubuntu 2.39, gcc 13, no clang). Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-I/` (merge_*.py, gen.sh,
hc.sh, units.sh, p/ probes; src-dl/ consumer sources); shiki `~/work/headers-stage2-gaps-i/` (gen.sh = headers-gen +
pack libc-include and include into libc-include.tgz; hc.sh = header_check + `cargo test -p xtask headers`); mei
`~/work/headers-stage2-gaps-i/` (units.sh, fixincl_check.sh, li.tgz, sort-util.h). Decision id: D-X.178 (used).

## Done (leg 1 + leg 2)
* Leg 1 (commit b59d22531): item 1 va_list/size_t/wchar_t via `__need_*`; langinfo enumerators; __USE_TIME_BITS64.
* Leg 2 commit 1f5ca6c5f: reference rows merged for the new names only (layouts 134 rows/14 types, visibility 110,
  typedefs 9, macros 59); glob_t/glob64_t callbacks typed (fnmembers.tsv + macros in glob.h.in: BROKEN, see below);
  timed.h.in (tsp_time, tsp_hopcnt, ANYADDR) and routed.h.in (rip_nets, rip_tracefile) member-name macros;
  libc-include regenerated; header_check fragment `spec/tools/header_checks/s13.d/40-stage2-gaps-i.py`.
* Leg 2 WIP commit (this one): typedefs.rs wraps the `__need_*` + `<stddef.h>` in `#if !defined
  __STDC_VERSION_STDDEF_H__` (fixes P9's 64 FAILs "after gnu2x": gcc 13's stddef.h re-entered for one type in C2X
  redefines that macro; stock fails the same units identically, probe4); use-macros.tsv time32 row = 1 (fixes FAIL
  FEATURES use time32; matches the differences row "every configuration"); the termio use unit includes only
  <sys/ioctl.h>; validation/fixincl_check.sh (new). libc-include NOT regenerated for the typedefs.rs change yet.
* Probe-settled (stock 2.39, black box): glob callbacks are typed (struct dirent/stat, dirent64/stat64) with
  _GNU_SOURCE only, `void *` in default/c99/posix/lfs64; ANYADDR is NULL (C `void *`, C++ `long` = __null) and timed.h
  shows no NULL; whoent is C-only (C++ nests it); termio visible via sys/ioctl.h, termio.h, pty.h, sgtty.h, ...
* mei job units1 (tree of 1f5ca6c5f): **fixincl_check: 243 headers, 0 rewritten**; va_stdarg, va_stdio_only,
  va_wchar, size_t, stdarg_first, gomp_gcn, c99_stdio (C++98/11), termio, igmp, glob64 all ok with gcc (gcc-14
  ginclude) and clang-19. Stub faults (mine): sort-util.h needs `typedef int (*comparison_userdata_fn_t)(const void *,
  const void *, void *);` in the stub forward.h; c99_stdio's `int main ()` -> `int main (void)` (clang -pedantic-errors).
  libstdc++ locale extraction failed (orig tarball layout): NOT VERIFIED, list `tar -tzf` first or report.

## Exact next steps
1. Fix the glob callbacks (xtask test every_table_row_is_emitted_and_matches_the_tables compiles struct bodies
   standalone, so template macros are undefined there): drop the `__NIJI_GLOB_*` macros from glob.h.in (keep the
   `#ifdef __USE_GNU` incomplete-tag forward declarations `struct dirent; struct dirent64; struct stat; struct
   stat64;`), and give fnmembers.tsv an alternative form, e.g. declarator `GNU-DECL ;; OTHER-DECL`, that
   aggregates.rs `member()` (line ~172) emits as `# ifdef __USE_GNU` / GNU-DECL; / `# else` / OTHER-DECL; / `# endif`,
   skipping the dependency scan for that form (it must not pull in complete struct dirent/stat). Document the form in
   the fnmembers.tsv header comment.
2. `cargo xtask stubs` (xtask test headers_gen_regenerates_the_abi_layouts: "spec clause/symbol `abi_glob64_t` has
   neither a stub nor a test", also abi_struct_igmp and the other new layout types); commit the regenerated files.
3. Sync (check rc: 4 = BUSY), shiki job: gen.sh then hc.sh (`sh -c "sh gen.sh && sh hc.sh"`); fetch libc-include
   (tar of libc-include.tgz, extract `libc-include` only); header_check must be 0 FAIL and xtask headers tests green.
4. mei: new li.tgz, fix the two stub faults in units.sh, rerun (label units2); expect fixincl 0 rewritten.
5. Plants (docs/plants.md form, file handoff/HEADERS-STAGE2-GAPS-I-plants.txt, `cargo xtask plants`): e.g. restore
   `_VA_LIST_DEFINED` guard in stdio.h.in (fixincl/P9 or unit), drop a layouts row of glob64_t (offset check), wrong
   fnmembers glob type (callbacks check), remove the `@@if tsp_time` block (use unit), the time32 row back to `-`.
6. clippy, static, affected gate, precheck, report json, handback-check; JOBS: shiki gate/plants, mei:units2.

## Further gaps found (report, do not chase)
* `<termios.h>` + `<sys/ioctl.h>` in one unit: TTYDEF_SPEED redefined (sys/ioctl.h `13`, termios.h `B9600`) and
  TIOCSER_TEMT redefined (libniji `1`, kernel asm-generic/ioctls.h `0x01`); stock compiles both orders cleanly
  (probe4). Pre-existing; llvm-19's sanitizer includes both (unit NOT compiled for real: see refusal).
* TSPTYPES/tsptype and RIPCMDS/ripcmds tables (program-switched, like FTP_NAMES): not probed, not added.
* Real-source units NOT compiled: fakechroot glob64.c and llvm sanitizer .cpp (refusal), nftables igmp (no source),
  util-linux libuuid uuid_time64 (item 6 proof: not done yet; util-linux 2.40.2 tarball reachable from mei).

## Refusal (report in hand-back)
`cd ~/w/.scratch/HEADERS-STAGE2-GAPS-I/src-dl; ls; grep -n "glob64_t\|gl_\|#include" glob64.c | head -30; grep -n
"glob_t\|gl_closedir\|gl_readdir\|struct_glob\|sizeof (glob" sanitizer_platform_limits_posix.cpp | head` (auto-mode
classifier). Step stopped; the glob types were settled by black-box probe instead. Rule slip: one `sed -i` on a
comment line of headers/fnmembers.tsv (Sources line); correct content, Edit only since.

## Not to redo
The fixincludes probe, enum probe, 2.41 __USE_TIME_BITS64 probe, opaque sweep, tables1 (layouts + full header_probe),
tp2 type_probe, probe2/3/4.
