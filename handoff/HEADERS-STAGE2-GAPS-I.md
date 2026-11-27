# HEADERS-STAGE2-GAPS-I: handoff (run 1, leg 3: hand-back)

Branch task/headers-stage2-gaps-i (stacks on HEADERS-STAGE2-GAPS-H 012838ad7; no merge done). HEAD: see the report
json. Role shiki:~/w/sh1 (stock Ubuntu 2.39, gcc 13). Unit proofs on mei `~/work/headers-stage2-gaps-i/` (units.sh,
fixincl_check.sh, uuid*.sh; scratch copies in `~/w/.scratch/HEADERS-STAGE2-GAPS-I/`). Decision: D-X.178
(records/decisions/D-X/D-X.178.toml). Plants file: handoff/HEADERS-STAGE2-GAPS-I-plants.txt.

## State per brief item
* 1 fixincludes: va_list in stdio.h, wchar.h through `__need___va_list` + `<stdarg.h>` + `typedef __gnuc_va_list
  va_list`, with stock's `_VA_LIST_DEFINED` guard-and-define form (probe: stock defines it after either header and a
  program's earlier definition suppresses the typedef); printf.h includes the whole `<stdarg.h>` (stock: va_start and
  va_arg usable after printf.h alone). size_t/ptrdiff_t/wchar_t via `__need_*` + `<stddef.h>` in stdlib.h, unistd.h,
  sys/types.h (typedefs.rs COMPILER_STDDEF_HEADERS). **mei units5: fixincl_check 243 headers, 0 rewritten**; va_list,
  size_t, libgomp gcn, libstdc++ C99 stdio (C++98/11) units ok with gcc (gcc-14 ginclude) and clang-19.
* langinfo (coordinator note): the enumerators/macros were matched in leg 1. libstdc++ config/locale/gnu: 7 of 8 .cc compile
  with gcc and clang. ctype_members.cc needed `_ISbit`, which is now added (probe-settled values). c_locale.cc fails
  identically against the host's own libc (host libstdc++ 13 headers with gcc-14 sources): harness, not libniji.
* 2: comparison_fn_t (systemd sort-util.h unit ok with _GNU_SOURCE, as systemd builds), struct termio (sys/ioctl.h
  and termio.h), glob64_t (layout + callbacks typed per configuration via fnmembers.tsv `GNU ;; OTHER` form),
  struct igmp: units ok.
* 3 opaque sweep (leg 2): glob64_t, struct igmp, struct aiocb64 and fpos64_t now have members, as in the reference.
* 5 protocols/*: talkd, rwhod, timed, routed names and layouts (leg 2). A real inetutils build is NOT VERIFIED.
* 6 __USE_TIME_BITS64: defined in every configuration (differences row, D-X.178). Proof: util-linux 2.41.5
  uuid_time.c against our tree, linked with libuuid.sym, exports `uuid_time64@@UUID_2.40` (stock 2.39:
  `uuid_time@@UUID_1.0` only).

## Further gaps (reported, not chased)
* `<termios.h>` + `<sys/ioctl.h>` in one unit: TTYDEF_SPEED and TIOCSER_TEMT are redefined (stock is clean). This is pre-existing.
* timed.h TSPTYPES/tsptype and routed.h RIPCMDS/ripcmds tables (program-switched, like FTP_NAMES) are not added.
* These real units were not compiled: fakechroot glob64.c and the llvm-19 sanitizer .cpp (leg-2 refusal), nftables igmp (no
  source), and the inetutils talkd build.

## Not to redo
The fixincludes, enum, 2.41, opaque, va_list-form and _ISbit probes; tables1, tp2; units1-5; uuid1-2.

## Exact next step
None for this story beyond the train. If the plants or test job named in the report went red: fix what it names, then
rerun only that job.
