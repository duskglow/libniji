# HEADERS-GETTEXT-CLANG report

Branch `task/headers-gettext-clang` from origin/next db602e58c. Commits: ab2c1b599 (code, tables, header_check rows,
D-26.4), e02f0bc0e (clippy `type_complexity`: a `FloatBase` alias, no behaviour change), then handoff/report commits.
Role: keke `~/w/r3`.

## What changed (Yohane's items)

1. **gettext:** `<libintl.h>` defines `__GNU_GETTEXT_SUPPORTED_REVISION(major)` as `((major) == 0 ? 1 : -1)`, int,
   usable in `#if` (form probed on the reference: value, type, `#if` use, argument parenthesised). header_check unit
   `libintl.h __GNU_GETTEXT_SUPPORTED_REVISION forms` (s10.py) in every configuration that shows it; "not shown" units
   elsewhere.
   The claim is backed: `catalogs_products` (catalogs_equal_the_reference: ngettext, dngettext, dcngettext plural
   forms, and a msgfmt revision-1 catalog with system-dependent `<PRIu64>` segments, compared with the reference)
   passes, 3 of 3, on the final head's products (job `headers-gettext-clang-catalogs`).
2. **clang:** features.h sets `__NIJI_FLOATN` / `__NIJI_FLOAT128` from the compiler's predefined macros;
   headers-gen guards every generated line naming a `_FloatN` type, suffix or builtin (`values::floatn_guards`, D-26.4).
   stdbit.h's bool typedef is `bool` in C++. header_check P5 runs with clang-19 too (C and C++: c17, gnu2x,
   c17+_GNU_SOURCE, c++17, gnu++20; resource dir before the libc's, as the driver orders them): 0 failures on the
   final head for every header (the full run: `header_check: 0 failed of 21913`, clang P5 included).
3. **The rows:** utmp.h `ut_name`, `ut_time`, `ut_xtime`, `ut_addr` (+ UTMP_FILENAME, WTMP_FILENAME); ifaddrs.h and
   net/if.h `ifa_broadaddr`, `ifa_dstaddr`; `greg_t` typedef rows. `gregset_t`, `fpregset_t`, `REG_*`,
   `<scsi/scsi.h>` command and status codes, `struct seminfo`, `SEM_INFO`, `SEM_STAT`, `SEM_STAT_ANY` were already
   on next (other header slices); compile probes below confirm them.

## Proofs

**Compile probes** (gcc, g++, clang-19, clang++-19; C c17/gnu17/gnu2x/c17+_GNU_SOURCE, C++ c++17/gnu++17/gnu++20;
reference system vs libniji's tree, job `headers-gettext-clang-probe6` on ab2c1b599's headers, unchanged since):
ut_name/ut_time/ut_xtime/ut_addr, ifa_broadaddr/ifa_dstaddr, the gettext forms, greg_t/gregset_t/fpregset_t/REG_*,
uc_mcontext.gregs[REG_RIP], the SCSI codes and status values, struct seminfo + SEM_*: same result as the reference in
every cell. Remaining differences, all recorded: `__HAVE_FLOAT*`/`__HAVE_DISTINCT_FLOAT128` not defined (reserved,
D-26.4), HUGE_VAL_F* and strict-C++ CMPLXF* (INFORMATIONAL below), clang tgmath (INFORMATIONAL), utmpx
`__e_exit` naming in strict modes (INFORMATIONAL).

**Packages** (sources from the local Debian mirror only, `dpkg-source -x`; build dependencies' -dev packages extracted,
not installed, from the same mirror). The compiler wrapper compiles with `-nostdinc`, the compiler's own headers,
libniji's `libc-include` (the system flavour's `/usr/include`), then every non-libc6-dev header of the host and the
extracted packages; it links libniji's `niji-crt1.o` and `libc.a` statically (`target/niji` of the final head).
Jobs `headers-gettext-clang-pkg1` to `-pkg6` (pkg2: tar, lvm2; pkg1: sysvinit, psutil; pkg3: parted, openldap, ruby gcc; pkg4: ruby clang; pkg5, pkg6: postgresql):

| Package | Step | Result |
|---|---|---|
| tar 1.35+dfsg-3.1 | configure | `checking for GNU gettext in libc... yes` (AM_GNU_GETTEXT external, need-formatstring-macros), `whether to use NLS... yes` |
| tar | make, make install | rc 0; 39 `.mo` files installed; the built (static, libniji) tar prints its help in German (`Aufruf: tar [OPTION...] [DATEI]...`) with LANGUAGE=de |
| postgresql-17 17.12 | JIT bitcode with clang-19 | configure rc 0 (--with-llvm, CLANG = the clang-19 wrapper); `src/timezone/localtime.bc` (the file Yohane's build stopped on: stdlib.h `_Float128`) rc 0 through make; `float.bc`, `numeric.bc` (math.h-heavy) rc 0 with make's bitcode flags, compiled directly (make's prerequisites need a real bison for jsonpath_gram.c) |
| sysvinit 3.14-4 | `make utmp.o` (gcc and clang-19) | rc 0, both |
| ruby3.3 3.3.8-2 | `make signal.o` (gcc and clang-19) | rc 0 with gcc (configured with gcc) and with clang-19 (configured with clang-19: a gcc-configured tree asks for gcc's sanitizer header) |
| python-psutil 7.0.0-2 | `_psutil_posix.c` (gcc and clang-19, Python 3.13 headers) | rc 0, both |
| parted 3.6-5 | `libparted/arch/linux.c` | rc 0 (`make -C libparted arch/linux.lo`, after the gnulib build) |
| lvm2 2.03.31-2 | `libdm/libdm-common.c`, `device_mapper/libdm-common.c` | rc 0, both |
| openldap 2.6.10 | configure, `--enable-dnssrv` (res_query) | configure rc 0: `checking for resolver link (default)... yes` |

Build-host shims (the host lacks the tools, not a libniji matter): bison and flex answer `--version` only
(postgresql's release tarball ships its parsers); autotools outputs are touched after dpkg-source's patches so make
does not rerun autoconf (no autoconf on the host). openldap: `ac_cv_func_pthread_kill_other_threads_np=no`, what a
dynamic link against libniji's libc.so.6 (no such symbol) or the reference (compat-only version) finds; the static
wrapper alone finds it in libc.a and takes configure's LinuxThreads branch.

## Gate (FULL: the train's)

`cargo xtask affected --list` escalates to FULL (generated headers, header templates, xtask). Per the brief: job
`headers-gettext-clang-test1` (kind test) on e02f0bc0e: `cargo xtask clippy` OK, `cargo xtask static` OK,
`cargo test -p xtask` OK, `python3 spec/tools/header_check.py`: `0 failed of 21913`. Precheck on the final head.

## Plants (`cargo xtask plants`, job `headers-gettext-clang-plants`, 5 of 5 CAUGHT, no WARN)

floatn-guard-128 (values.rs `_Float128` guard dropped: xtask unit test), gettext-major (`== 0` -> `>= 0`: header_check
GETTEXT unit), utmp-addr (`ut_addr_v6[0]` -> `[1]`: member names utmp), float128-clang (features.h's
`__FLT128_MANT_DIG__` test dropped: clang P5 stdlib.h), stdbit-cxx-bool (C++ `bool` branch -> `_Bool`: clang P5
stdbit.h c++17). Results: handoff/HEADERS-GETTEXT-CLANG-plants.txt.

## INFORMATIONAL

* libniji lacks HUGE_VAL_F32/F64/F32X/F64X/F128 (reference: GNU C and C++; not in visibility.tsv).
* Strict C++ (`-std=c++17`): libniji defines CMPLXF32/F64X/F128, the reference does not.
* Strict modes: the reference names utmpx `__e_termination`/`__e_exit` and ucontext `__gregs`, `__fpregs`,
  `__uc_flags`; libniji has the unprefixed names in every mode.
* clang's own `<tgmath.h>` comes before the libc's in the driver's search order; libniji's tgmath.h needs
  `__builtin_tgmath`.
* libniji's libc.so.6 has no `pthread_kill_other_threads_np` (the reference exports it as a compat-only
  `GLIBC_2.2.5` version, for old binaries); libc.a has it as an ordinary symbol, so a static link finds it where the
  reference's dynamic link does not (openldap's configure then assumes LinuxThreads).
