REPORT: Mostly gcc's: fixincludes is a legacy, text-heuristic mechanism that also rewrites stock glibc's `pthread.h`, and gcc upstream (musl), Arch and Void already switch it off. But libniji's headers set off its `stdio_va_list` sed, and libniji's own `_VA_LIST_DEFINED` guard turns that rewrite into a hard `va_list` error. A small generator change (compiler `__need_*` forms) removes all four rewrites; Aqours can add `--disable-fixincludes` on their side as well.

# GCC-FIXINCLUDES-PROBE: full report (research only; no header or code change)

Sources: gcc-14 14.2.0 as Debian trixie ships it (`gcc-14_14.2.0-19`, pool files; sha256 of orig and debian
tarballs match the `.dsc`; the `.dsc` signature was NOT VERIFIED). Everything ran on mei under jobrun; logs are
`~/logs/<label>.log` there, and outputs are in `~/work/gcc-fixincludes-probe/` (fixed trees `out-v2/<tree>/fixed`). Scripts:
`handoff/gcc-fixincludes-probe/`. Line numbers refer to the gcc-14.2.0 tree (`fixincludes/...`, `gcc/...`).

## 1. Reproduction without a gcc build (job `gcc-fixincludes-probe-trees-v2`)

Built only `libiberty` and `fixincludes/` (job `gcc-fixincludes-probe-build2`), then ran `fixinc.sh` exactly as
`gcc/Makefile.in:3432-3438` does (TARGET_MACHINE=x86_64-pc-linux-gnu, MACRO_LIST from the host gcc, default
VERBOSE=2) over each `libc6-dev`'s own `usr/include`. **"Applying X to f" is not "f rewritten"**: fixincl prints it
when a fix's tests pass, but writes a file only when the output differs (`fixincl.c:1390-1391`, `test_for_changes`
at `fixincl.c:1271`). The real effect is the "Fixed:" line, i.e. the files in `include-fixed/`.

| Tree | "Applying" (C-library headers) | Files actually rewritten |
|---|---|---|
| stock 2.39 (Ubuntu `libc6-dev 2.39-0ubuntu8.9`) | sun_malloc, glibc_stdint, pthread_incomplete_struct_argument, sysv68_string, hpux8_bogus_inlines, machine_name / io_quotes_use (multiarch sys/, bits/) | **pthread.h** |
| trixie 2.41 (`2.41-12+deb13u4`, Aqours stage 0) | same set (+ bits/sched.h, sys/epoll.h, sys/pidfd.h) | **pthread.h** |
| libniji 24.2, 25.1, 25.2, 26.1, 26.2, 26.3 (identical lists) | stdio_stdarg_h, stdio_va_list (stdio.h); stdio_va_list_clients (wchar.h); gnu_types (stdlib.h, unistd.h); pthread_incomplete_struct_argument, sun_malloc, sysv68_string | **stdio.h, wchar.h, stdlib.h, unistd.h** |

- This matches Yohane's 8 lines. Three of the 8 (`pthread_incomplete_struct_argument`, `sun_malloc`, `sysv68_string`)
  also print on stock and change nothing in libniji's files. Stock's `pthread.h` *is* rewritten, so "stock passes all
  of them, nothing is rewritten" is not right (black box: fixincl's "Fixed: pthread.h" on both stock trees).
- **Bisect:** snapshots 15 to 24.1 ship no headers in `libc6-dev` (0 `.h` files; e.g. 21.4's deb has only `usr/lib`,
  `usr/share`). 24.2 is the first that ships libniji's headers (its MANIFEST: "libniji's own C headers in both
  flavours", RELEASE-SNAPSHOT-1124D, origin/next 44304e709). Every snapshot with headers draws all four rewrites, so
  this is the first time Aqours' gcc saw libniji's headers, not a regression from a later generator change. I did not
  go down to a commit, because that needs `git log` of other people's changes, which the rules forbid. The forms in
  question are already in 24.2.
- **Aqours' stage logs (`/srv/aqours/logs/stage{0,1,2}/gcc-14_14.2.0-19.log.zst`):** stage 0 built against libc6-dev
  2.41-12+deb13u4, stage 1 against `2.39+niji20261121.4-1` (ships no headers), stage 2 against 26.3. Stage 1's
  libc-header "Applying" set is the same as trixie 2.41's, including `glibc_stdint`, whose select is the text
  "GNU C Library" (inclhack.def:2278). So stage 1's gcc was most likely fixing Debian glibc headers, not libniji's
  (NOT VERIFIED how they got there). That is why stage 1 "applied none of these".

## 2. Why each fix fires (inclhack.def test fields; test_text never read; extractor `fixfields.py` skips it)

| Fix (inclhack.def) | Test fields | libniji text that matches / misses the bypass |
|---|---|---|
| stdio_stdarg_h (4482-4498) | files stdio.h; **no select**; bypass `include.*(stdarg\.h\|machine/ansi\.h)`; mach `*-*-solaris2.1[0-9]*` with not_machine (so it runs on every non-Solaris target); c_fix wrap adding `#define __need___va_list` + `#include <stdarg.h>` | stdio.h never includes `<stdarg.h>` (headers/stdio.h.in:20-27 typedefs `__builtin_va_list` itself) |
| stdio_va_list (4508-4545) | files stdio.h; **no select**; bypass `__gnuc_va_list\|_BSD_VA_LIST_\|__DJ_va_list\|_G_va_list`; same not-Solaris mach; sed renames `typedef ... va_list;` to `__not_va_list__`, and `_VA_LIST_DEFINED` goes out unchanged (4540-4543) | stdio.h:1020-1022 `#ifndef _VA_LIST_DEFINED / #define _VA_LIST_DEFINED / typedef __builtin_va_list va_list;` and no bypass name |
| stdio_va_list_clients (4554-4594) | files incl. wchar.h; bypass as above **or** `include <stdarg\.h>\|#ifdef va_start`; same sed | wchar.h:1536-1538 (headers/wchar.h.d/020-va-list.frag), the same block |
| gnu_types (2332-2350) | files sys/types.h, stdlib.h, sys/stdtypes.h, stddef.h, memory.h, unistd.h; select `^[ \t]*typedef[ \t]+.*[ \t](ptrdiff\|size\|wchar)_t;`; bypass `_GCC_(PTRDIFF\|SIZE\|WCHAR)_T`; mach Solaris 10+ excluded | stdlib.h:81 and unistd.h:270 `typedef unsigned long size_t;`, stdlib.h:88 `typedef __WCHAR_TYPE__ wchar_t;` (the generator's guarded typedef, xtask/src/headers.rs:654) |
| pthread_incomplete_struct_argument (3806-3816) | files pthread.h; select `struct __jmp_buf_tag`; c_fix format on `^extern int __sigsetjmp (struct __jmp_buf_tag __env[1]...` | select matches; no such `__sigsetjmp` line, so nothing changes (stock: changed) |
| sun_malloc (4771-4788) | files malloc.h; **no select**; bypass `_CLASSIC_ANSI_TYPES`; sed on K&R `char *malloc` forms | no bypass text, sed matches nothing: no change (stock too) |
| sysv68_string (4943-4980) | files string.h, testing.h; **no select**; bypass `_CLASSIC_ANSI_TYPES`; SysV68 sed | as above: no change (stock too) |

No `mach` field limits any of the 8 to other systems. Names like `sun_*` and `sysv68_*` are only history: with no
`select` and no `mach`, these fixes test every `malloc.h`/`string.h` on every target.

**How the rewrite breaks `va_list` (job `gcc-fixincludes-probe-compile2`, 80 compiles, gcc-14's own ginclude
stdarg.h/stddef.h first, as in a built gcc):** gcc's `<stdarg.h>` skips its `typedef __gnuc_va_list va_list;` when
`_VA_LIST_DEFINED` is defined ("used in Windows NT 3.5", gcc/ginclude/stdarg.h:95-103). The fixed stdio.h still
defines `_VA_LIST_DEFINED` but no longer typedefs `va_list`. So `#include <stdio.h>` then `<stdarg.h>` then `va_list
x;` fails: "unknown type name 'va_list'" (gnu17, C++98, C++11; also through wchar.h). That is libgomp's error, and
the same as libstdc++'s "ISO C99 support in <stdio.h>" test (libstdc++-v3/acinclude.m4:988-1005: stdio.h,
stdarg.h, `va_list args`). As shipped, the same probes pass (19/20; the one failure is strict C99 not exposing
`va_list` from stdio.h, the same in every column). The gnu_types rewrites are type-equivalent (`__SIZE_TYPE__`), but
they freeze copies of stdlib.h/unistd.h into include-fixed, which then hide later libniji updates wherever
include-fixed is kept.

## 3. Whose problem it is

- **When gcc runs it:** `gcc/configure.ac:2559` sets `STMP_FIXINC=stmp-fixinc` by default, and `:2570-2572` empties
  it when `enable_fixincludes=no` (`--disable-fixincludes`). There is no AC_ARG_ENABLE, and the option is not in
  gcc.gnu.org/install/configure.html (fetched 11-26; its only fixincludes mentions are under --with-local-prefix and
  --with-headers). `fixincludes/mkfixinc.sh:13-31` installs a no-op `fixinc.sh` for cygwin, mingw, some powerpc
  eabi/rtems, vxworks7 and **`*-musl*` (line 23)**. Every other target, including any `*-linux-gnu`, runs it.
  `limits.h`/`syslimits.h` are installed by `stmp-int-hdrs` whether or not fixincludes runs
  (gcc/Makefile.in:3277, 3340-3348).
- **Debian gcc-14 (14.2.0-19):** runs fixincludes during the build (no `--disable-fixincludes` anywhere in debian/;
  the only patch touching fixincludes/ is cross-install-location.diff). It then deletes `include-fixed` from the
  package: `rm -rf $(d)/$(gcc_lib_dir)/include-fixed` in the main install target (debian/rules2:2696; also
  2542, 2602). That has been in place since gcc-10 (debian/changelog: "Move limits.h and syslimits.h into
  <gcc-lib-dir>/include, and remove <gcc-lib-dir>/include-fixed", 10-20200321-1). So the installed gcc is clean,
  but the in-tree build compiler uses the fixed headers. libgomp fails, and libstdc++ is configured without C99
  stdio, which then ships in its `c++config.h`.
- **Other distributions** (packaging files fetched 11-26, job `gcc-fixincludes-probe-distro`):
  - Arch (glibc) configures with `--disable-fixincludes` (PKGBUILD:139).
  - Void (glibc and musl) runs `sed -i 's@\./fixinc\.sh@-c true@' gcc/Makefile.in` with the comment "Do not run
    fixincludes" (srcpkgs/gcc/template:250-251).
  - Fedora (glibc) runs fixincludes, then deletes every installed header marked "auto-edited by fixincludes", after
    printing its diff (gcc.spec:3222-3228).
  - Alpine (musl) has nothing in its APKBUILD; upstream's `*-musl*` no-op covers it.
  - Buildroot (gcc.mk, gcc-final.mk) and crosstool-ng (cc/gcc.sh) do nothing, so uClibc-ng targets run fixincludes
    there.
  - How uClibc-ng and bionic shape their headers: NOT VERIFIED (not looked at). Gentoo: fetch failed (502), NOT
    VERIFIED.
- **gcc's own description:** fixincludes/README speaks of headers "broken by the manufacturer"; the fixes target
  SunOS, SysV68, HP-UX, SCO, NT and Ultrix headers. gccint "Header Dirs" says fixed headers live in GCC_INCLUDE_DIR
  and that cross headers "already suitable for GCC" need nothing special. I found no gcc text saying a modern,
  standard-conforming C library needs fixincludes. Upstream's musl exemption, and Arch and Void turning it off,
  point to a legacy measure.

## 4. Options (cost, owner)

1. **libniji: get these types from the compiler's own headers.** Owner: libniji, in the header generator. Tested
   (`variant.py`, job `gcc-fixincludes-probe-compile2`):
   - va_list in stdio.h and wchar.h: `#define __need___va_list` / `#include <stdarg.h>` /
     `typedef __gnuc_va_list va_list;` instead of the `_VA_LIST_DEFINED` block (headers/stdio.h.in:20-27,
     headers/wchar.h.d/020-va-list.frag, and printf.h.in:14-16 for consistency).
   - size_t (stdlib.h, unistd.h, and sys/types.h, which `gnu_types` would hit on a non-multiarch layout: its
     multiarch copy matches the select) and wchar_t (stdlib.h): `#define __need_size_t` / `__need_wchar_t` and
     `#include <stddef.h>`.

   Result: fixincl rewrites **0** files, and all 20 probes per column give the same results as shipped. This is
   plain C through the compiler's interface: gcc's stdarg.h defines `__gnuc_va_list` and honours
   `__need___va_list`, and stddef.h honours `__need_size_t`/`__need_wchar_t`. clang 19's headers support all three
   (/usr/lib/llvm-19/lib/clang/19/include/stddef.h:24-39, stdarg.h:23-35). It is not copied glibc text (stock's side
   is black box only). The honest caveat: it passes `stdio_*` partly *because* the bypass regexes look for exactly
   this text.
   - Cost: libniji's headers then depend on the compiler's freestanding headers for three types (gcc and clang:
     yes; other compilers NOT VERIFIED).
   - Smallest step: drop only the `_VA_LIST_DEFINED` guard. Then a fixed stdio.h no longer kills `<stdarg.h>`'s
     `va_list`, but `stdio.h` alone stops providing `va_list` under fixincludes. NOT TESTED separately.
2. **Aqours: no fixincludes for their gcc.** Add `--disable-fixincludes` to CONFARGS (debian/rules2:263), as Arch
   does, or use Void's one-line sed. Owner: Aqours, a one-line packaging delta per gcc version they build.
   - Cost: stock's `pthread.h` fix is also skipped (Arch ships that way).
   - It does not help anyone else who builds gcc against libniji.
3. **Upstream gcc**, e.g. `__builtin_va_list` added to the `stdio_va_list`/`_clients` bypass, a `select` for
   `sun_malloc`/`sysv68_string`, or a configure default that skips fixincludes for glibc-ABI targets. Owner: gcc.
   Unlikely to arrive soon, and only in a future release (gcc 16+); gcc-14 in trixie would still need option 2.
   Likelihood NOT VERIFIED.

**Recommendation (the owner rules):** option 1 in libniji. Every gcc built against libniji, on any distribution or in
cross builds, runs fixincludes, and the hard error comes from libniji's own guard choice. Option 2 at Aqours as
cheap belt-and-braces. Option 3 is not worth waiting for. Add Yohane's quick check (fixincl over the installed tree,
empty result) to the header probes.
