# HEADERS-MACRO-FORM-AUDIT: macro form audit, libniji vs the reference (report)

**State:** branch `task/headers-macro-form-audit` (from origin/next efa8bb9c0); head: see
`handoff/reports/HEADERS-MACRO-FORM-AUDIT.json`. Audit only: no header, template or generator changed (brief).

**What landed:** `spec/tools/header_checks/form.py`, a module of the existing harness (`header_check.py` lists the
directory; the only change to `header_check.py` is `ctx.inc, ctx.work`, so a module probes the tree the harness was
given). It runs inside `header_check.py` (xtask test `network_headers_pass_header_check`) in **report mode**:
`NOTE FORM-DIFF ...` / `NOTE FORM-RUNTIME ...` lines and a summary, never a FAIL. The only FAILs are the probe's own
control (a header of the probe with one macro of each form: constant, `const`-function call, enumerator, char
constant, float, cast, undeclared identifier, undeclared call, undefined), so a probe that stops telling forms apart
is caught. Cost: about 15 s wall / 80 s CPU on the Skylake-SP host (16 compilers); `NIJI_FORM_PROBE=0` skips it.
Stand-alone: `python3 spec/tools/header_checks/form.py [--tsv F] [--diff F] [--values] [--headers H,..]`.
Turning it into a failing check (coordinator, after the follow-ups): return the FORM-DIFF rows from `problems()`.

**Method (black box):** the same unit compiled against `libc-include/` (`-nostdinc`) and the host's own headers; only
exit status, the unit's own line numbers (`-fdiagnostics-plain-output`, no caret) and `nm -S` sizes of the unit's own
objects are read. Facts per (macro, defining header, configuration, side): defined; `#if`; expression;
integer constant expression (`_Static_assert(((M) & 1) + 1)`: value-dependent, so a `const` call cannot be folded);
file-scope array size; static initializer (C); `constexpr` (C++); type (`_Generic` / overload code; `__is_enum` in
C++) and `sizeof`; run-time value (`--values`, reference only, see Not verified). C units use
`-Werror=implicit-function-declaration -Werror=implicit-int` (gcc 14's default): with gcc 13 and `-w` a macro that
calls an undeclared function looked usable. A unit is re-compiled without its failing lines until it compiles clean
(the C front end reports an undeclared identifier once only, so one round can show false passes).
Configurations: compiler default (gnu17), `-std=c11`, `-std=c11 -D_POSIX_C_SOURCE=200809L`, `-D_XOPEN_SOURCE=700`,
`-D_GNU_SOURCE`, `-D_DEFAULT_SOURCE`, `-D_DYNAMIC_STACK_SIZE_SOURCE` (added: it selects the run-time forms), `g++ -std=c++17`.

## Inventory and result (job headers-macro-form-audit-t7 on mei, final probe; origin/next efa8bb9c0 headers)

8265 object-like macros (libniji's own `__NIJI_*` excluded; job t8 on the final head gives the same numbers as t7;
`regexp.h`, `sys/elf.h` are must-fail),
13480 (macro, defining header) pairs, 115 headers, x 8 configurations. Both package flavours ship the same
`libc-include/` tree, so one inventory covers both. **12042 pairs (7241 macros) match in every configuration;
1438 pairs (1024 macros) differ in at least one** (2771 rows).

## Differences, grouped by cause (libniji form -> reference form)

| # | cause | macros (header) | configs | libniji | reference |
|---|---|---|---|---|---|
| A1 | run-time in the reference, constant in libniji | PTHREAD_STACK_MIN (`<pthread.h>`, `<dirent.h>`; `<limits.h>` matches) | gnu, dynstack, cxx17 | int constant 16384 | `long` run-time expression (value 16384 here) |
| A2 | does not compile in libniji | MINSIGSTKSZ, SIGSTKSZ (`<signal.h>`) | gnu, dynstack, cxx17 | no use compiles: `sysconf` is not declared by `<signal.h>` (gcc 13 accepts the C use only through an implicit `int` declaration, which truncates the `long`; gcc 14 and g++ reject it) | run-time `long` expression (14528 here) |
| B1 | macro names a function the header does not declare | `basename` (`<libgen.h>`, expands to `__xpg_basename`), `res_init`, `res_ninit`, `res_nclose` (`<resolv.h>`) | all | not an expression (C: implicit declaration under gcc 13, error under gcc 14) | function designator |
| B2 | wrong macro | INFINITY in `<sys/vlimit.h>` | all | the float INFINITY | `int` constant usable in `#if` |
| B3 | not a file-scope array size in libniji | MS_NOUSER (`<sys/mount.h>`, `(-2147483647-1)`) | C configs | ICE, initializer, not an array size (cause not diagnosed) | array size |
| C1 | C++ type: enumerator in the reference, `int` literal in libniji | 1,200 rows: `_SC_*`, `_PC_*`, `_CS_*` (`<unistd.h>`, `<signal.h>`), nl_item (`<langinfo.h>`), MSG_*, IPPROTO_*, MS_*/MNT_*, PT_*, REG_* (ucontext, regex), MM_*, PRIO_*/RLIMIT_*, FTW_*, IFF_*, EPOLL*, ST_*, PTHREAD_*, DT_*, TFD_*, EFD_*, FSETLOCKING_*, ITIMER_*, IN_*, SFD_*, UL_*, XATTR_* | cxx17 | `int` (`unsigned` for EPOLL*) | an enumeration type (overloads, templates, `decltype` differ) |
| C2 | C++ type: char constant in the reference | tar.h `*TYPE`, CBRK/CEOL/CSTATUS, ELFMAG1-3, REC_EOF/EOR/ESC, SCOPE_DELIMITER, _POSIX_VDISABLE | cxx17 | `int`, size 4 | `char`, size 1 |
| D1 | `#if`-usable in libniji only (reference: a cast or other non-`#if` form) | INADDR_* (10), RE_* syntax bits (39), RES_* (5), BLKBSZGET/BLKBSZSET/BLKGETSIZE64, AI_/NI_IDN_* (4), CLOCKS_PER_SEC, ELF_NGREG, FAN_EVENT_METADATA_LEN, IPC_PRIVATE, MM_NULLMC, PR_SET_PTRACER_ANY, _SS_PADSIZE, FP_XSTATE_MAGIC2_SIZE, ICMP_ADVLENMIN, NFDBITS | most | usable in `#if` | not usable in `#if` |
| D2 | `#if`-usable in the reference only | SIG_ERR, LC_GLOBAL_LOCALE | most | not in `#if` | `#if` accepts it (a cast spelled so the preprocessor reads it as arithmetic) |
| E1 | expression in libniji only: the reference's header alone does not declare what its macro expands to | 125 `<arpa/nameser_compat.h>` names; TTYDEF_* (`<sgtty.h>`, `<sys/ioctl.h>`, `<sys/ttydefaults.h>`, `<pty.h>`, `<termio.h>`); SSIZE_MAX (`<dirent.h>`); _XOPEN_IOV_MAX (`<limits.h>`, xopen700); DLFO_EH_SEGMENT_TYPE (`<dlfcn.h>`, gnu, cxx17) | listed | integer constant | defined, no use compiles |
| F1 | defined in libniji only | SYS_listmount, SYS_statmount, SYS_lsm_get_self_attr, SYS_lsm_set_self_attr, SYS_lsm_list_modules | all | defined | undefined (newer kernel numbers) |
| F2 | defined in the reference only | __SOCKADDR_ARG, __CONST_SOCKADDR_ARG (`<sys/socket.h>`) | cxx17 | undefined | defined (reserved names) |

**Run-time macros of the reference** (`FORM-RUNTIME`): MB_CUR_MAX, SIGRTMIN, SIGRTMAX, SHMLBA, errno, h_errno (libniji
matches in every configuration); MINSIGSTKSZ, SIGSTKSZ (A2, A3), PTHREAD_STACK_MIN (A1; `<limits.h>` matches).

## Recommended follow-ups (the coordinator briefs them)
1. A1-A3 (owner ruling #8417 scope): PTHREAD_STACK_MIN in `<pthread.h>`/`<dirent.h>` as `<limits.h>` has it;
   MINSIGSTKSZ/SIGSTKSZ of type `long` and compiling in C++ (declare `sysconf` or call through a declared helper).
2. B1, B2, B3: header defects (B1 can truncate a pointer under gcc 13); small generator/template fixes.
3. C1, C2: generator emits the enumerator (enumeration type) in C++, char constants as char literals. RULING NEEDED
   on C1 against D-X.126 (MS_* chosen as macros only, COMPAT-34): the type differs in C++ for every such macro.
4. D1/D2/E1/F: low impact; D1 means code that compiles on libniji fails on the reference. Ruling on whether to match.

## Not verified
* libniji run-time values: `target/niji` is not a sysroot and no niji6 install is on the host, so `--values` ran the
  reference only (`--niji-cc` takes a link command). Constants: same value as compile time (P1 checks integers).
* The Zen 1 class: reference MINSIGSTKSZ/SIGSTKSZ (14528 on Skylake-SP with AVX-512) not measured there.
* B3's cause was not diagnosed (audit only).

## Not to redo
The inventory and the table above (job t4); the probe design notes in the module docstring.
