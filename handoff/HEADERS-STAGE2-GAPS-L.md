# HEADERS-STAGE2-GAPS-L: handoff (run 1, leg 2 -> leg 3)

Branch task/headers-stage2-gaps-l, cut from origin/next 6295b2301, no merge. Role keke:~/w/n1 (synced at leg 2's
code head). Job scripts on keke ~/work/headers-stage2-gaps-l/ (gen.sh, verify.sh, units.sh, units2.sh, fetch.sh,
p1.sh, p2.sh, p3.py, sweep.sh, pa.sh); local copies, cls.py, cmp.py, splice.py, uniontag.py in the story scratch
dir. Unit sources on keke ~/work/headers-stage2-gaps-l/src (gcc-14, systemd 257.13, elogind 255.17 orig tarballs).

## Done
* Leg 1 (commits 3756bc4c1, b87194d7e): math.h C++ visibility (raw -nostdinc++ splice), C2X C++ forms
  (math.h.d/190), PA_* enumerators and visibility, __environ, TTYDEF spellings, features.py kernel dir, header_probe
  `--raw-cxx`/`--configs`, s13.d/70-stage2-gaps-l.py checks.
* Leg 2 (6c04a9d26, c9d508ccf):
  - tgmath.h: `@@owns-macro` for the CMPLX family (complex.h defines them; `@@owns` left @@emit-c asking macros.tsv:
    headers-gen had 9 problems and wrote nothing).
  - headers/nested-tags.tsv: struct icmp / ih_idseq, ih_pmtu, ih_rtradv; struct ip6_hdr / ip6_hdrctl (stock C++
    does not see them at namespace scope). header_check nested_spelling follows dotted member paths (no `<anon>`).
  - xtask/src/headers/cond.rs union_tag(): no tag when visibility.tsv shows the typedef in a C++ config
    (La_x86_64_vector; pthread_attr_t keeps its tag). Checked on keke: C++ `union La_x86_64_vector *` rejected,
    typedef usable, C fine.
  - reserved.tsv __environ row lacked a tab (policy read as "emitlibc.so.6 ..." = omitted). xtask check_reserved()
    now refuses any policy but emit/emit-once/omit; unit test reserved_policy_text_is_checked (passes).
  - libc-include regenerated on keke and committed (headers-gen clean, 242 headers).
  - spec/abi/provenance.d/HEADERS-STAGE2-GAPS-L.txt (order 220) - committed in the leg-2 handoff commit.
* Unit 1 PASSED (job headers-stage2-gaps-l-units1): libstdc++ C99 math configure body (fpclassify..isunordered,
  volatile d1,d2,i) with `g++ -nostdinc++ -fno-exceptions` -std=c++98/c++11/gnu++98/gnu++17 against libniji, and
  installed libstdc++ <cmath> std::fpclassify/isnan/isunordered/signbit c++98..20. PA_* values static_assert ok.
  units1's units 2-4 failed for harness reasons only (gcc inner tarball not extracted; meson/autotools config.h
  missing; no autoreconf on keke) -> units2.sh.

## Running at leg end (read, do not restart)
* keke `headers-stage2-gaps-l-verify1` (~/logs/...-verify1.log): full header_check, features.py standalone,
  p1/p2/p3 libniji vs stock diffs (stock logs ~/logs/headers-stage2-gaps-l-{p1,p2ref,p3ref2}.log; expect only
  path/workdir noise), raw C++ sweep libniji side into /dev/shm/claude/headers-stage2-gaps-l/{sweep2,sweepgnu2}.
* keke `headers-stage2-gaps-l-units2` (~/logs/...-units2.log): wrapc.c as C++ with -include cmath, systemd/elogind
  stdio-util.h VA_FORMAT_ADVANCE TU with a stub config.h (SIZEOF_TIME_T etc., no header names), fakechroot setenv.c
  with empty config.h. A failure that is a header gap: report (header, name, unit), don't chase.

## Exact next steps (leg 3)
1. `~/bin/jobwait headers-stage2-gaps-l-verify1 headers-stage2-gaps-l-units2`; read the logs (tail/grep only).
2. Sweep: copy sweep2/visibility.tsv and sweepgnu2/visibility.tsv back (stock: sweep1/ref, sweepgnu/ref on keke,
   local copies sweep1/ref.tsv, sweepgnu/ref.tsv), run cls.py; CXX class must be only link.h La_x86_64_regs/retval
   (D-X.130 accepted difference). Anything else: fix or INFORMATIONAL.
3. Add to s13.d/70-stage2-gaps-l.py a C++ unit proving the nested tags are not at namespace scope: after
   `#include <netinet/ip_icmp.h>` and `<netinet/ip6.h>`, define `struct ih_idseq { char niji[3]; };` (and ip6_hdrctl,
   ih_pmtu, ih_rtradv) and assert sizeof == 3 (compiles only when the header nests them); in cxx11+ also
   `sizeof (decltype (icmp::icmp_hun)::ih_idseq) == 4`. Run header_check --only "GAPS-L" on keke.
4. Plants file handoff/HEADERS-STAGE2-GAPS-L-plants.txt (docs/plants.md format): cond.rs union_tag (drop
   `&& !cxx_typedef`) caught by `cargo xtask headers-gen --check`; check_reserved always Ok -> cargo test -p xtask
   reserved_policy; nested-tags.tsv row removal -> headers-gen --check / header_check --only GAPS-L; math.h.in
   classification guard -> header_check --only "GAPS-L C\+\+ math"; PA enum value -> --only "GAPS-L printf";
   features.py kernel dir; TTYDEF spelling. Run via `cargo xtask plants` (/dev/shm/claude/headers-stage2-gaps-l/
   plants-1, Zen 1 host).
5. `cargo xtask affected` (base origin/next) -> gate (or FULL: precheck + test jobs), clippy (foreground), static,
   precheck --jobs, report json handoff/reports/HEADERS-STAGE2-GAPS-L.json, handback-check.
