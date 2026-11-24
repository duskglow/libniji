# HEADERS-S16-FOLLOWUP: slice 16 leftovers (sys/elf.h must_fail, aligned vectors, P2/P6)

Branch `task/headers-s16-followup` (from origin/next). Role shiki:~/w/sh2. Code commit a6dca838a; head: see the report json.

**Done.**
1. Ruling #7519: `<sys/elf.h>` uses the `must_fail` form (`header_checks/s16.py` `must_fail`, its "P5 ... must fail" unit;
   `s16.tsv` row p1 0; the template's `#error` says "is not provided"). The `p1` reject value, its P5 unit and its doc
   line are removed from the shared `spec/tools/header_check.py`; nothing else in that file changed.
2. Ruling #7520: a `typedefs.tsv` vector row (`cxx` `T __vector(N)`) whose align is below its size is generated as
   `vector_size` + `__aligned__ (align)` (`xtask/src/headers/typedefs.rs`). Rows: `La_x86_64_ymm` = `float __vector(8)`
   (32/16), `La_x86_64_zmm` = `double __vector(8)` (64/16), from a black-box probe of the reference (own probe function
   linkage names via nm -C; sizeof/_Alignof/types_compatible in C). Layout probe on libniji's link.h: identical in
   gnu17/c99/c11 (sizes, alignments, vector compatibility, element type, offsets inside a structure, La_x86_64_vector
   64/16, retval 240/16, regs 768/16) and C++ linkage names identical. Generator test
   `a_vector_typedef_is_emitted_as_a_vector` gains an aligned row. D-X.163 (amends D-X.130 (3)); PROVENANCE line.
3. #7521 recorded in plan/headers.md (after the slice table) for the sys/types.h slice.
4. header_check.py: 0 failed of 16192. headers-gen --check ok. clippy, static, records check rc 0.

**P2 / P6 results (run, NOT fixed here: budget).**
* P6 `run.sh --niji-headers s27_dl*` (8 programs): 6 passed, 2 do not build:
  `s27_dladdr` (`RTLD_DL_LINKMAP`, `RTLD_DI_LINKMAP` from <dlfcn.h>; `ELFMAG` from <elf.h>) and `s27_dlinfo`
  (`RTLD_DI_LINKMAP`, `RTLD_DI_PHDR`, `RTLD_DI_SERINFOSIZE`, ...). These are slice 16 rows (dlfcn.h / elf.h).
* P2 `header_probe.py --include libc-include` on the 10 slice-16 headers + `probe_compare.py visibility`: 5 differences
  in 6825 rows: (a) `link.h La_x86_64_vector typedef`: table `!time64only`, libniji also in cxx98..cxx23 (the table's
  `La_x86_64_regs` row compares equal; check whether D-X.130 (5)'s C++ difference should extend to the union or the
  generator should withhold it in C++); (b) `sys/profil.h FD_CLR/FD_ISSET/FD_SET/FD_ZERO macro`: table `!time64only`,
  libniji only gnu17..cxx23 (not the strict C/POSIX levels): comes through <sys/time.h> / <sys/select.h> (slice 7), a
  finding for that slice unless sys/profil.h's include is the cause.

**Exact next step.** A follow-up story: add the missing `RTLD_DI_*` / `RTLD_DL_*` values to dlfcn.h and `ELFMAG` to elf.h
(check macros.tsv / constants.tsv rows and their visibility), rerun P6 for s27_dladdr/s27_dlinfo; decide (a) above.

**Not to redo.** The reference probe of ymm/zmm (results above); the must_fail move.
Plants file: `handoff/HEADERS-S16-FOLLOWUP-plants.txt` (4 plants).
