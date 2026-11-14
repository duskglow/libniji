# Liella's own C header set (SL-HEADERS-PLAN)

Owner ruling R4 (#5538, 2026-11-14): "we do need headers, and using glibcs is against the spirit of the
project." Aqours cannot compile anything against Liella alone until Liella ships its own headers
(`plan/system-libc.md` §2 and §6 item 7). This plan answers the four questions of the brief: the
inventory (§1), how the headers are generated (§2), how they are proved (§3) and the slices
SL-HEADERS-1..20 (§4). §5 lists the owner rulings. Companion files: `plan/headers-inventory.tsv` (one
row per header name, with its spec section and slice), the slice briefs
`handoff/SL-HEADERS-PLAN/SL-HEADERS-<n>.md`, and the pending decision
`records/pending/SL-HEADERS-PLAN-1.toml` (series D-X).

## 1. Inventory

**Source.** Devuan excalibur `libc6-dev` 2.41-12+deb13u4 (amd64) from the project's Devuan mirror. The
chain was verified: each suite's `InRelease` with `gpgv` and the pinned Devuan keyring (Good
signature), then `main/binary-amd64/Packages.xz` against its SHA-256 in the verified `InRelease`,
then the `.deb` against its SHA-256 in `Packages`. Member names came from `dpkg-deb -c`; no member was
extracted. The same names were taken from the reference system's `dpkg -L libc6-dev`
(2.39-0ubuntu8.9).

**Counts (names only).**

* 478 `.h` names, plus `bits/stab.def`. 323 of them are under the multiarch directory
  `/usr/include/x86_64-linux-gnu/`.
* **243 are public headers**, and the public set is identical in 2.39 and 2.41.
* 235 are the reference's internal split: 231 `bits/*.h` names, plus `gnu/stubs*.h` and
  `gnu/lib-names*.h`.
* The only names that 2.41 adds are two `bits/` files. (`plan/system-libc.md` counted 505 headers
  with another method; the numbers here come from the names.)

**Kernel UAPI.** `linux-libc-dev` 6.12.116-1 (2560 names) provides `linux/`, `asm/` (per
architecture), `asm-generic/`, `drm/`, `rdma/`, `sound/`, `scsi/`, `misc/`, `mtd/`, `xen/` and
`video/`.

* No file name is in both packages. The only shared directory is `scsi/`: libc6-dev has `scsi.h`,
  `scsi_ioctl.h` and `sg.h`, and the kernel package has other names there.
* **Liella ships no UAPI header.** It depends on `linux-libc-dev`, as libc6-dev does.

**Not Liella's either.**

* `crypt.h` comes from `libcrypt-dev` (libxcrypt), and the Sun RPC headers except `rpc/netdb.h` come
  from the `rpcsvc-proto` and `libtirpc-dev` packages. These are libc6-dev's other dependencies.
* The compiler's own headers (`gcc -print-file-name=include`): `stddef.h`, `stdarg.h`, `float.h`,
  `stdbool.h`, `stdalign.h`, `stdnoreturn.h`, `iso646.h`, `stdatomic.h`, `stdckdint.h` and the
  intrinsics headers. `limits.h` and `stdint.h` **are** Liella's, because the compiler's versions
  chain to the C library's.

**Liella's internal split.** None of the 235 internal names are reproduced as such. Liella's internal
headers are its own design, under `__liella/`, with the parts that depend on the architecture in the
multiarch directory. There are three exceptions:

* `gnu/stubs.h`: configure scripts test its `__stub_<name>` macros. It lists Liella's own
  ENOSYS-only functions, from the spec.
* `gnu/lib-names.h`: it holds the soname strings, from `versions.tsv`.
* Any other internal name that the corpus scan (SL-HEADERS-1) shows programs include directly.

**Public headers by spec section** (243; the TSV is the authority, and the slice of each header is in
§4):

| § | headers |
|---|---|
| 0 base | `features.h` `features-time64.h` `stdc-predef.h` `sys/cdefs.h` `gnu-versions.h` `sys/types.h` `sys/bitypes.h` |
| 1 | `sys/auxv.h` `getopt.h` `argp.h` `sysexits.h` |
| 2 | `assert.h` `err.h` `errno.h` `sys/errno.h` `error.h` `execinfo.h` `fmtmsg.h` |
| 3 | `alloca.h` `malloc.h` `mcheck.h` `obstack.h` |
| 4 | `string.h` `strings.h` `memory.h` `wchar.h` `libgen.h` `argz.h` `envz.h` |
| 5 | `ctype.h` `wctype.h` |
| 6 | `iconv.h` `uchar.h` `gconv.h` |
| 7 | `locale.h` `langinfo.h` `libintl.h` `monetary.h` `nl_types.h` |
| 8 | `stdlib.h` `inttypes.h` `stdint.h` `limits.h` `stdbit.h` `endian.h` `byteswap.h` |
| 9 | `sys/random.h` |
| 10 | `math.h` `complex.h` `tgmath.h` `fenv.h` `fpu_control.h` `ieee754.h` `values.h` `finclude/…/math-vector-fortran.h` |
| 11 | `fcntl.h` `sys/fcntl.h` `poll.h` `sys/poll.h` `sys/select.h` `sys/epoll.h` `sys/eventfd.h` `sys/fanotify.h` `sys/inotify.h` `sys/file.h` `sys/ioctl.h` `sys/mman.h` `sys/sendfile.h` `sys/uio.h` `aio.h`; device ioctl headers `sys/kd.h` `sys/vt.h` `sys/soundcard.h` `sys/mtio.h` `sys/pci.h` `sys/raw.h` `scsi/scsi.h` `scsi/scsi_ioctl.h` `scsi/sg.h` |
| 12 | `stdio.h` `printf.h` `stdio_ext.h` |
| 13 | `unistd.h` `sys/unistd.h` `sys/stat.h` `sys/statfs.h` `sys/vfs.h` `sys/statvfs.h` `dirent.h` `sys/dir.h` `fts.h` `ftw.h` `mntent.h` `fstab.h` `sys/mount.h` `sys/swap.h` `sys/sysmacros.h` `sys/xattr.h` `utime.h` `paths.h` `tar.h` `cpio.h` `nfs/nfs.h` |
| 14 | `regex.h` `re_comp.h` `regexp.h` `fnmatch.h` `glob.h` `wordexp.h` `search.h` `sys/queue.h` |
| 15 | `sys/socket.h` `sys/socketvar.h` `sys/un.h` `arpa/inet.h` `ifaddrs.h`; `net/` (10); `netinet/` (13); `netpacket/packet.h`; legacy families `netash/` `netatalk/` `netax25/` `neteconet/` `netipx/` `netiucv/` `netrom/` `netrose/` (one each); `arpa/ftp.h` `arpa/telnet.h` `arpa/tftp.h`; `protocols/` (4) |
| 16 | `netdb.h` `resolv.h` `arpa/nameser.h` `arpa/nameser_compat.h` `nss.h` `aliases.h` `rpc/netdb.h` |
| 17 | `pwd.h` `grp.h` `shadow.h` `gshadow.h` `utmp.h` `utmpx.h` `lastlog.h` `sys/fsuid.h` |
| 18 | `spawn.h` `sys/wait.h` `wait.h` `sys/prctl.h` `sys/personality.h` `sys/pidfd.h` `sys/ptrace.h` |
| 19 | `termios.h` `sys/termios.h` `termio.h` `sgtty.h` `pty.h` `ttyent.h` `sys/ttychars.h` `sys/ttydefaults.h` |
| 20 | `signal.h` `sys/signal.h` `sys/signalfd.h` |
| 21 | `setjmp.h` `ucontext.h` `sys/ucontext.h` |
| 22 | `pthread.h` `semaphore.h` `threads.h` `sys/single_threaded.h` `sys/rseq.h` `thread_db.h` `proc_service.h` |
| 23 | `time.h` `sys/time.h` `sys/times.h` `sys/timeb.h` `sys/timerfd.h` `sys/timex.h` |
| 24 | `sched.h` `sys/resource.h` `sys/utsname.h` `sys/sysinfo.h` `sys/param.h` `ulimit.h` `sys/vlimit.h` `sys/acct.h` `sys/quota.h` `sys/reboot.h` `sys/klog.h` `sys/io.h` `sys/perm.h` `gnu/libc-version.h` |
| 25 | `mqueue.h` `sys/ipc.h` `sys/msg.h` `sys/sem.h` `sys/shm.h` |
| 26 | `syslog.h` `sys/syslog.h` |
| 27 | `dlfcn.h` `link.h` `elf.h` `sys/elf.h` `sys/platform/x86.h` `a.out.h` `ar.h` `stab.h` |
| 28 | `sys/gmon.h` `sys/gmon_out.h` `sys/profil.h` `sys/debugreg.h` `sys/procfs.h` `sys/reg.h` `sys/user.h` |
| 29 | `sys/vm86.h` |
| A | `syscall.h` `sys/syscall.h` (`SYS_*` from the kernel UAPI's `__NR_*`) |

Several names are aliases of another header. POSIX and the man pages name both forms: `poll.h` and
`sys/poll.h`, `wait.h` and `sys/wait.h`, `syslog.h` and `sys/syslog.h`, `sys/vfs.h` and
`sys/statfs.h`, and the `sys/` forms of `errno.h`, `fcntl.h`, `signal.h`, `termios.h` and
`unistd.h`. The legacy ones are R-H7.

## 2. Generation

### 2.1 What `spec/abi` holds, and what headers still need

Appendix C (OD-1) is read from the reference's public headers only through the compiler, and it
records facts, not text.

| fact | in `spec/abi` today | missing for headers |
|---|---|---|
| functions | `prototypes.tsv`, 3778 rows from `gcc -aux-info`. Types are spelled with the reference's reserved typedefs (`__pid_t`). The `header` column is the physical file: 1972 rows name a `bits/` file | the public header of each name; the form with typedefs resolved; `noexcept`; diagnostic attributes |
| integer constants | `constants.tsv`, 7030 values | header; macro or enumerator (`#ifdef` visibility); C type (`INT64_MAX` is `long`, `UINT32_MAX` is `unsigned int`); `SYS_*` (absent: the kernel's `__NR_*` are the source). Names with `__` (`__GLIBC__`) are excluded by design |
| other macros | none | string macros (`_PATH_BSHELL`, `PRId64`), floating macros (`M_PI`, `HUGE_VALF`), pointer-valued macros (`SIG_IGN` is recorded only as the integer 1), function-like macros (`WEXITSTATUS`, `S_ISDIR`, `CMSG_DATA`, `IN6_IS_ADDR_*`, `LIST_*` in `sys/queue.h`, obstack macros) |
| layouts | `layouts.tsv`: 161 types with member names, offsets, types and bit-fields (DWARF) | scalar typedefs (`pid_t`, `off_t`, `int64_t`, `socklen_t`, ...); struct, union and enum tag names (C++ linkage: `FILE` mangles through its tag); protocol, ELF and debug types (`struct ip`, `struct tcphdr`, `Elf64_Ehdr`, `struct user_regs_struct`, `struct flock`, `struct stat64`, ...) |
| initializers | `initializers.tsv` (10) | none |
| data objects | `versions.tsv`: 88 libc `OBJECT` names with sizes | C types (`environ`, `stdin`, `optarg`, `tzname`, `__libc_single_threaded`, ...) |
| inline protocols | Appendix C.9 is normative: `errno`, `assert`, ctype masks, `setjmp`, `FD_*`, `MB_CUR_MAX`, `SIGRTMIN`, `PTHREAD_STACK_MIN`, cleanup push/pop, `CMSG_NXTHDR`, `_chk` argument shapes | none: the headers implement C.9 exactly |
| call-site symbols | 44 `__isoc*` and 83 `*_chk` names are exported, so compiled programs reference them | which symbol a source call reaches for a given set of flags (`scanf` → `__isoc99_scanf`, `strtol` in C2X mode, `_FILE_OFFSET_BITS=64` names, fortified calls) |
| visibility | spec 0.5 names the layers; nothing is recorded per name | per (header, configuration): the visible names and their kinds |
| native | `native.tsv` → `include/liella/native.h` (`native-gen`, D-X.10) | none: it is the pattern for `headers-gen` |

### 2.2 New Appendix C tables (SL-HEADERS-2..4, ruling R-H0)

The new tables use the same provenance as C.0: compile-only probes of the reference. They are read
in only two forms:

* **diagnostics that name the probe's own identifiers.** For example, "`x` undeclared" for
  `(void)&x;`, and "unknown type name" for types;
* **symbols and data of the probe's own object.** For example `int vis_X;` under `#ifdef X`,
  `_Generic` type codes, the bytes of `const char s[] = X;`, and C++ functions such as
  `void p(decltype(&f))` read back through `nm -C`.

No one reads `-E` or `-dM` output or disassembly of the reference headers.

| table | content |
|---|---|
| `ftm-configs.tsv` | the configurations probed (§2.3) |
| `visibility.tsv` | header, configuration, name and kind: function, object, typedef, complete tag, `#ifdef` macro, enumerator |
| `typedefs.tsv` | resolved type and C++ linkage spelling of each typedef and tag; data-object types; typedef-resolved prototypes |
| `macros.tsv` | type of each integer constant; value and type of string and floating macros |
| `callsites.tsv` | per function and configuration, the symbol an `&f` (at -O0) or a call (at -O2, `_FORTIFY_SOURCE` 1..3) references |
| `noexcept.tsv` | C++17 overload probe on the function type (`R(*)(A...) noexcept`) |

### 2.3 Feature-test macros

* **Sources.** The macros and what they imply come from `feature_test_macros(7)`, POSIX XSH 2.2.1
  (`_POSIX_C_SOURCE`, `_XOPEN_SOURCE`) and 2.2.2 (the name space per header), C's
  `__STDC_VERSION__` and reserved identifiers (C 7.1.3), the ISO/IEC TS 18661 `__STDC_WANT_IEC_60559_*`
  macros, and `__STDC_WANT_LIB_EXT2__`.
* **Levels.** Liella's `features.h` turns them into its own levels, computed once:
  * `__LIELLA_ISOC`: 90, 99, 11, 17 or 23;
  * `__LIELLA_POSIX`: 0 up to 200809;
  * `__LIELLA_XSI`: 0, 500, 600 or 700;
  * `__LIELLA_MISC`: the BSD/SVID set that `_DEFAULT_SOURCE` makes visible;
  * `__LIELLA_GNU`, `__LIELLA_LFS64` (the `*64` names), `__LIELLA_DYNSTACK`
    (`_DYNAMIC_STACK_SIZE_SOURCE`, C.9), `__LIELLA_FORTIFY` (0..3).

  glibc's `__USE_*` names appear only where R-H2 rules.
* **Conditions.** Each declaration carries a condition over these levels. It comes from the standard
  that defines the name (C clause, or POSIX page and its XSI marking), the man page's "Feature Test
  Macro Requirements for glibc", and `visibility.tsv`. Where a man page and the probe disagree, the
  probe decides, because what programs compile against is the interface, and the difference is
  noted.
* **Strict modes.** In strict ISO or POSIX modes a header defines its standard's names, the reserved
  prefixes, and nothing else that the probe does not show.
* **`_FILE_OFFSET_BITS=64` and `_TIME_BITS=64`.** C0.4 says the `*64` symbols are exact aliases on
  x86_64, so these change no type. The headers still reference the symbol that `callsites.tsv`
  gives, so interposers such as fakeroot and `dpkg-shlibdeps` see what they see with the reference.
  Values the reference rejects (compile failure is a probe fact) are rejected.
* **C++.** `g++` predefines `_GNU_SOURCE`, as GCC documents, so the whole GNU set must compile as
  C++.
* **Configurations probed** (≈30):
  * C: `-std=c89/c99/c11/c17/c2x` with `__STRICT_ANSI__`, and `gnu17` and `gnu2x` with no macro;
  * POSIX: `_POSIX_C_SOURCE` = 1, 2, 199309, 199506, 200112, 200809;
  * X/Open: `_XOPEN_SOURCE` = (empty), 500, 600, 700, and `_XOPEN_SOURCE_EXTENDED`;
  * ISO: `_ISOC99_SOURCE`, `_ISOC11_SOURCE`, `_ISOC2X_SOURCE`;
  * `_DEFAULT_SOURCE` under a strict `-std`, `_BSD_SOURCE`/`_SVID_SOURCE`, and `_GNU_SOURCE`;
  * `_LARGEFILE64_SOURCE`, `_FILE_OFFSET_BITS=64`, `_TIME_BITS=64`, `_DYNAMIC_STACK_SIZE_SOURCE`,
    `_REENTRANT`;
  * `_FORTIFY_SOURCE` 1, 2 and 3 at -O2;
  * C++: `-std=c++98` up to `c++26`.

### 2.4 The generator: `cargo xtask headers-gen [--check]`

* **Shape.** The same shape as `native-gen` and `abi-gen`.
* **Inputs.**
  * the `spec/abi` tables (§2.1, §2.2);
  * Liella-written templates `headers/<path>.h.in`. These hold the structure, include guards and
    Liella-authored function-like macros written from the standard or man page (for example
    `sys/queue.h` from `queue(7)`, obstack from the GNU C Library manual, `CMSG_*` from
    POSIX/`cmsg(3)`, C.9's protocols).
  * **No ABI fact is typed into a template.** Values, layouts, prototypes and conditions enter only
    through generator markers.
* **Output.** `include/`, generated and committed like `include/liella/native.h`, with an
  `@generated` header line.
  * Facts that depend on the architecture go to a multiarch subtree, so `Multi-Arch: same` packages
    stay byte-identical in `/usr/include`.
  * `layouts-aarch64.tsv` and `versions-aarch64.tsv` already exist, so a second architecture is a
    second fact set, not a second header set.
* **`--check`.** Runs in `cargo xtask static`. It fails when:
  * the output is stale;
  * a declared function is not exported at a default (`@@`) version, or a compat-only (`@`) name is
    declared;
  * a table name assigned to a header is missing or emitted twice;
  * a template defines a name that a table owns.
* **Generated tests.** For each header, a C TU and a C++ TU of `_Static_assert`s cover sizes,
  alignments, offsets, constant values and types, and prototype identity
  (`__builtin_types_compatible_p` against the typedef-resolved form). The gate compiles them.
* **Staging.** `cargo xtask so` stages `target/liella/include`. Tests compile with `-nostdinc`,
  `-isystem` on Liella's headers, the compiler's include directory, and a staging of the build
  host's `linux-libc-dev` directories (`linux`, `asm`, `asm-generic`, ...).
* **Kernel UAPI coexistence.** `linux/libc-compat.h` (kernel text) describes a two-way protocol:
  * **Libc header first.** The kernel recognises the libc by `__GLIBC__`, the include guards
    `_NET_IF_H`, `_NETINET_IN_H` and `_SYS_XATTR_H`, and `__USE_MISC`/`__USE_GNU`. It then sets the
    `__UAPI_DEF_*` macros.
  * **Kernel header first.** The libc wraps its conflicting definitions in `#if !__UAPI_DEF_FOO`.
  * **Without `__GLIBC__`.** The kernel's other branch defaults every `__UAPI_DEF_*` to 1 unless the
    libc predefined it.

  Liella's choice follows R-H1/R-H2.

## 3. Proof

Each slice proves its own headers with P1, P2, P3, P5, P6, P8 and P9; P4 and P7 grow per slice.

| | check | how |
|---|---|---|
| P1 | identity with the tables | generated `_Static_assert` TUs, C and C++: sizes, offsets, constants, constant types, initializer bytes |
| P2 | visibility | the SL-HEADERS-2 probe, rerun on Liella's headers, gives `visibility.tsv` per (header, configuration) for the slice's headers, except rows in `docs/differences-from-glibc.md` |
| P3 | call sites | the `callsites.tsv` probe on Liella's headers, at -O0 and with fortify at -O2: the same symbols |
| P4 | C++ | `noexcept.tsv` and `typedefs.tsv` reproduced; `extern "C"` everywhere; the C++-standard overloads ([cstring.syn], [cwchar.syn]); every libstdc++ standard header compiles in every `-std` (SL-HEADERS-20) |
| P5 | hygiene | each header compiles alone, compiles when included twice, compiles under `-std=c89 -pedantic-errors` where C89 has it, and compiles with `-Wall -Wextra -Wsystem-headers -Werror`; namespace check in strict modes |
| P6 | conformance | `conformance/run.sh --liella-headers` (SL-HEADERS-5) builds every program whose `#include`s are already covered, against Liella's headers, with the same verdicts. 154 programs use 106 headers today (`string.h` 97, `stdlib.h` 83, `stdio.h` 82, `unistd.h` 80, `errno.h` 62, `pthread.h` 44, ...) |
| P7 | package corpus | the smoke programs (`validation/smoke`: zlib, xz-utils, sqlite3, grep, sed, diffutils, findutils, coreutils, gawk, bash, miniperl, python3.13 sample, cppunit), built against Liella's headers, give the same test-suite results and the same `nm -D --undefined-only` set per binary as the reference-header build |
| P8 | kernel UAPI | for each Liella header with a kernel counterpart (the `libc-compat.h` set, plus pairs the corpus uses): both include orders, with and without `_GNU_SOURCE` |
| P9 | plants | `cargo xtask plants`: a table row, a template condition, a redirect and a C.9 macro each mutated; the checks above must catch every one |
| P10 | C.9 macros | differential evaluation against programs built on the reference: ctype for all 384 inputs in C and C.UTF-8, `W*` status macros over all 16-bit values, `FD_*` over 0..1023 |

## 4. Slices (SL-HEADERS-1..20, each S or M)

Order: the tables first, then the generator, then a minimal C program, then groups by how much the
conformance and smoke programs use them. Fortify comes before any Aqours rebuild (R-H6). Slices 7–17
depend only on 6 and may run in parallel.

| # | slice | size | needs | headers / content | proof |
|---|---|---|---|---|---|
| 1 | corpus scan | S | none | `#include` counts and use of reserved names (`__GLIBC__`, `__USE_*`, `sys/cdefs.h` macros, `__*_t`, FILE members, direct `bits/` includes) in the Essential, build-essential and smoke sources; program sources only, never `lib/`, `gnulib/` or `gnulib-tests/` | rerun identical; spot counts |
| 2 | visibility tables | M | R-H0 | `ftm-configs.tsv`, `visibility.tsv`, C.12 text | every `prototypes.tsv` and `constants.tsv` name visible somewhere; rerun identical; plants on the checker |
| 3 | type and value tables | M | 2 | `typedefs.tsv`, `macros.tsv`, data-object types, new layout rows (§2.1) | cross-check with `layouts.tsv` and `versions.tsv` sizes; rerun identical |
| 4 | call-site and noexcept tables | M | 2 | `callsites.tsv` (-O0, and -O2 fortify 1..3), `noexcept.tsv` | every `__isoc*` and `*_chk` export reached by some call; rerun identical |
| 5 | generator and base layer | M | 2, 3, R-H1..3 | `headers-gen`, the `--check` in static, staging, `run.sh --liella-headers`; `features.h`, `features-time64.h`, `stdc-predef.h` (Liella's `__STDC_ISO_10646__` from its Unicode tables, `__STDC_IEC_559__`, ...), `sys/cdefs.h`, `sys/types.h`, `sys/bitypes.h`, `stdint.h`, `inttypes.h`, `limits.h`, `endian.h`, `byteswap.h`, `errno.h`, `sys/errno.h`, `alloca.h`, `gnu-versions.h`, `gnu/stubs.h` | P1, P2, P5, P9 |
| 6 | minimal C program | M | 4, 5 | `stdio.h`, `stdlib.h`, `string.h`, `strings.h`, `memory.h`, `unistd.h`, `sys/unistd.h`, `assert.h`, `ctype.h`, `getopt.h`, `time.h` | P1–P3, P5, P6 (hello and the covered conformance programs), P10 (ctype, `assert`, `errno`) |
| 7 | POSIX files and processes | M | 6 | 29 headers: `fcntl.h`, `sys/stat.h`, `dirent.h`, `sys/wait.h`, `sys/time.h`, `sys/select.h`, `poll.h`, `sys/mman.h`, `sys/ioctl.h`, `sys/xattr.h`, `spawn.h`, ... | P6; P7 for zlib, xz-utils and sqlite3; P8 for `sys/xattr.h`; P10 for `W*` and `FD_*` |
| 8 | signals, threads, contexts | M | 6 | `signal.h`, `setjmp.h`, `ucontext.h`, `sys/ucontext.h`, `pthread.h` (C.9 cleanup in its C and C++ forms), `semaphore.h`, `threads.h`, `sched.h`, `sys/single_threaded.h`, `sys/rseq.h` | P6 (the pthread and signal programs); P3 for `setjmp` and `sigsetjmp` |
| 9 | Linux process and event interfaces | M | 6 | `sys/epoll.h`, `sys/signalfd.h`, `sys/timerfd.h`, `sys/eventfd.h`, `sys/inotify.h`, `sys/fanotify.h`, `sys/prctl.h`, `sys/pidfd.h`, `sys/auxv.h`, `sys/random.h`, `sys/syscall.h`, `syscall.h`, `sys/ptrace.h`, `sys/user.h`, `sys/reg.h`, `sys/procfs.h`, `sys/debugreg.h`, `sys/personality.h` | P6; P8 (both include orders with `linux/*.h`) |
| 10 | locale, text, matching | M | 6 | `locale.h`, `langinfo.h`, `nl_types.h`, `libintl.h`, `monetary.h`, `wchar.h`, `wctype.h`, `uchar.h`, `iconv.h`, `gconv.h`, `stdbit.h`, `regex.h`, `re_comp.h`, `regexp.h`, `fnmatch.h`, `glob.h`, `wordexp.h`, `search.h`, `fts.h`, `ftw.h` | P7 for grep, sed, diffutils, findutils and gawk |
| 11 | math | M | 6, R-H5 | `math.h`, `complex.h`, `tgmath.h`, `fenv.h`, `fpu_control.h`, `ieee754.h`, `values.h`, Fortran `math-vector-fortran.h` | libm conformance programs; type-generic macros on every format; C++ `<cmath>` and `<complex>` |
| 12 | network core, name services | M | 6 | `sys/socket.h`, `sys/un.h`, `netinet/in.h`, `arpa/inet.h`, `netdb.h`, `net/if.h`, `ifaddrs.h`, `netinet/tcp.h`, `netinet/udp.h`, `resolv.h`, `arpa/nameser*.h`, `nss.h`, `aliases.h`, `rpc/netdb.h`, `sys/socketvar.h` | P8 (`libc-compat.h` in both orders); P10 for `CMSG_*` and `IN6_*` |
| 13 | network protocols, families | M | 12 | 35 headers: `netinet/ip*.h`, `icmp6.h`, `net/ethernet.h`, `net/route.h`, `netpacket/packet.h`, the 8 legacy families, `arpa/ftp.h`, `arpa/telnet.h`, `arpa/tftp.h`, `protocols/*` | P1 on the new layout rows; P8 |
| 14 | users, terminals, IPC, logging | M | 6 | `pwd.h`, `grp.h`, `shadow.h`, `gshadow.h`, `utmp.h`, `utmpx.h`, `lastlog.h`, `sys/fsuid.h`, `termios.h`, `termio.h`, `sgtty.h`, `pty.h`, `ttyent.h`, `sys/tty*.h`, `syslog.h`, `mqueue.h`, `sys/ipc.h`, `sys/msg.h`, `sys/sem.h`, `sys/shm.h`, `aio.h` | P6; P7 for coreutils and bash |
| 15 | administration, devices, legacy | S | 6, R-H7 | `mntent.h`, `fstab.h`, `sys/mount.h`, `sys/swap.h`, `sys/reboot.h`, `sys/klog.h`, `sys/sysinfo.h`, `sys/quota.h`, `sys/acct.h`, `sys/io.h`, `sys/perm.h`, `sys/vlimit.h`, `ulimit.h`, `gnu/libc-version.h`, `sys/timeb.h`, `sys/timex.h`, device ioctl headers, `scsi/*`, `nfs/nfs.h`, `sys/vm86.h` | P1, P2, P5, P8 (`sys/mount.h` with `linux/mount.h`) |
| 16 | ELF, dynamic linking, debugging | M | 6 | `dlfcn.h`, `link.h`, `elf.h` (gABI/psABI types and `constants.tsv` values), `sys/elf.h`, `a.out.h`, `ar.h`, `stab.h`, `sys/platform/x86.h`, `thread_db.h`, `proc_service.h`, `sys/gmon*.h`, `sys/profil.h`, `gnu/lib-names.h` | P1 on the ELF rows; P6 (the dlfcn, link and elf programs) |
| 17 | GNU and BSD extension APIs | M | 6 | `argp.h`, `argz.h`, `envz.h`, `obstack.h`, `printf.h`, `stdio_ext.h`, `malloc.h`, `mcheck.h`, `err.h`, `error.h`, `execinfo.h`, `fmtmsg.h`, `sysexits.h`, `sys/queue.h`, `tar.h`, `cpio.h` | P6; P10 for obstack and queue macros |
| 18 | fortify | M | 4, 6, 7, 10, 12 | `_FORTIFY_SOURCE` 1..3 from GCC's object-size builtins and C.9's `_chk` shapes | P3 at -O2 with every `*_chk` export reached; deliberate overflows abort as the spec says; P7 with `dpkg-buildflags` |
| 19 | diagnostic attributes | S | 6 | `format`, `nonnull`, `warn_unused_result`, `malloc`/`alloc_size`, `deprecated`, `noreturn`, from the standards and a warning differential | the same warnings (code and line) as the reference headers on a probe corpus with `-Wall -Wextra -Wformat=2` |
| 20 | switch and package | M | all | the full C++ check (P4); `liella-gcc`/`liella-cc` default to Liella's headers; `liella6-dev` installs them (`/usr/lib/liella/include`; SL-FLAVOUR gets `/usr/include` and the multiarch directory) | P4; P6 for all 154 programs; P7 for the whole smoke corpus; differences rows for anything accepted |

`plan/system-libc.md` §6 item 7 becomes these slices. Its item 11 (SL-BOOTSTRAP-ESSENTIAL) needs
slice 20 and, under R-H6, slice 18.

## 5. Rulings needed (owner)

| id | question | recommendation |
|---|---|---|
| R-H0 | Extend OD-1 (Appendix C) from ABI facts to the API facts of §2.2, read only through the compiler | yes. Slices 2–4 need it, and it keeps the reference's text out of every artifact |
| R-H1 | Identity macros: `__GLIBC__`, `__GLIBC_MINOR__` (the ABI level, 39), `__GNU_LIBRARY__`, `__GLIBC_PREREQ` | yes (configure checks; `plan/system-libc.md` §3.2; the kernel's `libc-compat.h` keys on `__GLIBC__`). Also add Liella's own `__LIELLA__`, a new public interface |
| R-H2 | glibc-internal names that other software tests | define exactly the names that external headers and programs test, with the same meaning: `__USE_MISC`, `__USE_GNU` and the include guards `_NET_IF_H`, `_NETINET_IN_H` and `_SYS_XATTR_H` (kernel UAPI), plus whatever SL-HEADERS-1 finds. Nothing else of the `__USE_*` family |
| R-H3 | `sys/cdefs.h` vocabulary (`__BEGIN_DECLS`, `__THROW`, `__P`, ...) and reserved type names (`__pid_t`, `__off64_t`, ...) | ship the subset that SL-HEADERS-1 finds in use, with Liella-written definitions; add more only on a build failure |
| R-H4 | FILE: complete, with the member names from `layouts.tsv`, or opaque | complete. The ABI already fixes the layout, and C.9 relies on `_flags`. An opaque FILE is cleaner but breaks programs that read members |
| R-H5 | libmvec `simd` declarations | none until M-0/M-1 (`plan/system-libc.md` item 19). Rebuilt programs then call scalar libm in vectorised loops |
| R-H6 | Fortify ordering | SL-BOOTSTRAP-ESSENTIAL waits for slice 18, so no Aqours package is built without `_FORTIFY_SOURCE` |
| R-H7 | Legacy headers ("should Liella have it at all?"): `sgtty.h`, `termio.h`, `regexp.h`, `re_comp.h`, `sys/timeb.h`, `sys/vlimit.h`, `sys/dir.h`, `sys/vm86.h`, `sys/elf.h`, `memory.h`, `values.h`, `a.out.h`, `stab.h`, the 8 legacy socket families, `nfs/nfs.h`, `sys/raw.h`, `gconv.h`, `gnu-versions.h`, `thread_db.h`, `proc_service.h` | ship them all: they cost only header text, and `--check` keeps out every name that Liella does not export at a default version |

## 6. Not verified here

* The probe configurations were not run in this planning story. The `__isoc*` and `_chk` mappings are
  inferred from the exports, not probed.
* The reference compiler is GCC 13.3, and excalibur builds with GCC 14. Header branches that depend
  on the compiler version are re-probed when the excalibur reference exists
  (`plan/system-libc.md` §4).
* Debian's default `_FORTIFY_SOURCE` level from `dpkg-buildflags` is not checked here; slice 18
  reads it.
