# HEADERS-STAGE2-GAPS-G: handoff (run 1, end of leg 1: refresh)

Branch task/headers-stage2-gaps-g from origin/next dff0cc5c1; role keke:~/w/r5. Scratch: local
`~/w/.scratch/HEADERS-STAGE2-GAPS-G/` (sweep1.py, lp.py line probe, kinc.py, splice.py, probe files f1.c f3c.c f6.c
cl.c, s2g.names); keke `~/work/headers-stage2-gaps-g/` (same scripts, `kern/` UAPI links, `niji-inc/` copy of
libc-include for probes, stock/ niji/ probe tables, cmp.out, sweep1.out, kinc.out, types1/). Run probes as
`python3 lp.py F.c ~/work/headers-stage2-gaps-g/niji-inc $PWD/kern {c|all|clang|allc}` (LPWARN=1 keeps warnings);
refresh niji-inc from ~/w/r5/libc-include after each headers-gen. No decision rows yet; black-box probes only.

## Committed in the WIP commit (NOT yet compiled: no cargo locally; first step is headers-gen on keke)
* `spec/tools/header_probe.d/stage2-gaps-g.py`: candidates (items 2, 7, tags).
* visibility.tsv +816 stock rows, macros.tsv +63, typedefs.tsv +11 (splice of the --names probe and of a full
  type_probe run, only the s2g.names names; existing rows unchanged).
* Item 3: `rename_targets` (xtask/src/headers.rs) + `Gen.renamed`; functions.rs parenthesizes the declarator
  `(__x) (...)` of each `#define x __x` target. Probe f3c.c: stock declares __b64_ntop, __b64_pton, __res_ninit,
  __res_nclose even when the program defined function-like macros of them first; libniji declared b64_ntop etc.
  (__xpg_basename: neither side declares it in that case; leave). Corner: stock errors when a program defines a
  0-argument `__res_init()` macro first, libniji will not (more permissive; INFORMATIONAL).
* Item 1: `headers/redirects.tsv` + functions.rs `redirect_line`: in the multi-symbol branch a symbol with a
  redirects row gets that declaration/attributes. Row: __xpg_strerror_r `int (int, char *, unsigned long)`,
  nonnull 2, no wur.

## Probe results (stock 2.39 vs libniji next, keke)
* strerror_r: XSI form (gnu17, posix200112+, xopen600/700, default, fortify w/o GNU) is `int`, nonnull(2), no wur
  at any level; GNU form has wur ONLY under fortify. nothrow+leaf in C everywhere on stock (libniji none in C: systemic).
* Sweep (sweep1.out, C configs, public names): type switches = strerror_r (fixed); getitimer/setitimer/getrlimit
  /setrlimit(64) in `gnu` C take the enum types (`__itimer_which_t` ...) where libniji has int; getumask (ftw.h,
  cxx/gnu); bindresvport6, _argp_input, _option_is_short undeclared in libniji. warn_unused_result: 147 functions
  have wur on stock only under _FORTIFY_SOURCE but in every configuration on libniji (diagattrs.tsv was written
  from the fortify=3 base config): a -Werror=unused-result build breaks only on libniji. Not fixed: next story
  (needs attr_probe per-config wur column, value `fortify`, and a `__NIJI_WUR` macro).
* __BYTE_ORDER __LITTLE_ENDIAN(1234) __BIG_ENDIAN(4321) __PDP_ENDIAN(3412) __FLOAT_WORD_ORDER: macros, every config,
  98 headers. __bswap_16/32/64: static (redeclarable static) FUNCTIONS, type __uintN_t(__uintN_t), result type
  __uintN_t, in endian.h-reaching headers under misc levels (20 configs), byteswap.h/arpa/inet.h all 39.
* CTRL(c) == ((c) & 037): int, one evaluation, ICE, `#if`-usable; in sys/ttydefaults.h (all configs) and 11 more
  headers. CCEQ(val, c) == ((c) == (val) && (val) != _POSIX_VDISABLE): termios.h pty.h sys/termios.h termio.h
  (20 configs), uses _POSIX_VDISABLE by name (fails without <unistd.h> on stock too).
* obstack: missing on libniji only obstack_begin, obstack_specify_allocation(_with_arg), obstack_chunkfun,
  obstack_freefun, obstack_empty_p, obstack_memory_used (macros; _obstack_begin/_obstack_begin_1/
  _obstack_memory_used exist; libniji's initial next_free = (chunk + 16 + mask) & ~mask, crates/legacy/src/obstack.rs).
* PTRACE_O_* / PTRACE_EVENT_*: ENUMERATORS in sys/ptrace.h (all 39); values now in macros.tsv; need the enum
  emitted (enumerators.tsv has them; find their tag; sys/ptrace.h.in has `@@enum __ptrace_request` only).
* Clang fortify (item 6): stock clang C at levels 2,3 (and syslog at 1) makes printf fprintf sprintf snprintf
  asprintf dprintf syslog obstack_printf function-like MACROS to the `__*_chk` route; clang C++ has no variadic
  wrappers (plain printf..., v* still __v*_chk); open family plain under clang (no mode wrapper). libniji emits
  `__builtin_va_arg_pack` under clang: C++ errors, C silently references an undefined symbol. Fix in
  xtask/src/headers/slices/s18.rs: rows with $V / variadic pass / mode forms get `!defined __clang__`; clang C gets
  `#define f(...) __f_chk (...)` from the fortify.tsv args ($V -> __VA_ARGS__).
* Kernel UAPI includes (kinc.out): stock-only closures: sys/fanotify.h -> linux/fanotify.h, netinet/if_ether.h ->
  linux/if_ether.h, netinet/if_fddi.h -> linux/if_fddi.h (FDDI_K_*, fddihdr come from there: do not emit them,
  include it; kernel-spelled.tsv mechanism), and many more (errno.h -> linux/errno.h 19 headers, linux/limits.h,
  falloc, close_range, socket/sockios, param, ppp, if_slip, stat, quota, prctl, sysinfo, rseq, atalk): fix the three
  premise ones, list the rest.
* FTS64/FTSENT64 (item 5): not started (layout rows needed: offsetof/sizeof probes vs FTS/FTSENT).

## Exact next steps
1. On keke: `cargo xtask headers-gen`; fix compile/generator errors; check resolv.h `(__b64_ntop) (` and string.h
   strerror_r XSI line.
2. headers/reserved.tsv emit rows: __BYTE_ORDER __LITTLE_ENDIAN __BIG_ENDIAN __PDP_ENDIAN __FLOAT_WORD_ORDER
   __SIGRTMIN __SIGRTMAX __toupper_l __tolower_l (reason "R-H7: Yohane stage 2 (HEADERS-STAGE2-GAPS-G): <pkg>").
3. Templates: obstack 7 macros; CTRL/CCEQ (`@@owns-macro`, `@@when`, `#ifndef` guarded, one text); __bswap_* static
   inline (endian.h, byteswap.h; providers.tsv none rows as GAPS-F did for port I/O); sys/fanotify.h,
   netinet/if_ether.h, netinet/if_fddi.h `#include <linux/...>` + kernel-spelled for their names; PTRACE enum.
4. s18.rs clang (item 6). 5. header_check fragments s<slice>.d/NN-stage2-gaps-g.py; plants file
   handoff/HEADERS-STAGE2-GAPS-G-plants.txt; packages proof (logs /srv/aqours/logs/stage2 on keke).
6. Gate: `cargo xtask affected` likely FULL (generator inputs) -> precheck + test job (cargo test -p xtask,
   header_check) as GAPS-F did.
