# SL-HEADERS-S17: slice 17, GNU and BSD extension APIs (handoff after run 2, story SL-HEADERS-S17-2)

**State.** Branch `task/sl-headers-s17-2` (stacked on `task/sl-headers-s17`, base origin/next 241236c4e). Slice 17 headers are generated
and compile alone: `header_check.py` 0 failed of 2555 units (P5 and P1, 40 configurations, slice 17 headers now in its lists). clippy and
`cargo xtask static` clean on the head. P10 identical (-O0, -O2). P2 and P3 are NOT green (below). Say "slice 17".

**Run 2: causes of the 8 header-alone failures (all fixed in the generator or a template, none worked around).**
* `obstack_alloc_failed_handler` / `error_print_progname` (function-pointer objects) were spelled `void ( *)() name`: `object()` now puts the name inside `(*name)`.
* `struct argp_state` / `_IO_FILE` / `enum mcheck_status` first named inside a parameter list: `type_words` declares the tag first; mcheck.h.in defines its enum before `@@emit`.
* `error_t` missing from argz.h/envz.h outside the GNU configurations: `covered` (names an included header provides) ignored configurations. It now needs the
  including header's configurations to be a subset. Side effect: sys/types.h now also emits `__ino64_t`, `__off_t`, `__pid_t` where the table lists them (guarded, harmless; check in the landing gate).
* printf.h: `va_list` is visible in every configuration per the table; printf.h.in owns it. `__va_list_tag (*) [1]` rewrite ran before `( *)` was normalised.
* `header_check.py`: `LONG_MIN`-style literal, `_LIBC_LIMITS_H_` guard names and `PTHREAD_STACK_MIN` (a sysconf call in the GNU configurations) cannot be value-asserted: tool fixed.

**P2/P3 (probe_compare, 16 headers) after the fixes: 65 P2 rows and 12 P3 rows differ.** By cause:
* inherited from slice 6, not slice 17: ctype `*_l`/`_tolower`/`isascii` macro configurations and `fwrite_unlocked` macro in the fortify configurations (seen through argp.h, malloc.h, stdio_ext.h).
* `_argp_input`, `_option_is_short`, `_option_is_end` (reference-internal inline helpers of argp.h), and the inline forms of `argp_usage` (calls `argp_state_help` and `stderr`) and
  `argz_next` (calls `strlen`): libniji calls the exported function. RULING NEEDED, see report.
* `struct _obstack_chunk`: no layouts.tsv rows (abi_extract.py lists no such type): add it to the extraction list and rerun; until then the tag is not defined.
* tar.h: the table has an include-error row in the time64-only configuration, so tar.h keeps features.h (cpio.h, sysexits.h, sys/queue.h do not).
* `extra struct _IO_FILE` in printf.h: libniji defines the whole FILE structure where the table shows only the typedef. Harmless, to settle.

**Plants (`validation/headers_s17.plants`, 8).** 1 CAUGHT by its message (header_check.py), 7 WARN-NO-MSG: a mutant of a template or of the generator fails the headers-gen staleness check of
`cargo xtask so` first (`stale under libc-include/`), so the plant's own command (header_check.py, P10) never runs on it. The msg regexes name what that command prints on a regenerated tree.

**Decisions in force.** Pending record `records/pending/SL-HEADERS-S17-2-1.toml` (D-X, generator additions).
**Not to redo.** probe rerun for slice 17 candidates (visibility.tsv/macros.tsv rows already in the tables); header-alone compile fixes; P10.
**Next step.** (1) `_obstack_chunk` layout rows; (2) ruling on the argp/argz inline forms, then accept file for `probe_compare.py --accept` or implement; (3) look at the slice 6
ctype/stdio P2 residue (shared with slice 6); (4) P6.
**Run it.** `python3 spec/tools/header_check.py`; `cargo xtask headers-gen --check`; `sh validation/headers_p10_s17.sh`; plants `validation/headers_s17.plants`
(`cargo xtask plants validation/headers_s17.plants --dir <new RAM dir>`); P2/P3 as in handoff/SL-HEADERS-S06.md.
**Plan rows to read.** plan/headers.md sections 2.4, 3, 4 (slice 17).
