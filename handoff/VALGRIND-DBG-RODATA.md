# VALGRIND-DBG-RODATA: valgrind reads libc6-dbg (GitHub #13, the rest)

Branch `task/valgrind-dbg-rodata` (from origin/next 5e1903cce). Brief `~/state/briefs/VALGRIND-DBG-RODATA.md`.
Role mei `~/w/s4` (Skylake-SP). Job scripts and outputs: mei `~/work/VALGRIND-DBG-RODATA/` (local copies in
`~/w/.scratch/VALGRIND-DBG-RODATA/job/`). Decision D-X.150 (reserved, record written). Plants file
`handoff/VALGRIND-DBG-RODATA-plants.txt`. Report `handoff/reports/VALGRIND-DBG-RODATA.json`.

## State

* Item 1 (`find_rodata` abort with libc6-dbg): FIXED. Item 3 (memcheck's extra start-up report): FIXED.
* Acceptance on the final head (job `valgrind-dbg-rodata-accfinal`, swapped trixie root, valgrind 3.24.0, libniji's
  whole libc6-dbg): memcheck error summaries equal stock on /bin/true (0), bad (1), cxx (1, 32 bytes lost at cxx.cc)
  and leak (1, 9 bytes lost); drd 0 errors as stock; `--track-origins=yes` on /bin/true and ./leak: 0 errors.
* NOT met: the leak record names malloc and `niji_boundary::printf::vasprintf` (printf.rs:957), then three `???`
  frames, no `main` (stock: malloc, vasprintf, asprintf, main). Cause found (below); fix needs a ruling.
* Item 2 (helgrind `hg_main.c:5421 Assertion 'found'`): still asserts; diagnosed only as far as below.

## Item 1: cause and fix

valgrind 3.24's `coregrind/m_debuginfo/readelf.c` (valgrind source, allowed by the brief): the first `.rodata*`
section of the object sets its read-only data range; on the separate debug file, `find_rodata` matches only the
section named `.rodata` and asserts its size equals that range. libc.so.6 had `.rodata1` (the start-up group,
D-X.133/D-X.143) first, so every tool aborted. Fix (D-X.150): the link-script fragment `RODATA1_FIRST` names the
output section `.startup_ro` (inputs still `.rodata1`); `cargo xtask so` refuses a libc.so.6 or loader with a
`.rodata*` section other than `.rodata` (`check_debug_reader_rodata`); `tools/deb/check-system.sh` refuses such a
packaged shared object. cmp_load base vs head: same program headers and section addresses/sizes; loaded bytes
differ only in the build-id and one ELF header byte (`e_shoff`); the loader byte-identical. Static programs keep
GNU ld's `.rodata` + `.rodata1` (INFORMATIONAL, not changed).

## Item 3: cause and fix

memcheck: `Conditional jump ... uninitialised value(s)` at `niji_boundary::ld::link::link` (link.rs:357), origin
link's stack frame. The loader's lazy cache cell was `OnceCell<Option<(&[u8], Cache)>>`: an empty cell is a tag
byte beside an unwritten payload, and the optimised release of the cache bytes loaded the payload's pointer word
(`0x2818(%rsp)`) and branched on it before applying the tag (`cmpb $0,0x2810(%rsp)` + `cmove`). Never a wrong result,
but memcheck cannot know. Stock /bin/true: 0 errors, 0 suppressed (stock has no such read). Fix: `LazyCache` is
`OnceCell<(&[u8], Option<Cache>)>` (empty bytes = no cache): the empty cell is the bytes' null address, which
`OnceCell::new` writes. Test `lazy_cache_reads_once_and_its_empty_state_is_the_bytes_address`
(crates/ld/tests/search_cache.rs). Loader text 48 bytes smaller; libc.so.6 unchanged (only build-id, `e_shoff`
and the 16-byte source tag differ).

## Leak frames: cause found, fix needs a ruling

`--trace-symtab` (job `valgrind-dbg-rodata-diag1`, out in `~/work/VALGRIND-DBG-RODATA/diag1/out/d-trace.err`):
valgrind acquired libc.so.6's read-only PT_LOADs twice, bias 0x4970000 first and the real 0x8930000 second, and took
`.eh_frame` at 0x4d243c8 (the first bias). 0x4970000 is the loader's whole-file view of libc.so.6 (`map_view`,
DYN-START-RING), mapped PROT_READ from offset 0 before the segments: valgrind's `di_notify_mmap` (debuginfo.c)
records every read-only mapping of the file until the object's debug info is read, munmap does not drop a pending
one, and the view covers every PT_LOAD's file range, so its ranges come first. Every unwind through libc then uses
CFI read from the wrong place. libc's FDE for vasprintf is correct (checked with readelf). Options:
(a) map views PROT_NONE and `mprotect` them PROT_READ (valgrind ignores mprotect): +1 system call per loaded object
at every start; (b) the same only under valgrind (its documented `RUNNING_ON_VALGRIND` client request, a few
instructions natively); (c) map each object over its own view's address range (a redesign). Recommend (b) after a
start-up count of (a).

## Item 2: helgrind (not fixed)

The assertion is in helgrind's `_VG_USERREQ__HG_PTHREAD_JOIN_POST` handler: the joined `pthread_t` was never
registered by helgrind's `pthread_create` child wrapper (`SET_MY_PTHREAD_T`, which sends the child's
`pthread_self()`). Either libniji's `pthread_create` is not wrapped, or the child's `pthread_self()` differs from the
`pthread_t` the parent joins. Next: `valgrind --tool=helgrind --trace-redir=yes -v ./thr` in the swapped root.
It may share the view cause (redirections are set from the symbol table, which is read with the right text bias,
so probably not).

## Final jobs (mei, code head 470f25d84; later commits are handoff/ only)

* `valgrind-dbg-rodata-gate`: direct tier (FULL: the train's, xtask/src/so.rs), 11 suites, 760 passed, 0 failed,
  peak 4.3G. `valgrind-dbg-rodata-plants`: 6 plants, 5 CAUGHT, 1 WARN-NO-MSG (ro-first-name, the products build's
  check caught it first; dropped from the file as a duplicate of ro-first-name-build).
* `valgrind-dbg-rodata-build3`: system debs, check-system PASS. `valgrind-dbg-rodata-accfinal`: the acceptance
  above (rc 1 = the leak-frame case only).
* `valgrind-dbg-rodata-startup2`: `NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products` green (33 passed).
* `valgrind-dbg-rodata-startup`: `cargo xtask test` pass 1 6990 passed; pass 2 4854 passed, 1 FAILED:
  `niji-boundary::s03_memory_symbols kernel_pages_map_aligned_past_a_gap_without_an_aligned_range` (line 1008,
  "the pinned gap is whole again": a mapping appeared in the gap the test made the kernel's first choice). The test
  passes 20 of 20 in pass 1 on the same build; a direct pass-2 rerun was inconclusive (the copy run held 63 tests,
  not this one). Not shown related or unrelated: this change maps nothing after start-up, and the loader change
  only replaces the lazy cache cell's layout. Rerun: `valgrind-dbg-rodata-xtest2` (`cargo xtask test`), started
  at the hand-back.

## Exact next steps

1. Read `valgrind-dbg-rodata-xtest2`: green = the s03 failure is intermittent (report it to the heap/s03 owner as a
   flake to explain); red again = run that test's pass-2 copy (`target/debug/niji-deps/`) with origin/next's
   products to tell whether the loader change matters.
2. Ruling on the leak-frame fix (a/b/c); then a follow-up story: the loader change, a start-up count, vgacc leak.
3. Item 2 diagnosis as above.
