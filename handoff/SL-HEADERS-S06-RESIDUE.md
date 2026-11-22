# SL-HEADERS-S06-RESIDUE: slice 6 macros seen through other headers (handoff, run 1)

**State.** Branch `task/sl-headers-s06-residue` from origin/next ae9ea6879 (no merge). Commits: 3efa15ab1 (templates +
regenerated `libc-include/ctype.h`, `libc-include/stdio.h`), e534c3025 (P2 fixed-rows test, plants file), then this note
and `handoff/reports/SL-HEADERS-S06-RESIDUE.json`. Say "slice 6". Role mei:~/w/s2.

**Premise checked.** S17-3's rows were not "through an include" only: the same rows print for `ctype.h` and `stdio.h`
alone. Reproduced with P2 (`header_probe.py --include libc-include --headers ctype.h,stdio.h,argp.h,malloc.h,stdio_ext.h`,
then `probe_compare.py visibility --accept validation/headers_s17.accept`): 40 unaccepted of 40 differences in 1195 rows
(job `sl-headers-s06-residue-p2a`). `header_check.py` had 0 failures before and after: it does not compile template-owned
macros, and neither does the generator's rows test (it only counts them).

**Causes (all in slice 6 templates; no generator change, no table change).**
1. `_tolower _toupper isascii toascii` sat inside ctype.h.in's `#ifndef __cplusplus` block; visibility.tsv has them as
   macros in every C++ configuration too. Moved outside, same `@@when`.
2. The 12 `isX_l` macros were `@@owns-macro` but never defined. Now `# define isalnum_l(c, l) isalnum_l ((c), (l))`:
   the call is the exported function, so behaviour for any locale argument (null, LC_GLOBAL_LOCALE) is the function's.
3. `tolower_l` / `toupper_l` (ctype.h) and `fwrite_unlocked` (stdio.h) are macros only in the fortify configurations;
   owned, never defined. Now `__tolower_l` / `__toupper_l` and `fwrite_unlocked` calls, the symbols callsites.tsv records.

**After.** P2 on the 5 headers: 0 differences in 1195 rows. P3 (`callsite_probe.py`, ctype.h + stdio.h): 86 -> 82
differences, none new (the 4 tolower_l/toupper_l call rows are fixed). `header_check.py --include libc-include`: 0 failed
of 13677; `headers-gen --check` ok (job `sl-headers-s06-residue-p23b`). Clippy, static (`sl-headers-s06-residue-lint`).
`xtask/tests/header_p2_fixed_rows.rs` now probes ctype.h and stdio.h from the templates as they are (`headers-gen --out`).

**Jobs.** Plants `sl-headers-s06-residue-plants` (`handoff/SL-HEADERS-S06-RESIDUE-plants.txt`): 3 plants, 3 CAUGHT with
their msg, 0 WARN. First gate `sl-headers-s06-residue-gate` (direct; FULL is the train's: generated-table inputs changed):
597 passed, 2 failed, both `xtask --test transcripts` (iconv): the role had no `target/niji/` products but kept an older
`target/niji-test/iconv` whose ELF interpreter `target/niji/ld-linux-x86-64.so.2` was gone (exec ENOENT). After `cargo
xtask so` the suite passed (`sl-headers-s06-residue-transcripts`, 6 passed); the gate was rerun on the final head
(`sl-headers-s06-residue-gate2`). INFORMATIONAL: `iconv_test_link()` in xtask/tests/transcripts.rs checks only that the
link is a file; a link whose interpreter is missing should skip with a reason, or `so` should not leave it behind.

**Open / INFORMATIONAL.** P3 still shows the 12 `isX_l` fortify `call` rows (the table records no symbol for a call: an
inline table read). That is slice 18's (fortify) or a performance story's; an inline form would have to keep a null /
LC_GLOBAL_LOCALE argument safe. The other 78 P3 rows of ctype.h/stdio.h were there before and are not classified here.
**Not to redo.** A `spec/tools/header_checks/s06.py` macro check was drafted and dropped: it duplicates P2 and a template
plant cannot reach `header_check.py` (the plants tool's product build stops on a stale `libc-include/` first).
**Next step.** Land (FULL gate is the train's). **Plan rows to read.** plan/headers.md 2.4, 3 (P2, P3), 4.1.
