# DL-RDEBUG-VERSION: `r_version` 2 per spec clause s27_4_c_0

State: branch task/dl-rdebug-version (from origin/next 5e1903cce), role mei:~/w/s3. Outcome: **step 2** (nothing found that
relies on 1; layout made safe for 40-byte COPY readers).

## Decisions in force
D-27.62 (Version paragraph rewritten), COMPAT-36 (registered only: restore 1), differences row for `_r_debug`'s version
and size (`docs/differences-from-glibc.md`), spec clause s27_4_c_0 unchanged (it was right for the structure; the old
"1 without dlmopen" was a probe of the reference).

## The change
- `ld.so`'s `_r_debug` is `r_debug_extended`: 48 bytes, `r_version` 2, `r_next` at 40 (null: one namespace).
  Code: `crates/ld/src/debug.rs` (constants), `crates/boundary/src/ld/debug.rs` (`RDebugBase` 40 bytes, `RDebug` = base + `r_next`).
- A program's COPY relocation copies the program's own symbol size: 40 for every program linked against the reference
  (`readelf -s` of the reference's ld.so: `_r_debug` size 40), 48 for one linked against libniji. `note_copy` accepts
  both; a 40-byte copy gets `r_version` 1 (a reader of version 2 would read the program's next variable as `r_next`),
  a 48-byte copy gets 2.
- Tests: `crates/ld/tests/rendezvous.rs` (the three existing programs now expect 2 and a null `r_next`;
  `rendezvous_reference_sized_copy_says_version_1` builds a program with the host C library, whose copy is 40 bytes, and
  runs it under libniji's loader). Plants: `handoff/DL-RDEBUG-VERSION-plants.txt` (4).

## Evidence (trixie root, libniji swapped in, `tools/deb/dbg-system.sh` with gdb, gnat-14, valgrind; v1 = base, v2 = this change)
gdb 16.3 (stops in `setlocale` of /usr/bin/true, `info sharedlibrary`, pending breakpoint in perl's dlopened POSIX.so),
gnat1 (40-byte COPY of `_r_debug`; gcc-14 compiles an Ada unit): outputs byte-identical under v1 and v2 (`diff` of
run.out, dlopen.out, gnat.out, empty; only `_r_debug`'s symbol size differs, 40 vs 48). Same one FAIL in both
(`dbg: FAIL gdb in a running /usr/bin/true`: the script's regex wants `at FILE:LINE` on the `#5` line; the frame
printed is `#0  setlocale () at boundary/src/posix/s07_locale.rs:18`; a script-check issue present before the change).
valgrind in that root: `m_debuginfo/readelf.c:3013 (find_rodata): Assertion 'di->rodata_size == sh_size' failed`,
identical under v1 and v2 (INFORMATIONAL; not the rendezvous).
Other root files naming `_r_debug` (string search): libgnat, libasan/tsan/lsan/ubsan/hwasan, cc1, lto1, lto-dump, libpython3.13, gprofng-display-html: not exercised.

## Open / next
Gate, plants and start-up checks: see the report json. Not done: valgrind's assertion (separate story); the other readers above.
