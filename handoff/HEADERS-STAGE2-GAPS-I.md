# HEADERS-STAGE2-GAPS-I: handoff (run 1, end of leg 1)

Branch task/headers-stage2-gaps-i (stacks on HEADERS-STAGE2-GAPS-H 012838ad7; 6 commits behind origin/next, no
merge done). Role shiki:~/w/sh1 (stock Ubuntu 2.39 host, gcc 13, no clang, no gcc-14). Scratch: local
`~/w/.scratch/HEADERS-STAGE2-GAPS-I/` (gfp/ = GCC-FIXINCLUDES-PROBE scripts; src-dl/ = consumer sources: fakechroot
glob64.c, systemd sort-util.h, llvm-19 sanitizer_platform_limits_posix.cpp, inetutils-2.5); shiki
`~/work/headers-stage2-gaps-i/`; keke `~/work/headers-stage2-gaps-i/` (tb64.py, the 2.41 root is GAPS-H's
`~/work/headers-stage2-gaps-h/excal/usr/include`). Decision id reserved: D-X.178 (used).

## Done in leg 1 (one WIP commit; NOTHING VERIFIED by header_check, gate or plants yet)
* Item 1 (fixincludes): va_list in stdio.h.in, wchar.h.d/020-va-list.frag, printf.h.in through
  `__need___va_list` + `<stdarg.h>` + `typedef __gnuc_va_list va_list;` (no `_VA_LIST_DEFINED`); size_t,
  ptrdiff_t, wchar_t through `__need_*` + `<stddef.h>` in the gnu_types files (typedefs.rs
  `COMPILER_STDDEF_HEADERS`). libc-include regenerated for these (headers-gen --check ok on shiki, job gen1).
* Langinfo item: fresh stock enum probe (shiki job enum1, `enum_probe.py --out`, 43118 rows); `--derive` gives
  the stock enumerator names (__CURRENCY_SYMBOL ... __INT_N_SIGN_POSN, __THOUSANDS_SEP, __ALTMON_n, __ERA_YEAR,
  __YESEXPR/__NOEXPR/__YESSTR/__NOSTR; CRNCYSTR -> _NL_MONETARY_CRNCYSTR). Only the `anon ABDAY_1` block of
  enumerators.tsv was taken (scratch take_langinfo.py); enum-macros.tsv as derived (only langinfo rows changed).
  INFORMATIONAL: the derive also regroups `anon ADDR_COMPAT_LAYOUT`/`PER_BSD` (sys/personality.h) and
  `TCP_CLOSE`/`TCP_NO_QUEUE` (netinet/tcp.h): table drift, NOT taken here.
* Item 6: features.h.in defines `__USE_TIME_BITS64 1` (undef first) in every configuration; probe (keke tb64.py):
  2.41 = 1 in 65 of 67 configs, a program's own definition replaced; time32/time64only are include errors in 2.39
  and 2.41; stock 2.39 never defines it. use-macros.tsv: 65 rows set to 1. Differences row (Behaviour, top) and
  D-X.178. libc-include/features.h NOT yet regenerated.
* Item 2/3/5 prep: `spec/tools/header_probe.d/stage2-gaps-i.py` (candidates: comparison_fn_t, talkd/rwhod/timed/
  routed names, tags termio osockaddr whod whoent outmp tsp rip netinfo); abi_extract.py TYPES (+glob64_t, struct
  igmp, struct aiocb64, fpos64_t, struct termio, CTL_MSG, CTL_RESPONSE, the protocols structs) and HEADERS
  (+termio.h netinet/igmp.h protocols/*.h). Opaque sweep result: the only `__niji_opaque` types in libc-include are
  glob64_t (glob.h), struct igmp (netinet/igmp.h), struct aiocb64 (aio.h), struct _G_fpos64_t (stdio.h, resolv.h).
* Job running at refresh: shiki `headers-stage2-gaps-i-tables1` (tables1.sh: abi_extract --layouts-only in a copy
  `~/work/headers-stage2-gaps-i/spec-copy`, then a full header_probe to `~/work/headers-stage2-gaps-i/hp`).

## Exact next step
1. `~/bin/jobwait headers-stage2-gaps-i-tables1` on shiki (one call, timeout 600000); read
   `~/work/headers-stage2-gaps-i/ae.log` (skips) and hp.log.
2. Merge: from spec-copy/abi/layouts.tsv take only the rows of the new TYPES; from hp/visibility.tsv only rows whose
   name is a stage2-gaps-i candidate (any header). Do NOT take spec-copy's enumerators.tsv wholesale (it drops the
   anon rows); keep the committed one. Then type_probe for typedefs.tsv/macros.tsv of those names
   (`type_probe.py --headers ... --out` scratch, merge the new names' rows; or `--retry-missing`).
3. Sync, headers-gen on shiki (gen.sh), fetch libc-include back (tar via ssh), check glob64_t/igmp/aiocb64/fpos64
   members, termio, comparison_fn_t, protocols names in the output.
4. header_check (`python3 spec/tools/header_check.py --work <dir>`) + `cargo test -p xtask headers`; add
   header_check fragment rows (s-slices' `.d/` files) for the new names; fixincl check on mei (built fixincl in
   `~/work/gcc-fixincludes-probe/build/fixincludes`, run_trees.sh form; goal: no "Fixed:" for libniji headers);
   unit compiles (libgomp va_list case, acinclude C99 stdio test, sort-util.h, sanitizer cpp, glob64.c, igmp,
   libstdc++ config/locale/gnu/*.cc, inetutils talk, libuuid uuid_time.c with nm uuid_time64).
5. Plants (docs/plants.md form, `cargo xtask plants`), clippy, static, gate, precheck, report json, handback-check.

## Not to redo
The fixincludes probe (GCC-FIXINCLUDES-PROBE), the enum probe, the 2.41 __USE_TIME_BITS64 probe, the opaque sweep.
