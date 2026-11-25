# HEADERS-STAGE2-GAPS-D: perl's gaps, IN_* class macros, sockaddr union member names, array parameters, __WALL, __timezone

**State:** branch `task/headers-stage2-gaps-d` (from origin/next ecc8c95e3), role keke:~/w/n1. Leg 1 ended in a REFRESH.
Decisions: none new; every change follows black-box probes of the reference (no differences row, no compat option).
Scratch probe scripts (not committed) on keke: `~/work/headers-stage2-gaps-d/{gp.py,aps.py,ac.sh}`; local copies in the
story scratch dir. `gp.py <include-dir> <workdir>`: stock vs libniji compile probes and a facts program (prints SAME/DIFF);
`aps.py <repo> [<include-dir>]`: the array-parameter sweep.

## Done in leg 1 (commits: see `git log origin/next..HEAD`)
* **perl (1)** `__priority_which_t`: reserved.tsv emit; `headers/sys/resource.h.in` owns it (enum `__priority_which` in
  C under `__NIJI_GNU`, int otherwise and in C++: probed) and owns getpriority/setpriority, declared with the typedef
  (the generator emits prototypes typedef-resolved, `int`, which conflicts with the enum in GNU C).
* **perl (2) / heimdal** `h_addr`: macro `h_addr_list[0]` in netdb.h (`@@owns`/`@@if`); visibility rows probed (netdb.h,
  misc + C++).
* **perl (3)** `struct ip_mreq_source`: layouts.tsv (12/4; imr_multiaddr 0, imr_interface 4, imr_sourceaddr 8),
  typedefs.tsv, visibility rows (10 headers, misc + C++).
* **perl d_bsd**: cause found: perl's Configure greps the TEXT of /usr/include/signal.h and sys/signal.h for the string
  `SIGTSTP`; libniji's signal.h defines it in that file. d_bsd only reaches `$Config{d_bsd}`; nothing in perl reads it.
  Not changed: RULING NEEDED (recommend no change; matching needs signal numbers moved to another file).
* **glib** IN_CLASSA/B/C/D, IN_MULTICAST, IN_EXPERIMENTAL, IN_BADCLASS: netinet/in.h template; semantics from a
  facts probe (all 256 top bytes, low bits 0 and all ones, -1, a 33-bit value, evaluation count 1, type int in C,
  not int in C++ (a comparison), not usable in `#if`); visibility rows in 10 headers, every configuration.
* **fakechroot** union members: layouts.tsv `__SOCKADDR_ARG`/`__CONST_SOCKADDR_ARG` now the 13 members the reference's
  union has (`__sockaddr__`, `__sockaddr_at__`, ... `__sockaddr_x25__`; probed by name and pointer type); were 5 `__tu_*`.
* **gperftools** `__WALL` (int 0x40000000), `__WCLONE` (unsigned int 0x80000000), `__WNOTHREAD` (int 0x20000000):
  macros.tsv + reserved emit + visibility rows (sys/wait.h, wait.h, stdlib.h, math.h, complex.h, tgmath.h).
* **icu** `__timezone`/`__daylight`/`__tzname`: exported by libniji (versions.tsv, GLOBAL objects; probe: same objects
  as timezone/daylight/tzname); reserved emit; visibility rows unchanged (time.h and 8 others).
* **gdb** pr* types: no difference at this head (sys/procfs.h, thread_db.h, proc_service.h, C and C++, and autoconf's
  usability test). The stage-2 log shows gdb's configure judged thread_db.h/proc_service.h unusable on 25.1, so gdb
  used its own copy; cause on 25.1 not reproduced (the 25.1 deb tree was not probed with the right include order).
* **Array parameters**: generator (`xtask/src/headers/functions.rs` `array_params`) takes `[N]` as well as a typedef;
  `headers/arrayparams.tsv` +29 rows from the sweep (aps.py: 34 functions on the reference, 33 match after the change;
  `regexec` has a variable-length bound, not matched: INFORMATIONAL).
* header_probe.py `package_names()` lists the new names; tag `ip_mreq_source`.

## Results so far
* gp.py after the change (job headers-stage2-gaps-d-gen2, before the getpriority/IN_* fix): 1293 SAME, 17 DIFF (the two
  fixed since: IN_* C++ type, getpriority in GNU C). facts: SAME. gen3 (regen + gp + header_check) was running at refresh.

## Exact next step
1. Read `~/logs/headers-stage2-gaps-d-gen3.log` on keke: gp TOTAL must be diff=0, header_check 0 failed; fix any.
2. Commit the generated libc-include (`cargo xtask headers-gen` output) separately; push.
3. Tests: header_check rows (s12.py: IN_* values, union member names, ip_mreq_source offsets, h_addr; s07.py: __WALL,
   __priority_which_t redeclaration in C/C++, utimensat/futimens array form with -Werror=array-parameter; time.h
   __timezone in a module that fits). Plants file handoff/HEADERS-STAGE2-GAPS-D-plants.txt; `cargo xtask plants`.
4. Proof jobs: build system debs (copy 1125B's debs.sh method into ~/work/headers-stage2-gaps-d/), run ystage2 niji perl
   (mei's ~/work/RELEASE-SNAPSHOT-1125B/ystage2.sh method); compile fakechroot bind.c, faketime libfaketime.c
   (-Werror=array-parameter), glib ginetaddress.c, icu putilimp.h users against the tree.
5. clippy + static + precheck; report json; handback-check.
