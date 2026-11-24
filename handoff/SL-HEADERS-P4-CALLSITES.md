# SL-HEADERS-P4-CALLSITES (handoff, leg 1 of run 1)

**State.** Branch `task/sl-headers-p4-callsites` (from origin/next 44304e709), WIP commit 86d4904ef pushed. Role keke:~/w/n4
(synced at 86d4904ef). Scratch: `~/w/.scratch/SL-HEADERS-P4-CALLSITES/` (triage.py, base/ = baseline tables,
triage-base.txt). Job running: `sl-headers-p4-callsites-p4run1` (keke; `~/work/sl-headers-p4-callsites/job1.sh`:
`cargo xtask headers-gen`, `git status -- libc-include`, `cargo xtask so`, P4 full into
/dev/shm/claude/sl-headers-p4-callsites/p4run1). Baseline job `sl-headers-p4-callsites-p4base` (head 44304e709; P4
part 1 clean, 0 FAIL every -std; part 2 FAIL: full P4 takes ~13 min).

**Ruling in force.** Challenge #8468 ACCEPTED by the coordinator (22:50Z): P4 part 2 follows D-X.107 (R-H2/R-H3) and
the same headers/reserved.tsv exclusion as P2/P3 (`spec/tools/probe_compare.py reserved_omitted`); emit stdio_ext.h's
documented `__` names; fix the real non-`__` differences; list the rest with reasons. No ruling against D-X.107; a
Yohane package that needs an excluded `__` name gets it later by the usual rule.

**Baseline triage (rows: noexcept / callsites spec-only / mine-only).** Policy-omitted (`__x` default or reserved.tsv
omit row), now excluded: math.h 716 names (716/716/716), resolv.h 33, argp.h 9+2 omit-row, argz.h 5, error.h 4,
string.h 5, ctype.h 2, libintl.h 2, stdio.h 1, signal.h 1 + bsd_signal (omit row), sys/shm.h, unistd.h, sys/gmon.h,
nss.h, libgen.h 1 each, netinet/in.h bindresvport6 and sys/stat.h getumask (omit rows; differences rows D-X.117,
D-X.124). Real, fixed in 86d4904ef (unverified until p4run1): stdio_ext.h 9 `__f*` (reserved.tsv emit rows, spec
12.4); stdbit.h 70 exception specs (`__NIJI_NOTHROW` on every definition in stdbit.h.in); math.h 34 C99 functions
cxx98 `throw()` (headers.rs nothrow set = cxx17 noexcept OR cxx98 throw()); math.h isinf/isnan C function decls
(`@@when` + `@@owns-macro` instead of `@@if`); alloca function decl (alloca.h.in `@@owns-macro`, providers.tsv `none`).
Real, NOT fixed yet (callsites, ABI form): ctype.h 12 `is*_l` (reference inline `-`, libniji calls the export);
libintl.h gettext/dgettext/ngettext/dngettext (reference calls dcgettext/dcngettext at -O2, libniji the named
function); netrose/rose.h `__cmsg_nxthdr` (reference inline `-`, libniji call); sys/pidfd.h 4 functions (reference C++
has no C linkage: `_Z11pidfd_getfdiij`...; libniji declares `extern "C"`, the generator already filters this, see
functions.rs comment "a C++-mangled name that no library exports"); stdlib.h div cxx17 (reference `error`, libniji
`noexcept`: libniji's stdlib.h + the C++ library give one `div`, the reference an overload set). argp_usage,
argz_next, _option_is_end: accepted by validation/headers_s17.accept (D-X.131), read by the new harness.

**Harness (validation/headers_p4.py, 86d4904ef).** Part 2 compares row sets minus policy-omitted names; prints each
other difference as `P4 FAIL <table>: <line>` (probe_compare form; noexcept lines `differs noexcept <h> <name> table
[a b c] libniji [...]`); accepted lines come from validation/headers_p4.accept and validation/headers_s*.accept; a
headers_p4.accept line not seen is a stale FAIL. `--no-cxx` skips part 1 (for plants).

**Next step.** (1) `~/bin/jobwait sl-headers-p4-callsites-p4run1` on keke; read `grep -E '^P4|^ M|error' ~/logs/...log`.
If headers-gen errored, fix the template/table. (2) Bring the regenerated libc-include files into the worktree (the
job's `git status` lists them; copy each with `ssh cat` into the worktree, never open them) and commit them. (3) For
each remaining FAIL line: fix if cheap, else add it to validation/headers_p4.accept under a reason comment (pidfd:
needs a differences row via `ops reserve` or RULING NEEDED; ctype `_l`, gettext, cmsg in rose.h, div: follow-up story
with reason). (4) Plants in docs/plants.md format in handoff/SL-HEADERS-P4-CALLSITES-plants.txt: drop one math.h `__`
... NOTE the brief's math.h `__` plant cannot apply (none emitted, ruling #8468): plant `__fpending` emit row removed
(reserved.tsv) and one other (e.g. the stdbit `__NIJI_NOTHROW`, or the isinf `@@when`), cmd
`cargo xtask headers-gen && cargo xtask so && python3 validation/headers_p4.py --no-cxx --jobs 24`, msg `P4 FAIL`.
(5) clippy, static, `cargo xtask affected --list --base origin/next` (FULL expected: headers/*.in; then gate =
precheck + the rows test `headers::tests::every_table_row_is_emitted_and_matches_the_tables` + niji_cc_headers), report
json, precheck, handback-check.

**Not to redo.** The baseline triage; the reserved-name policy question (ruled).
