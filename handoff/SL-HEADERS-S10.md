# SL-HEADERS-S10: slice 10, locale, text, matching (handoff after run 1)

**State.** Branch `task/sl-headers-s10`. INCOMPLETE (context checkpoint). Say "slice 10" in commits and handoffs.
The 20 headers are generated and committed (`headers/<name>.in`, `libc-include/`): `locale.h langinfo.h nl_types.h libintl.h
monetary.h wchar.h wctype.h uchar.h iconv.h gconv.h stdbit.h regex.h re_comp.h regexp.h fnmatch.h glob.h wordexp.h search.h
fts.h ftw.h` (the three legacy ones, `gconv.h`, `re_comp.h`, `regexp.h`, are shipped: plan/headers.md section 5.1 lists them).
`headers-gen` clean; `header_check.py` (P5, P1, now covering the 20 headers): 0 failed of 2987 units.

**Done.**
* Templates: mostly `@@emit` only. `wchar.h` has the [cwchar.syn] overloads of the five search functions, `NULL`, `va_list`;
  `locale.h` owns `LC_GLOBAL_LOCALE` (`((locale_t) -1L)`, from crates/boundary/src/locale.rs); `fnmatch.h` owns `FNM_NOSYS`
  (`(-1)`, value read from the probe's own object under `_XOPEN_SOURCE`); `uchar.h` owns `char8_t`/`char16_t`/`char32_t`;
  `stdbit.h` is written from C23 7.18 (70 static inline functions over the compiler's popcount/clz/ctz builtins, 14
  type-generic macros via `_Generic` in C and overloads in C++, `__STDC_ENDIAN_*`).
* Generator: bit-field members in `members_nested` (`regex_t`); `features.h` skips the `_TIME_BITS` check when
  `__NIJI_TIME_BITS_UNCHECKED` is defined (only `fnmatch.h` does: the probe table says it compiles under `_TIME_BITS=64` alone).
* Plants: `handoff/SL-HEADERS-S10/plants.txt` (6 plants; run through `cargo xtask plants`).

**Open.**
* P2 (`header_probe.py` + `probe_compare.py visibility`): 67 differences of 1594 rows on the last tree, not classified. Seen:
  feature-test macros (`_DEFAULT_SOURCE`, `_ATFILE_SOURCE`, ...) defined in fewer configurations by `features.h`;
  `regexp.h`: the probe table says the reference header fails to compile in every configuration (`include-error *`), libniji's compiles
  (RULING NEEDED, below); `fts.h`/`regex.h` lack `_SIGSET_NWORDS`, `_STRUCT_TIMESPEC` (reserved include-guard names);
  `wchar.h` shows `struct _IO_FILE` (not in the table).
* P3 (`callsite_probe.py` + `probe_compare.py callsites`): 204 differences of 641 rows, not classified (`call` rows under
  fortify are slice 18's).
* P6, P7 (grep, sed, diffutils, findutils, gawk), the C++ `<cwchar>` compile, P9 on the P2/P3 checkers: not run.
* `ftw.h` and `fts.h` do not include `<sys/stat.h>` / `<sys/select.h>` yet (slice 7 and the `FD_*` owner are separate): `ftw.h`
  owns `statx`, `statx_timestamp`, `getumask` (skipped), and `header_check.py` P1 skips `statx*` for it. After slice 7 lands, add
  the include and drop those exceptions.
* Pending decision record for the generator additions (bit-field members, `__NIJI_TIME_BITS_UNCHECKED`) not written:
  `cargo xtask records new decision D-NEW --series D-X --story SL-HEADERS-S10`.
* `libintl.h`: `dgettext`/`gettext`... macros (fortify rows) are left to slice 18 (`@@owns-macro`).

**Tools.** `$S/work/SL-HEADERS-S10/{mk.py,mk_stdbit.py,run.sh}` generate the templates and run gen, check, P2, P3 (run.sh on the
role: `bash ~/work/SL-HEADERS-S10/run.sh <n>`; output under the role host's RAM dir `run-<n>/p2.txt`, `p3.txt`). The templates
are committed; the scripts are scratch.

**Next steps.** Classify the 67 P2 and 204 P3 rows (fix a template, or a differences row plus a compat option), run P6/P7, the
pending record, plants on the checkers, clippy and static.
