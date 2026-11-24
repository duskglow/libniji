# HEADERS-CXX-ENUM-TYPES: enumerator and char-constant macros have the reference's C++ types (handoff)

**State:** branch `task/headers-cxx-enum-types` (from origin/next 44304e709), head: see `git log -1` (code commit
32f76996e). Role keke:~/w/r1. Scratch: ~/w/.scratch/HEADERS-CXX-ENUM-TYPES/ (probe TSVs enum2.tsv, form0.diff, form1.diff).

## Done (commit 32f76996e)
* `spec/tools/enum_probe.py`: black-box probe of the reference (C++ program prints per name is_enum, enumeration name via
  `__PRETTY_FUNCTION__`, group via `std::type_index`, `__underlying_type`, sizeof, value; C program: does `#if` see an
  identifier). `--derive <probe.tsv>` writes enumerators.tsv `anon FIRST` rows (59 unnamed enumerations, 758 rows) and
  `spec/abi/enum-macros.tsv` (897 macros -> enumerator, `not_in` headers). 37 `<langinfo.h>` macros with no same-value
  enumerator name a new reserved `__niji_NAME` member. No reference macro is a literal in C where C++ has an enumerator.
* Generator (`xtask/src/headers.rs`, `headers/enums.rs`, `headers/values.rs`): unnamed enumerations declared once
  (guard `__NIJI_D_anon_FIRST`, members under own conditions, `#ifndef` per member for kernel macros); every
  enumerators.tsv `enum TAG` goes through its whole definition (`declared_enum` no longer needs a typedefs.tsv row:
  fixes EPOLL_EVENTS, __socket_type, __itimer_which, __priority_which, __rusage_who); a mapped macro is
  `#define N ENUMERATOR` where the header declares it; `char` macros.tsv type -> char literal (`'0'`, `'\377'`).
  `headers/sys/xattr.h.in` macros name their enumerators (template check allows `#define N N` for enum-macros names).
* C2: 20 macros.tsv rows type `int` -> `char` (tar.h *TYPE, CBRK/CEOL/CSTATUS, ELFMAG1-3, REC_*, SCOPE_DELIMITER,
  _POSIX_VDISABLE); header_check.py P1 asserts `char` values too.
* `form.py`: new C++ facts `ut` (underlying type) and `same-as` (one type with the previous enumeration-typed macro),
  control cases NIJI_EA/NIJI_EB; `type differs` and `enumeration differs` now FAIL (header_check module and
  stand-alone). Exempt: MS_* (D-X.126); labelled gap NOTE: REG_* in regex.h (libniji declares no reg_errcode_t).
* Result (cxx17 type rows): before 1198 (1159 C1 + 39 C2, job headers-cxx-enum-types-form0) -> after 45 = 27 MS_* +
  18 REG_* (job headers-cxx-enum-types-gen5); enumeration differs 0. Other causes unchanged.
* clippy + static clean on 32f76996e (job headers-cxx-enum-types-lint1). `affected --list`: FULL escalated
  (generated headers): FULL is the train's; direct proof = `cargo test -p xtask` + precheck.

## Jobs started (keke, on 32f76996e)
* `headers-cxx-enum-types-test1` (kind test: `cargo test -p xtask`, log has the last 40 lines only: piped through tail,
  a slip; rerun without the pipe if the dispatcher needs the full log).
* `headers-cxx-enum-types-plants` (5 plants in `handoff/HEADERS-CXX-ENUM-TYPES-plants.txt`, dir
  /dev/shm/claude/headers-cxx-enum-types/plants-1).

## Exact next steps
1. `~/bin/jobwait headers-cxx-enum-types-test1 headers-cxx-enum-types-plants`; fix what they find (plants: each must
   be CAUGHT with its msg).
2. Overload proof (brief "Proof"): a C++ program, `void f(int)` / template `void f(T)` for enums, picks the enum
   overload for `_SC_ARG_MAX`, `EPOLLIN`, `REGTYPE` (char overload) on both libniji (`-nostdinc -isystem libc-include`
   + gcc include + kernel uapi, as form.py's Side flags) and the reference; record the output.
3. MS_* RULING: reference probe says the 27 MS_* are enumerators of one unnamed enumeration (underlying `int`) in
   C++ and macros naming them in C. D-X.126 keeps macros for include-order safety with `<linux/mount.h>`. Probe the
   reference: `<sys/mount.h>` then `<linux/mount.h>` and the reverse, C and C++; recommend keeping D-X.126 unless the
   reference compiles in both orders with enumerators (then propose dropping COMPAT-34 to a ruling).
4. Report: handoff/reports/HEADERS-CXX-ENUM-TYPES.json (+ "jobs"); precheck `--jobs headers-cxx-enum-types-test1,
   headers-cxx-enum-types-plants --head <sha>`; `ops handback-check`.

## Open / informational
* REG_* (18 rows): reg_errcode_t typedef missing in libniji's `<regex.h>` (typedefs/visibility gap): follow-up story.
* arpa/nameser.h: `ns_type`, `ns_class`, ... are opaque structs in libniji, enumerations in the reference (outside C1
  as the audit counted it; E1 group): follow-up.
* A D-row for the `anon` tag form and enum-macros.tsv is owed (reserve with `ops reserve --story ... D-X`), and a
  spec/abi/PROVENANCE line for enum_probe.py.
