# HEADERS-STAGE2-GAPS-L: handoff (run 1, leg 3: report)

Branch task/headers-stage2-gaps-l, cut from origin/next 6295b2301, no merge. Role keke:~/w/n1. Job scripts on keke
~/work/headers-stage2-gaps-l/ (verify.sh, units*.sh, nt.sh, stockunits.py, p1.sh, p2.sh, p3.py); local copies and
cls.py (sweep classifier) in the story scratch dir. Plants: handoff/HEADERS-STAGE2-GAPS-L-plants.txt.

## What the branch does
1. `<math.h>` in C++: visibility.tsv's C++ columns of math.h/tgmath.h come from the header alone (`header_probe.py
   --raw-cxx`, -nostdinc++), as libstdc++'s configure and `#include_next` see it: fpclassify .. isunordered and the
   C2X macros are now macros in C++ (C++98..23); math.h no longer pulls <stdlib.h> in C++ (the header alone of the
   reference does not; the C++ library's wrappers add it). issignaling/iszero/iscanonical/iseqsig are C++ functions
   in the reference's form (math.h.d/190). tgmath.h `@@owns-macro` for complex.h's CMPLX family.
2. Sweep of the class (raw C++ C++98/11/17/23 and gnu, every public header, libniji vs stock): C++-specific
   differences left: link.h La_x86_64_regs / La_x86_64_retval only (the accepted D-X.130 difference). The other 3700
   raw rows differ in C (gnu) as well (feature-macro names, typedefs; not C++-specific) and are the C comparison's
   (header_check passes it with its reviewed lists). Rows added to header_check: GAPS-L units (76 rows).
3. `<printf.h>` PA_INT .. PA_LAST: unnamed int enumeration, values 0..8, combinations with PA_FLAG_* as stock.
4. `<unistd.h>` `__environ` declared in every configuration (reserved.tsv row fixed: a missing tab read as an unknown
   policy, now refused by xtask check_reserved()).
5. Carried: (a) #8809 features.py standalone gets header_check's kernel UAPI dir (kernel_flags); (b) #8810
   TTYDEF_{C,I,L,O}FLAG spelled with the termios flag names (integer constants only where stock's are).
6. C++ nested tags: struct icmp's ih_idseq/ih_pmtu/ih_rtradv and struct ip6_hdr's ip6_hdrctl (nested-tags.tsv);
   La_x86_64_vector union unnamed in C++ (cond.rs union_tag).

## Proof (leg 3, keke)
* header_check whole: 0 failed of 26915 (job headers-stage2-gaps-l-nt); --only GAPS-L 0 of 76 on the final tree.
* Stock 2.39 accepts the new C++ units (nested tags, TS forms): 24 compiles, 0 failed (stockunits.py).
* p1 (PA_* values/types) and p3 (C2X C++ forms and values) libniji vs stock: 0 DIFF (the verify1 diffs were harness:
  -nostdinc without the C++ library dirs; the values program not run on the libniji side).
* Units: libstdc++ C99 math configure bodies c++98..20 (units1); systemd 257 / elogind 255 stdio-util.h users;
  fakechroot setenv.c with and without _GNU_SOURCE (units2); gcc-14 gm2-libs-ch/wrapc.c as C++ with <cmath> first,
  C++98..20, libniji and stock alike (units3c; config/system/gm2-libs-host stubs: HAVE_SIGNBITL/F unset, as no
  C library has those functions).
* features.py --only "FEATURES pkg npth rwlock": 0 failed standalone.

## Jobs (final head)
gate headers-stage2-gaps-l-gate, plants headers-stage2-gaps-l-plants; results in the report.

## Not to redo
The sweep (local sweep2/, sweepgnu2/, cls2.tsv), the units, the p1/p3 comparisons.

## Next step
Land (train). Nothing open in this story.
