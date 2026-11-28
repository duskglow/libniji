# HEADERS-STAGE2-GAPS-N: handoff (run 1)

Branch task/headers-stage2-gaps-n, cut from origin/next 6d0c3e0b6, no merge. Role mei:~/w/s1. Job scripts in
mei:~/work/headers-stage2-gaps-n/ (godiff.py sweep, ordercls.py / selforder.py classifiers, libgo.sh + libgocmp.py,
p34.sh probe, units.sh, proof.sh); sources there under src/ (gcc 14.2.0 godump.cc and libgo, compiler-rt 19.1.7,
systemd 257, elogind 255, copied from earlier runs' mirror downloads). Plants: handoff/HEADERS-STAGE2-GAPS-N-plants.txt.

## What the branch does
1. headers-gen (`Gen::definitions_first`): a header's emitted items keep their sorted order except that each macro
   comes after the macros of the same list that its definition names (a dry run of `macro_def`, read for
   identifiers; a cycle keeps the sorted order). gcc's -fdump-go-spec keeps a macro only when every name in its body
   is already a macro (gcc/godump.cc `go_define`), and libgo's mksysinfo.sh copies every `const` of that dump.
2. `<pthread.h>`: in C++ without exceptions (the `__sigsetjmp` form of the cleanup macros) the template declares
   `__pthread_register_cancel`, `__pthread_unregister_cancel`, `__pthread_unwind_next` and, under `__NIJI_GNU`, the
   `_defer` / `_restore` pair, under the generator's own guards. Stock: all five in C++98..23 with -fno-exceptions,
   none with exceptions (probe).
3. `<sys/quota.h>` `QCMD` (template, the kernel UAPI `<linux/quota.h>` form), visibility row `!time64only` (stock:
   every configuration, C and C++).
4. `__cpu_mask`: reserved.tsv `emit` (visibility.tsv already listed sched.h, pthread.h, spawn.h, thread_db.h).
5. header_check fragment `spec/tools/header_checks/s13.d/90-stage2-gaps-n.py`: units (QCMD, __cpu_mask) and
   `problems()`: the macro-order sweep (`gcc -E -dD`, every public header alone, six feature-macro sets, macros
   defined in the C library's own files only: the kernel UAPI text is the same for both), the report's
   -fdump-go-spec repro, the C++ -fno-exceptions checks. `--reference` passes on stock except the order sweep, which
   is skipped there: stock's own headers have 132 such cases (IP_RECVRETOPTS before IP_RETOPTS, SSIZE_MAX before
   LONG_MAX, NL_*MAX before INT_MAX, DT_PROCNUM, RE_SYNTAX_ED/SED, AT_HANDLE_FID), libniji has 0.

## Proof (see the report json for the job labels)
* Go-dump sweep, libniji vs stock 2.39 (153 headers x 6 sets): order-only drops 152 names / 7392 rows -> 0.
* libgo sysinfo.c (Yohane's flags): `_AF_INET`, `_AF_PACKET`, `_ENOTSUP` present, as stock.
* p34 probe (items 2-4, C and C++, both trees) 0 DIFF; units (tsan clang++-19 and g++, quota-util.c, test-sizeof.c)
  compile on stock and both libniji trees and fail on the pre-change headers.

## Not fixed (other classes, reported INFORMATIONAL)
* Names missing from libniji's headers altogether (stock has them, libgo's dump copies them, no libgo error yet):
  `<netinet/in.h>` IPPORT_* (25 enumerators), `<net/route.h>` RTCF_*, RTMSG_*, RT_CLASS_*, `<netinet/icmp6.h>`
  ICMPV6_EXT_ECHO_*, ND_OPT_RTR_ADV_INTERVAL, `<netinet/ip.h>` IPTOS_DSCP_VA; sweep `nameonly` 547 names in all.
* Consts that differ by form only (stock's hex/expression spellings, libniji's decimal; casts godump cannot read) and
  libniji's own `__NIJI_*` macros in the dump (1490 in sysinfo.c's); stock's internal `_BITS_*` / `__HAVE_*`.
* `sizeof_*` of reserved or kernel-shared struct types godump sees in stock only (`__FILE`, `__ptrace_*`,
  `group_filter`, `ip_opts`, `prctl_mm_map`, ...).

## Next step
Land through the train if the jobs are green. A follow-up story for the missing-name class above.
