# VALGRIND-LOADER-VIEW report

Branch `task/valgrind-loader-view`, base origin/next d5764940d. Host: Zen 1 host (role keke `~/w/n3`).

## Commits (all mine)
* 3f1ac8b16 hide the loader's whole-file views from valgrind (item 1, D-X.152 option (b))
* 354d33f30 name libc.so.6's compat-shadowed defaults `name@@VERSION` in `.symtab` (item 2, D-X.153)
* b47776ce9 compare libc.so.6's loaded bytes without the section header fields (leg 3: the first segment of
  libc.so.6 starts at offset 0 and covers e_shoff/e_shnum, which objcopy rewrites; the check refused every
  real build)
* niji_csys.h declaration (K5), trace clause on the products test, clippy type alias; clippy::panic allowed in
  the image tests
* D-X.152 record; plants file builds the loader module (`--features ld`)

## Item 1 (D-X.152): cause and choice
valgrind 3.24's `di_notify_mmap` records a readable file mapping made before an object's segments as a mapping of
the object and passes over an inaccessible one; its `di_notify_mprotect` does nothing. The loader's whole-file view
covered every read-only segment's file range, so valgrind took `.eh_frame`/`.rodata` bias from the view.
Start-up (perf stat -r 50, strace; Zen 1 host; programs run through env, a constant in every value):

| row | variant | user instr | kernel instr | faults | syscalls | mprotect |
|---|---|---|---|---|---|---|
| empty | head (b) | 1044564 | 2308606 | 141 | 28 | 2 |
| empty | (a) | 1044260 | 2283366 | 141 | 29 | 3 |
| three | head (b) | 1259839 | 2680150 | 150 | 58 | 5 |
| three | (a) | 1259613 | 2610446 | 150 | 62 | 9 |
| dlopen | head (b) | 1409158 | 2798013 | 171 | 70 | 3 |
| dlopen | (a) | 1408754 | 2738179 | 171 | 74 | 7 |

(a) costs one system call per view (deterministic), so (b). Bare-machine cost of (b), like for like (the head's
tree with origin/next's image.rs, same build, two rounds): user instructions base -> head empty 1044547/1044509 ->
1044564/1044578, three 1260012/1259974 -> 1260023/1260018, dlopen 1409162/1409164 -> 1409280/1409257 (about +40,
+40, +100; under 0.01 %); faults and system calls equal; kernel within run-to-run spread. The first "next" column
(measure-a4) came from a different build of origin/next (17.8 MB libc.so.6 vs 6.4 MB) and is not used.
Layout-bins (base views vs head): only LLVM hash suffixes differ.

## Item 2 (D-X.153)
helgrind wraps `pthread_create@*` by its `.symtab` name; the default was named plainly and only the compat stub
carried a version, so helgrind wrapped the stub and asserted at join. 207 defaults now also carry
`name@@VERSION` (`so: 207 defaults with a compat version beside them ...`).

## Acceptance (swapped trixie root, trixie valgrind, whole libc6-dbg, debs from b47776ce9)
`=== VGACC pass=8 fail=0`: memcheck true/bad/cxx equal to stock; leak `leak_at_main=1 unnamed_frames=0`; helgrind
thr 0 errors, thr_old 0 errors (the stub's jump into the also-wrapped default did not double-count), race 2 races
at race.c, all equal to stock. The debs predate only the header declaration, the test doc comment and lint fixes
(no change to the products' code); job `valgrind-loader-view-leg3` (its clippy/static lines are the pre-fix ones,
fixed afterwards and green in the gate and lint jobs).

## Gate, plants, start-up checks
GATE_PLANTS_STARTUP

## RULING NEEDED
* The loader module's unit tests (`crates/boundary/src/ld`, feature `ld`) are built by no gate or CI runner (the
  library tests use `export,selftest`). D-X.152's test runs only with
  `cargo test -p niji-boundary --lib --features ld` (the plants use that). Recommend: `cargo xtask affected` adds
  that run when a file under `crates/boundary/src/ld/` changes (a tooling story); until then the test is proven by
  plants but not run by the train.

## INFORMATIONAL
* libniji's loader resolves `libc.so.6` without the program's DT_RUNPATH (seen while measuring: a program linked
  with `-rpath` to a directory holding libniji's libc.so.6 got the system path); unchanged here, not chased.
* Waste: two product builds lost to a check of my own (loaded bytes at offset 0) that leg 2 never ran on a real
  build; a plant run lost because the test lived in a module no test runner builds.
