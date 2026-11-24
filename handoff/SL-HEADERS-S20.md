# SL-HEADERS-S20: slice 20, switch and package (handoff, run 2 = SL-HEADERS-S20-2, leg 2)

**State.** Branch `task/sl-headers-s20-2` (run 1's `task/sl-headers-s20` + run 2), base origin/next a0d93596e.
Code head 4b30fac10 (this note and the report are committed on top). Build role keke:~/w/n1. Say "slice 20" in commits.
`cargo xtask affected --list --base origin/next` escalates to FULL (headers/*.h.in and reserved.tsv are generator
inputs): the gate is the train's; this story proves itself with precheck + the `test` job (brief, step 4).

**Decisions in force.** D-X.168 (drivers default to libniji's headers; coexist sysroot `/usr/lib/niji/usr/include`
+ `x86_64-linux-gnu`, ruling #8413; now also records the unshipped headers as a known limitation, coordinator
ruling on #8419). D-X.169 (`__locale_t`, `__CORRECT_ISO_CPP_{STRING,WCHAR}_H_PROTO`, ruling #8412).

**Done.** Run 1: drivers, 236 manifest rows, P4 tool, `TESTS:` reason for niji_cc_headers.rs (commit 90ca3dad3).
Leg 1 (6228e4df5): P4 218 FAIL -> 4 FAIL, by cause: `__locale_t` undeclared (most of the 218: every
`<iostream>`/`<locale>`/`<sstream>` user, all -std), `<cwchar>`/`<cstring>` const-overload collisions; see
handoff/SL-HEADERS-S20-report.md for the counts.
Leg 2 (287819e37, b969dc875):
* The 4 remaining FAILs (c++98/c++03 `<complex.h>`, `<tgmath.h>`) are the C++ library's `#include_next` of a C header
  libniji does not ship. The spec tables name 7 headers with no file in the staged tree: aio.h, complex.h,
  mqueue.h, proc_service.h, tgmath.h, thread_db.h, Fortran math-vector-fortran.h. Every C compile of one fails
  through niji-cc (checked). Ruling (#8419, option B): land with them as a known limitation; the coordinator
  briefs SL-HEADERS-COMPLEX-TGMATH separately. `validation/headers_unshipped.tsv` lists them; P4 holds it equal to
  what is missing (a ratchet), counts those C++ cases "known", and compares callsites/noexcept without their rows.
  Release notes: docs/release-notes.d/SL-HEADERS-S20.md (changes) and -2.md (known limitations); install-packages.md.
* callsite_probe `--include` stopped on aio.h c89 because, without NIJI_UAPI, it put the host's /usr/include after
  libniji's tree and compiled the host's aio.h against libniji's cdefs. In --include mode it now uses $NIJI_UAPI or
  the `uapi` beside DIR, never the host's, and skips (and names) headers absent from DIR; P4 passes NIJI_UAPI.
* Plant s20-cc-nostdinc SURVIVED in the leg-1 plants run: the test's `-nostdinc` case compiled s.c, which fails on
  libniji's headers anyway. It now compiles p.c (compiles on libniji's headers only). 2 new plants on P4.

**Leg 2, later.** d22bb006b: callsite_probe's C++98 redeclaration units no longer get libniji's tree as -isystem
(g++ then accepts any exception specification: the cxx98 column read "unknown" for every function). 4b30fac10: the
C++ driver test names __locale_t right after <locale.h> (plant s20-cxx-locale_t had SURVIVED). Rulings #8419 (B:
unshipped headers a known limitation) and #8420 (A: P4 part 2's remaining rows go to a tracked follow-up; S20 lands
with P4's C++ part clean). Part 2 table by header: handoff/SL-HEADERS-S20-report.md.

**Jobs (keke), final code head 4b30fac10.** `sl-headers-s20-2-t6` (test: so, niji_cc_headers, P4 --no-noexcept: part 2
is the follow-up by ruling #8420), `sl-headers-s20-2-t5` (the same with part 2: C++ part 0 FAIL, part 2 red: the table) and `sl-headers-s20-2-plants3` (11 plants). Superseded: `-plants` (leg 1,
killed), `-t3`, `-t4`, `-plants2` (older heads; plants2: 10 CAUGHT, s20-cxx-locale_t SURVIVED, fixed since).

**Next step.** Hand-back done when plants3 is all CAUGHT. Next stories: the P4 part 2 follow-up (math.h __ names,
stdbit.h exception specifications, the rest of the table), SL-HEADERS-COMPLEX-TGMATH, the other unshipped headers,
P6 (`sh conformance/run.sh --niji-headers`), P7 (smoke with `--niji-headers`), a .deb built with the new rows.

**Plan rows to read.** plan/headers.md section 3 (P4), slice 20 row; D-X.107, D-X.113, D-X.168, D-X.169, D-10.125.
