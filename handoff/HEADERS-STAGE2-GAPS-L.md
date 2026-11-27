# HEADERS-STAGE2-GAPS-L: handoff (run 1, leg 1 -> leg 2)

Branch task/headers-stage2-gaps-l, cut from origin/next 6295b2301, no merge. Role keke:~/w/n1. Job scripts:
keke ~/work/headers-stage2-gaps-l/ (sweep.sh, p1.sh, p2.sh, p3.py, pa.sh, pa-names.txt); local copies and outputs in
the story scratch dir (sweep1/, sweepgnu/, cmp.py, cls.py, splice.py, vis.py). libc-include NOT regenerated yet.

## Done (committed; the second commit adds PA_* visibility rows and the tgmath.h @@owns line)
* header_probe.py: `--raw-cxx` (C++ compiles get -nostdinc++) and `--configs IDS`.
* Item 1 cause: visibility.tsv's C++ columns were probed through libstdc++'s <math.h>/<cmath> wrappers (they undefine
  the macros and add <stdlib.h>); libniji is compiled -nostdinc (raw) everywhere. math.h and tgmath.h C++ columns spliced
  from a raw stock run (splice.py: changed 510, removed 1464, added 130 rows). math.h.in no longer includes <stdlib.h> in
  C++. providers.tsv: 6 stale `generic` rows removed (fpclassify isfinite isgreater isnormal isunordered signbit).
  New math.h.d/190-cxx-c2x-functions.frag: C++ issignaling/iscanonical (overloads f, d, ld, _Float128 on g++>=13; int),
  iszero (template, bool), iseqsig (template over both types, int, noexcept; trait rejects __float128) - the stock form
  probed by p3.py (all identical C++98..23). reserved.tsv: __iscanonicall emit (C macro and C++ overload call it; it
  was never declared), __environ emit.
* Item 2 sweep (stock vs libniji, raw C++, cxx98..23, plus gnu to separate C-shared rows): C++-specific class =
  math.h/tgmath.h (fixed above) + link.h La_x86_64_regs/retval (D-X.130, accepted) + link.h La_x86_64_vector (stock
  shows in C++, libniji not) + nested tags netinet/ip6.h ip6_hdrctl, netinet/ip_icmp.h ih_idseq/ih_pmtu/ih_rtradv
  (libniji shows them at file scope in C++; stock nests them). The last two groups are NOT fixed yet.
* Item 3: enumerators.tsv `anon PA_INT` rows 0..8 (stock probe: unnamed enum, int, values 0..8; PA_INT|PA_FLAG_LONG=512);
  visibility rows printf.h !time64only (header_probe --names on stock).
* Item 4: __environ declared in every configuration on stock (p1); reserved.tsv emit (rows existed).
* Item 5a: header_check.kernel_dir/kernel_flags; features._compile uses them (standalone = harness).
* Item 5b: macro-spellings.tsv TTYDEF_{C,I,L,O}FLAG spelled with the termios flag names (value-derived; OFLAG uses
  XTABS: stock p2 matrix shows OFLAG/LFLAG not constant in pty.h/termio.h strict modes, all four not constant with
  sys/ioctl.h, sgtty.h, sys/ttydefaults.h alone).
* Tests: spec/tools/header_checks/s13.d/70-stage2-gaps-l.py (math C++ configure tests + macros, PA values/enum,
  __environ, TTYDEF matrix; p1_skip for TTYDEF in the TTY headers).

## Exact next steps (leg 2)
1. Sync; on keke `cargo xtask headers-gen` (pa.sh's run before the tgmath @@owns fix had 9 problems, all tgmath.h
   CMPLX*/_Complex_I "no macros.tsv value"). Check printf.h gets the anonymous PA enum (may need `@@enum` in
   printf.h.in). Copy the regenerated libc-include back into the worktree and commit it (GAPS-K lesson).
2. header_check on keke for the touched headers (math.h tgmath.h printf.h unistd.h + TTY headers, GAPS-L units),
   `python3 spec/tools/header_checks/features.py --only "FEATURES pkg npth rwlock gnu"` standalone.
3. libniji-side probes: p1.sh/p2.sh/p3.py with `~/w/n1/libc-include` as include arg; compare with the stock logs
   (~/logs/headers-stage2-gaps-l-{p1,p2ref,p3ref2}.log). Rerun sweep.sh (cxx) and cls.py: CXX class must be only
   link.h + the nested tags unless fixed.
4. Nested tags: look at headers/nested-tags.tsv (C++ scoping of ih_idseq etc.); La_x86_64_vector in C++. Fix if cheap,
   else INFORMATIONAL.
5. Unit proofs (keke jobs): libstdc++ configure tests (`g++ -nostdinc++ -std=c++98/-std=c++11 -fno-exceptions`,
   Yohane's repro), gcc-14 gm2-libs-ch/wrapc.c (source from the Debian mirror the snapshot scripts use, see
   ~/work/RELEASE-SNAPSHOT-1127B/*.sh), systemd 257/elogind 255 src/basic/stdio-util.h users, fakechroot setenv.c
   (~/work/RELEASE-SNAPSHOT-1127B/u2/pkgs/fakechroot).
6. spec/abi/PROVENANCE note (visibility.tsv splice + PA rows, enumerators.tsv PA rows, macro-spellings TTYDEF).
   Plants (handoff/HEADERS-STAGE2-GAPS-L-plants.txt via cargo xtask plants), gate (cargo xtask affected --run, or
   FULL: precheck + tests), clippy/static, precheck, report json, handback-check.
