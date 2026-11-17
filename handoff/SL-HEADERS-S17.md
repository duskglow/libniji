# SL-HEADERS-S17: slice 17, GNU and BSD extension APIs (handoff after run 1)

**State.** Branch `task/sl-headers-s17` (on origin/next 241236c4e, slice 6 headers included). INCOMPLETE: the 16 templates exist and
`headers-gen` runs clean and writes them; P5-lite, P2, P3, P10, plants, clippy and static are NOT green or not run. Say "slice 17".

**Done.**
* `spec/tools/header_probe.py` `standard_names()`: POSIX `tar.h`/`cpio.h` names, `sysexits(3)`, the queue macros of list(3), slist(3),
  stailq(3), tailq(3), circleq(3) and the obstack macros of the manual are probe candidates now. Full rerun on the reference
  (job `sl-headers-s17-probe`, rc 0, 18 min): visibility.tsv +152 rows, macros.tsv +63 rows, nothing else changed.
* Templates `headers/{argp,argz,envz,printf,stdio_ext,malloc,mcheck,err,error,execinfo,fmtmsg,sysexits,tar,cpio}.h.in` (`@@emit`),
  `headers/obstack.h.in` (macros from the GNU C Library manual, over the `struct obstack` members of layouts.tsv),
  `headers/sys/queue.h.in` (from the five man pages; `STAILQ_LAST` left out because the reference's table does not list it).
* Generator (`xtask/src/headers.rs` `members_nested`): bit-field members, `fn(...) *` members (declared `void (*)(void)`),
  anonymous-union members, pointers to const structures, members of typedef names that do not resolve to a scalar (`FILE *`, `wchar_t`).
  These need a pending decision record (series D-X): not written.
* `libc-include/` regenerated: 16 new files, the 27 earlier ones unchanged.
* `validation/headers_p10_s17.c`: P10 program for obstack and queue macros (build it twice, compare output); not yet run clean.

**Open (first things to do).**
1. P5-lite found 8 failures (gnu17, c11): `argp.h:167`, `obstack.h:35`, `error.h:21` "expected identifier or '(' before ')'"
   (look at those lines of the generated files: a function declaration whose name is a macro or a `__asm__` spelling), and
   `printf.h:58` "unknown type name `_IO_FILE`" (typedef FILE spelled through its tag; the generator needs the tag first).
2. `struct _obstack_chunk` has no layouts.tsv rows (left incomplete: only a pointer is used); add it to the layout extraction.
3. `header_check.py` HEADERS/SLICE lists do not include slice 17: add them, run P5 (c89 -pedantic where C89 has it) and P1.
4. P2/P3: `header_probe.py --include libc-include --headers <slice 17>` and `callsite_probe.py`, compare with `probe_compare.py`.
   Expect differences to settle: obstack macros present in the table but defined here from the manual; `obstack_free` macro form.
5. P10: `bash`-free: compile `validation/headers_p10_s17.c` with the reference headers and with `-nostdinc -isystem libc-include ...`,
   compare outputs (the second build links the reference libc; ABI identical).
6. Plants (`cargo xtask plants`): suggested sites: a `queue.h.in` macro body (TAILQ_REMOVE), obstack_finish alignment, the new
   `members_nested` branches, a `standard_names()` entry; none written.
7. clippy, `cargo xtask static`, `cargo xtask test`-class checks on the final head; pending decision record for the generator additions.

**Not verified.** Everything except: probe rerun rc 0, `headers-gen` writes with no problems, 24 of 32 header-alone compiles.
