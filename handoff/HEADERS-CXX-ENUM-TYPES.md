# HEADERS-CXX-ENUM-TYPES: enumerator and char-constant macros have the reference's C++ types (handoff)

**State (leg 2, HEADERS-CXX-ENUM-TYPES-2):** branch `task/headers-cxx-enum-types-2` = `task/headers-cxx-enum-types`
(head 2b38a86e5) plus bf7e09919, a merge of origin/next 623550dac resolving the CI conflict in
`spec/tools/header_checks/form.py`: both FAIL paths kept, next's `enforced()` (ENFORCED (macro, header) pairs,
HEADERS-FORM-BREAKS) and this branch's `failing()` (C++ type / enumeration differences). A row both select is
reported once (by `enforced()`, via `is_enforced()`); `enforced()` lines no longer carry their own `FAIL ` prefix
(header_check.py and `main()` print it; before the merge header_check printed `FAIL FAIL form ...`). Generated
headers current after the merge (`headers-gen --check` rc 0). Plants of the merge:
`handoff/HEADERS-CXX-ENUM-TYPES-2-plants.txt`. Head: `git log -1`. Role keke:~/w/n2.

**State (leg 1):** branch `task/headers-cxx-enum-types` (from origin/next 44304e709); code commits 32f76996e (the change) and
ed6a2610c (fixes found by `cargo test -p xtask`). Role keke:~/w/r1. Scratch:
~/w/.scratch/HEADERS-CXX-ENUM-TYPES/ (probe sources overload.cc/.c/.sh, ms.c/ms.sh). Decision: D-X.171 (new).
Plants files: `handoff/HEADERS-CXX-ENUM-TYPES-plants.txt` (5), `handoff/HEADERS-CXX-ENUM-TYPES-plants-2.txt` (3).

## Done
* C1 (32f76996e): `spec/tools/enum_probe.py` black-box probe -> enumerators.tsv `anon FIRST` rows (59 enumerations,
  758 rows) and `spec/abi/enum-macros.tsv` (897 macros); generator declares unnamed enumerations once (guard
  `__NIJI_D_anon_FIRST`), every `enum TAG` whole, a mapped macro `#define N ENUMERATOR`. 37 `<langinfo.h>` macros name
  a reserved `__niji_NAME` member.
* C2: 20 macros.tsv rows `int` -> `char`, emitted as char literals (`char` in C++, `int` in C).
* form.py: `type differs` and `enumeration differs` FAIL (facts `ut`, `same-as`; controls NIJI_EA/NIJI_EB).
  cxx17 type rows 1198 before (job headers-cxx-enum-types-form0) -> 45 after (27 MS_* D-X.126, 18 REG_* gap; job
  headers-cxx-enum-types-gen5); enumeration differs 0.
* ed6a2610c: (1) an unnamed enumeration is under `#if` "a member it would declare is not yet a macro" (`enum { };`
  with `<linux/in6.h>` first: s12 network_headers_pass_header_check); (2) an enumerator emitted outside any header
  (rows test) declares its whole enumeration; (3) rows test checks macros.tsv `char` (int in C, char in C++, value).
  D-X.171 record; spec/abi/PROVENANCE paragraph.
* Overload proof (job headers-cxx-enum-types-overload3, rc 0): overload set f(int)/f(unsigned)/f(char)/template
  f(enum) picks the same overload on both libraries for _SC_ARG_MAX, _PC_LINK_MAX, _CS_PATH, CODESET, EPOLLIN, MSG_PEEK,
  SOCK_STREAM, IPPROTO_TCP, RUSAGE_SELF (template f(enum)), REGTYPE, ELFMAG1, SCOPE_DELIMITER (f(char)); same `sizeof`
  and same-type-as-sibling; C `_Generic`: int (unsigned for EPOLL*) on both. Output "SAME on both libraries".
* MS_* probe (job headers-cxx-enum-types-ms): reference, both include orders of `<sys/mount.h>`/`<linux/mount.h>`, C
  and C++, `-Wall -Wextra -Wsystem-headers`: compiles, 0 warnings; MS_RDONLY/MS_BIND/MS_NOUSER are macros expanding to
  their own name and enumerators in C++ (MS_RMT_MASK a macro expression, MNT_FORCE enumerator). libniji: both orders
  compile, MS_* expand to literals (`int` in C++). D-X.126's premise (an enumerator and a macro of one name cannot
  come from two headers included in either order) does not hold for the reference. RULING NEEDED below; not
  implemented (brief: implement only what the ruling settles).

## Ruling (resolved: #8488, follow-up HEADERS-ENUM-FOLLOWUPS)
* MS_* (D-X.126 / COMPAT-34): recommend superseding D-X.126 in a follow-up story: MS_* as enumerators of one unnamed
  enumeration (underlying int) with self-naming macros, compiling in both include orders (libniji's own design: e.g.
  when the kernel header came first, its MS_* macros are replaced before the enumeration; when it comes after, its
  definitions must not replace them), then retire COMPAT-34 and the differences row. Evidence above.

## Open / informational
* REG_* (18 rows): `<regex.h>` declares no `reg_errcode_t`: follow-up (typedefs/visibility).
* arpa/nameser.h `ns_type`, `ns_class`, ...: opaque structs in libniji, enumerations in the reference: follow-up.
* `affected --list` escalates to FULL (generated headers): FULL is the train's; direct proof = `cargo test -p xtask`
  + precheck.

## Jobs on the merged head (leg 2), keke:~/w/n2
* headers-cxx-enum-types-2-lint1: `headers-gen --check` rc 0, clippy -D warnings, `cargo xtask static`: rc 0.
* headers-cxx-enum-types-2-test2: `header_check.py` (0 failed of 17150; form: cxx17 type rows 45 as before the
  merge, 18 REG_* known-gap NOTEs) + `cargo test -p xtask` (661 passed in the bin target, all targets green).
* headers-cxx-enum-types-2-plants1: HEADERS-CXX-ENUM-TYPES-2-plants.txt, 4 CAUGHT by message, 1 EQUIVALENT (m-no-dedupe,
  reason in the file), 0 WARN.
* Every commit after bf7e09919 is handoff/ only.
* Superseded: headers-cxx-enum-types-2-test1 (same checks green, HC=0 XT=0, but rc 134: a diagnostic
  `affected --list | head` appended to the job aborted on SIGPIPE).

## Jobs on the leg-1 code (ed6a2610c; later commits are handoff/ only), keke
* headers-cxx-enum-types-lint2: clippy -D warnings + `cargo xtask static`, rc 0.
* headers-cxx-enum-types-test3: `cargo test -p xtask`, rc 0 (661 passed in the bin target; other targets green).
* headers-cxx-enum-types-plants3: plants.txt rerun, 5/5 CAUGHT (by message: the commands run form.py, no cargo test).
* headers-cxx-enum-types-plants4: plants-2.txt, 3/3 CAUGHT (--timeout 1800).
* Probes: headers-cxx-enum-types-overload3 (proof, rc 0), headers-cxx-enum-types-ms (MS_* include orders).
* Superseded: test1 (head 32f76996e, 2 failures fixed in ed6a2610c), test2/gen6 (diagnostic reruns), plants (head
  32f76996e), plants2 (baseline of anon-empty-enum timed out at 600 s: headers-gen + test build + s12 check under
  load; no plant applied in that copy), overload/overload2 (probe flags: SCOPE_DELIMITER needs _GNU_SOURCE in C).

## Exact next step
Land task/headers-cxx-enum-types-2 (the merge resolved the CI conflict). Ruling #8488 settled MS_*: the follow-up
HEADERS-ENUM-FOLLOWUPS (MS_*, reg_errcode_t, nameser enumerations) waits for this landing; when it lands it drops
`KEPT` and the REG_* `GAPS` entry in form.py. Nothing else is owed on this branch.
