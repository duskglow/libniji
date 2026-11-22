# SL-HEADERS-S18 handoff: slice 18, fortify (`_FORTIFY_SOURCE` 1..3)

**State:** branch task/sl-headers-s18 (from origin/next f15d8264f), role keke:~/w/n1. Leg 1 of run 1 (generate) done up to
the tables; no generator code or template change yet.

**Rulings in force**
* #7599 (coordinator, 11-22): slice 18 owns the 171 fortify rows of P3 (85 names) and every `*_chk` export reached.
  The 55 other -O2 rows (27 names; list below) are NOT slice 18's: hand-back lists them with their reference targets;
  they become HEADERS-O2-INLINES. Adding `*_chk` prototype rows by probe is fine (table data).
* Slice contract (plan/headers.md 4.1): no edit of `xtask/src/headers.rs`, `xtask/src/headers/*.rs` outside `slices/`,
  `spec/tools/header_check.py`. Slice files: `xtask/src/headers/slices/s18.rs` (marker), `headers/slices/s18.tsv`
  only if a new template, `spec/tools/header_checks/s18.py` (own units), template lines `@@fortify`.

**Done (commits on the branch)**
* `spec/tools/abi_extract.py --prototypes-fortify` -> `spec/abi/chk-prototypes.tsv` (71 rows; run on keke, the
  reference system, job sl-headers-s18-chkprotos). A table of its own, not prototypes.tsv, so noexcept.tsv /
  callsites.tsv / the rows test keyed on prototypes.tsv keep their contents (ruling allowed prototypes.tsv; this is
  the narrower choice: say so in the hand-back). The 10 str/mem builtins and `__longjmp_chk` get no row (GCC
  builtins; longjmp is a redirect).
* `headers/fortify.tsv`: libniji's wrapper design per name (form builtin/chk/va/redirect/macro, `_chk` args with
  $N/$SN/$EN/$F/$V tokens, `safe` condition, object-size type). Header comment documents the tokens. Units
  checked against libniji's own `_chk` implementations (crates/boundary/src/posix/s04,s06,s11,s12,s17: destlen in
  wchar_t units for wide string/mem fns, bytes for poll/ppoll/wcrtomb/wcs*tombs; getgroups listlen: confirm bytes
  in `conv::groups_chk`).

**Exact next step: write `xtask/src/headers/slices/s18.rs`**, marker `@@fortify` (no args: the header's own rows):
1. Rows of fortify.tsv whose name the header declares (callsites.tsv `header` column == g.header; also aliases:
   sys/poll.h vs poll.h, sys/syslog.h vs syslog.h: the callsites header is sys/poll.h, sys/syslog.h).
2. Levels and symbol from callsites.tsv call/macro rows with `_chk` in symbol: config ids `fortifyN[+gnu|+gnu64]`
   -> set of N (expect contiguous to 3: `#if __NIJI_FORTIFY >= Lmin`); pread: `__pread64_chk` under
   `__NIJI_FOB64`, else `__pread_chk`. vprintf -> `__vfprintf_chk` (+stdout).
3. Once per header, guarded `#ifndef __NIJI_FORTIFY_DEFS`: `__NIJI_FORTIFY_FN` = `extern __inline __attribute__
   ((__always_inline__, __gnu_inline__, __artificial__))`; `__NIJI_BOS(p, t)` = level 3
   `__builtin_dynamic_object_size (p, t)` else `__builtin_object_size (p, t)`; `__NIJI_FORTIFY_FLAG` =
   `(__NIJI_FORTIFY > 1)`. $SN = `__NIJI_BOS ($N, T)` with T = 0 at level 1, `ost` at levels 2+; $EN = $SN /
   sizeof (*$N).
4. `_chk` declaration: from chk-prototypes.tsv, resolved like `Gen::declaration` (resolve(), typedef(); helpers
   are visible to the slice module as a descendant of `headers`). `__NIJI_NOTHROW` iff the wrapped name has it.
5. Wrapper: params named `__p0..` from `g.declaration(name)` (split on top-level commas); same exception spec as
   the declaration (C++ redeclaration). chk form: `if (S == (size_t) -1 || (__builtin_constant_p (safe) && (safe)))
   return name (params); return __x_chk (args);` va form uses `__builtin_va_arg_pack ()`; builtin form returns
   the builtin. Under `#if __NIJI_FORTIFY >= L` after the declarations (marker line placed after `@@emit`).
   redirect: `extern void longjmp (...) __asm__ ("__longjmp_chk")` under fortify (declaration order: check the
   function emitter's guard `__NIJI_D_<name>`; the redirect may need to be a second declaration with asm label
   before first use: GCC rejects an asm label after use; put it in place of the plain one if needed).
   macro: sys/select.h FD_* index through `__fdelt_chk (d)` under fortify (template edit in sys/select.h.in).
6. Template lines: `@@fortify` in string.h, strings.h, stdio.h, wchar.h, unistd.h, stdlib.h, sys/socket.h,
   sys/poll.h, sys/syslog.h, setjmp.h, sys/select.h templates; `cargo xtask headers-gen`.
7. P3: `~/work/sl-headers-s18/p3.sh <dir>` on keke (callsite_probe --include libc-include on those headers +
   probe_compare): fortify lines must be 0 (`python3 cat.py` in scratch splits fortify vs other; baseline
   `base`: 171 fortify / 55 other). Then P1/P5 (`header_check.py --include libc-include`), headers-gen --check,
   deliberate-overflow test (abort through `__chk_fail`, P10-style test in s18.rs or header_checks/s18.py),
   plants (docs/plants.md: one wrapper removed; one size argument off by one), P7 with dpkg-buildflags flags.

**dpkg-buildflags (brief: record Debian's default level):** keke (Ubuntu 24.04) gives `-D_FORTIFY_SOURCE=3`
(CPPFLAGS) and `-O2 ... -fstack-protector-strong`; that is Ubuntu's, not Devuan's. The excalibur chroot
(`~/work/sl-headers-s18/buildflags.sh`, mmdebstrap --mode=unshare from the LAN mirror) failed 3 times with the same
"InRelease is not signed / NO_PUBKEY B3982868D104092C" (keyrings tried: ~/keyrings/devuan-archive-keyring.gpg, the
.asc, a dearmored copy of the .asc). Next: read how tools/deb/lint.sh/proof.sh stage KEYRING (`$stage/keyring.gpg`)
and the mirror path (MIRRORS.md: maybe `.../devuan/merged`); do not retry blind.

**Non-fortify -O2 rows (HEADERS-O2-INLINES, 55 lines, 27 names)** from baseline compare (keke
~/work/sl-headers-s18/base/compare.txt): stdio.h feof_unlocked ferror_unlocked fgetc_unlocked fputc_unlocked
getc_unlocked getchar getchar_unlocked getline putc_unlocked putchar putchar_unlocked; stdlib.h abs atof atoi atol
atoll bsearch div; string.h basename memrchr rawmemchr strcasestr strchrnul; strings.h index rindex; wchar.h mbrlen.
Reference targets: callsites.tsv call rows of those names (e.g. getc_unlocked -> __uflow, getchar -> getc,stdin,
getline -> __getdelim (gnu), atoi -> strtol, atof -> strtod, bsearch -> -).

**Jobs so far (keke):** sl-headers-s18-p3base (baseline P3), sl-headers-s18-chkprotos (table), buildflags 1-3
(failed, see above). None prove the final head.
