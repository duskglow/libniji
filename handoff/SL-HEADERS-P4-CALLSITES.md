# SL-HEADERS-P4-CALLSITES (handoff and report, run 1, leg 2)

**State.** Branch `task/sl-headers-p4-callsites` (from origin/next 44304e709). Code head fdf52784e (5dff4e3e8 generator
+ regenerated headers, fdf52784e accept list + plants); this note and the report json on top. Role keke:~/w/n4.
Jobs: `sl-headers-p4-callsites-gate` (direct tier: `cargo xtask affected --run --base origin/next`),
`sl-headers-p4-callsites-xtask-headers` (`cargo test -p xtask --bin xtask headers`: the rows test and the generator's
tests, which `affected` does not list), `sl-headers-p4-callsites-plants-2` (handoff/SL-HEADERS-P4-CALLSITES-plants.txt).
**Results.** Gate rc 0: 6 suites, 695 passed, 0 failed, peak 1.8G. xtask headers: 26 passed, 0 failed (rows test
included). Plants-2: 6 CAUGHT, no WARN; its baselines are P4 part 2 (--no-cxx) and part 1 (--no-noexcept) green on
the code head. clippy (workspace, all targets, all features) and static clean. Superseded: `-p4run1` (math.h failed
in C++11+, fixed by 5dff4e3e8), `-p4run2` (rc 1: the 43 lines before the accept list), `-plants` (stopped; its
reserved.tsv emit->omit plant SURVIVED by design, see Plants).

**Ruling in force.** #8468 (challenge accepted): P4 part 2 follows D-X.107 (R-H2/R-H3) and the headers/reserved.tsv
exclusion P2/P3 use; the math.h `__` names stay omitted; emit stdio_ext.h's documented `__f*` names. D-X.158 (-O2
inline design), D-X.131 (slice 17 accepts), the pidfd C-linkage differences row.

**Triage (P4 part 2, baseline 44304e709 -> code head; lines are probe_compare lines).**

| Header | Cause | Before | After |
|---|---|---|---|
| math.h | `__` names, policy-omitted (#8468, D-X.107) | 716 names x 3 tables | excluded (reported as "left out by policy") |
| resolv.h 33, argp.h 9+2, argz.h 5, error.h 4, string.h 5, ctype.h 2, libintl.h 2, stdio.h 1, signal.h 1+bsd_signal, sys/shm.h, unistd.h, sys/gmon.h, nss.h, libgen.h 1 each, bindresvport6, getumask | `__` names or reserved.tsv omit rows (D-X.117, D-X.124) | as listed | excluded |
| stdio_ext.h | 9 documented `__f*` names missing (spec 12.4) | 9 | 0 (reserved.tsv emit rows) |
| stdbit.h | 70 exception specs missing | 70 | 0 (`__NIJI_NOTHROW` in stdbit.h.in) |
| math.h | 34 C99 functions without C++98 `throw()` | 34 | 0 (headers.rs nothrow: cxx17 noexcept OR cxx98 throw()) |
| math.h | isinf/isnan functions not declared (C) | 2 | 0 (`@@when` + `@@owns-macro`) |
| math.h (P4 part 1) | isinf/isnan functions declared in C++11+ (conflicts with `<cmath>`) | 0 -> 5 stds FAIL in run 1 | 0 (cond.rs NOT_CXX11, probe below) |
| alloca.h | alloca function declaration missing | 1 | 0 (`@@owns-macro`, providers.tsv) |
| ctype.h | 12 `is*_l`: reference inline at -O2, libniji calls the export | 24 lines | accepted, follow-up |
| libintl.h | gettext/dgettext/ngettext/dngettext: reference calls dcgettext/dcngettext at -O2 | 8 lines | accepted, follow-up |
| netrose/rose.h | `__cmsg_nxthdr` inline at -O2 on the reference | 2 lines | accepted, follow-up |
| sys/pidfd.h | C++ linkage: mangled names no library exports | 8 lines | accepted: existing differences row |
| stdlib.h | div cxx17 `error` vs `noexcept` | 1 line | accepted: probe artefact (probes below) |

After: P4 part 1 0 FAIL in c++98..c++23 (c++26 unsupported by GCC 13); part 2 43 lines, all in
validation/headers_p4.accept under their reasons, 0 other differences (p4run2 on 5dff4e3e8's code).

**Probe-settled (black box, Zen 1 host, reference Ubuntu 24.04 glibc 2.39).** (1) The reference's own math.h under
-nostdinc declares `int isinf (double)` and `int isnan (double)` in C++98/C++03, not in C++11..C++23 (with and
without _GNU_SOURCE, _DEFAULT_SOURCE, _XOPEN_SOURCE=700); C gnu17 yes, c99 no. (2) stdlib.h div in C++ (C++ library
dirs, both header sets): `&div` an overload set on both, `div(int,int)` div_t, `div(10L,3L)` ldiv_t, the noexcept of
div(int), div(long), div(long long), ldiv, lldiv, abs(int), abs(long), labs all equal: the noexcept.tsv row is a
probe artefact. Probe scripts: scratch only (`~/work/sl-headers-p4-callsites/*_probe*.sh` on the role host).

**Plants (brief step 6).** The brief's "one math.h `__` declaration dropped" has no site: ruling #8468 keeps them
omitted. Five plants instead: `__fpending` emit row -> omit; math.h `@@when isinf` -> `@@if` (the function is
dropped); the cxx98 `throw()` arm of headers.rs's nothrow set removed; one stdbit.h `__NIJI_NOTHROW` removed; the
C++11 exclusion of isinf removed (part 1).

**RULING NEEDED (precheck slice-contract RED).** The branch changes the shared generator in two places:
xtask/src/headers.rs (nothrow set = C++17 `noexcept` OR C++98 `throw()`: math.h's 34 C99 functions) and
xtask/src/headers/cond.rs (NOT_CXX11: isinf/isnan functions out of C++11+, probe-settled). Both are table-driven,
covered by the rows test and plants p4cs-cxx98-throw / p4cs-not-cxx11. Recommend: accept them in this story (a
cross-slice P4 follow-up, not a slice; the slice data cannot express either), else split them into a generator
story carrying these two commits unchanged. precheck is otherwise green on c3297077a.

**Follow-up.** Recommend story HEADERS-O2-INLINES-2: give the 17 remaining -O2 inline forms (ctype.h
`is*_l`, the gettext family via dcgettext/dcngettext and LC_MESSAGES, `__cmsg_nxthdr`) under D-X.158's design and
drop their 34 accept lines; the ctype ones read libniji's locale table inline, a layout dependence that needs the
locale owner's look.

**Not to redo.** The baseline triage; the reserved-name policy (ruled); the div and isinf probes.

**Next step.** If a job above is red: fix and rerun only it. Otherwise nothing: hand back.
