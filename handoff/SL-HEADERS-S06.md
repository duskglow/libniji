# SL-HEADERS-S06: slice 6, minimal C program (handoff after run 2, SL-HEADERS-S06-2)

**State.** Branch `task/sl-headers-s06-2` (continues `task/sl-headers-s06`). Head: see `handoff/reports/SL-HEADERS-S06-2.json`.
`headers-gen` is clean (27 headers written, `libc-include/` committed), `header_check.py` P5 and P1 are green
(0 failed of 1502 units over 40 configurations), clippy and static are clean. INCOMPLETE: P2, P3 and P6 are not green.
Say "slice 6" in commits and handoffs.

**Done (run 2).**
* Level `cxx` has four values (C 0, C++98 1, C++11 2, C++14 and later 3; `levels.tsv`, `features.h.in`, `LEVEL_COLS`
  thresholds 1 2 3): the `__isoc99_`/`__isoc23_` scanf conditions and `gets` (C89/C99, C++98/11) are expressible.
* Generator (`xtask/src/headers.rs`): nested-structure member rows (`it_interval.tv_sec`) skipped, pointer-to-tag members,
  function-typedef members (`cookie_read_function_t *`), trailing padding as `int : 32;` (`struct timex`), `struct _IO_FILE`
  defined from the `FILE` layout rows, function and function-pointer typedefs (`__compar_fn_t`, `cookie_*_function_t`),
  `wchar_t` in C (`__WCHAR_TYPE__`), the decayed `va_list`, structures named for linkage (`fpos_t` -> `struct _G_fpos_t`),
  an include right after `@@when` no longer counts as covering names (stdlib.h's size_t was missing in strict modes).
* Templates: `assert.h` declares `__assert_fail`, `__assert_perror_fail`, `__assert`; `stdio.h` declares `gets`; `stdlib.h`
  owns the macros `alloca be16toh le64toh` (they come from the conditional includes).
* Tools: `header_check.py` P5 over the slice 6 headers and P1 (macro integer values, typedef and structure sizes and
  alignments of the headers' visible names, per configuration; reserved `__x` names and include guards skipped);
  `header_probe.py --include DIR` and `callsite_probe.py --include DIR [--headers a.h,b.h]` (P2, P3 on libniji's headers);
  `spec/tools/probe_compare.py visibility|callsites --got DIR --headers ...` prints the rows that differ from the tables.
* Plants: `handoff/SL-HEADERS-S06/plants.txt` (7 plants).

**Open (measured on the first tree of run 2, before the last generator fixes; rerun `bash`-free as below).**
* P2 (`header_probe.py --include libc-include --headers <slice> --out D --jobs 16`, then `probe_compare.py visibility`): 455
  differences of 3034 rows, most reserved `__x` names and guards (now ignored by the compare script). The remaining real
  ones to settle: `_tolower/_toupper/isascii/toascii` macro configurations, `ispunct_l` (the `*_l` ctype macros are
  not defined), `WIFCONTINUED` configurations, `__STDC_ISO_10646__` extra in every header (has a differences row; the
  compare needs an `--accept` file for it).
* P3 (`callsite_probe.py --include ... --headers ...`, `probe_compare.py callsites`): 250 differences of 1110 rows, not
  yet classified (the `call` rows in fortify configurations are slice 18's; the `getc`/`putc` inline forms need a row or an
  implementation).
* P6 not run (`cargo xtask so`, `conformance/run.sh --niji-headers`); P10 and the C++ `<cstdio>`... compiles not run.
* Pending record for the generator additions (owns-macro, enumerator kind, fortify fallback, `LINKED_ELSEWHERE`, cxx level
  values) not written: `records/pending/SL-HEADERS-S06-1.toml`, `id = "D-NEW"`, series D-X.
* The `*_l` ctype macros and the `getc_unlocked` family inline forms (see run 1 notes) are still open.

**Next steps.** Rerun P2/P3 on the final tree; classify differences (fix the template or add a differences row with a
compat option); `cargo xtask so` then `conformance/run.sh --niji-headers` (P6); P10; the pending record.
