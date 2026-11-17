# SL-HEADERS-S10: slice 10, locale, text, matching (handoff after run 2)

**State.** Branch `task/sl-headers-s10-2` (continues `task/sl-headers-s10`). Say "slice 10" in commits. 20 headers generated and
committed; `header_check.py` (P5, P1, plus the inverse `regexp.h` unit) 0 failed of 2987; clippy and static clean on the final head.

**Done in run 2.** `regexp.h` (ruling #6415, premise checked): the reference fails in all 40 configurations even with INIT/GETC/PEEKC/UNGETC/RETURN/ERROR
defined, and `step`/`advance`/`loc1`/`loc2`/`locs` have no default version in either library, so libniji's header is an `#error`; probe
table in `records/pending/SL-HEADERS-S10-2-1.toml` (no differs row needed). P2 67 -> 38, P3 204 -> 60: `fnmatch.h` leaks no
feature-test macros; `stdbit.h` gnu_inline externs (address names the exported `stdc_*`) and C++ generic names as macros;
`wchar.h` `FILE` incomplete plus `wcswcs` overloads; `ftw.h` `nftw` redirect to `nftw64` under FOB64. Plants: 6 + 6 in `plants.txt`.

**P2 (38 left, not slice 10's).** 28 from `sys/types.h`/`endian.h` (earlier slices; the same 7 rows differ on those headers themselves: `be16toh`,
`le64toh` in every configuration, `__off_t`/`__ino64_t`/`__pid_t` typedef conditions, `_SIGSET_NWORDS`, `_STRUCT_TIMESPEC`) on regex.h, fts.h, ftw.h,
re_comp.h; 5 `ftw.h` rows (`statx`, `statx_timestamp`, `getumask`, `timespec`) wait for `<sys/stat.h>` (slice 7); 5 `libintl.h` fortify macro rows (slice 18).
**P3 (60 left).** All `call` rows of `wchar.h` and `libintl.h` under the fortify configurations (slice 18).

**Open.** P6 and P7 (grep, sed, diffutils, findutils, gawk) and the C++ `<cwchar>` compile were not run (budget). After slice 7: add the
`sys/stat.h` include to `ftw.h`/`fts.h` and drop the `statx` exception in `header_check.py`. Fix `endian.h`/`sys/types.h` rows in their own story.

**Tools.** scratch `run.sh <n>` on the role host (gen, header_check, P2, P3); `regexp_probe.py` in scratch.
