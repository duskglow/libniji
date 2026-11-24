# SL-HEADERS-UNSHIPPED-REST: thread_db.h, proc_service.h, aio.h, mqueue.h, math-vector-fortran.h (handoff)

**State.** Branch `task/sl-headers-unshipped-rest` (from origin/next 44304e709, still next's head at leg 2).
Commits: 1115454a3 (leg 1: templates, tables, generated headers), 2d347eaf8 (typedef from its tag's layout rows),
4761f0be5 (mq_open fortify, form `mode2`), then a WIP test commit and this note. Build role keke:~/w/n3 (synced at
4761f0be5 + the WIP files; resync before every job). Scratch on keke: `~/work/sl-headers-unshipped-rest/` (t1.sh,
t2.sh job scripts; mqprobe.sh = the mq_open fortify probe; tdc.c/tdc-ref = the client built on the reference;
p4out/ = leg 2's P4 tables). Not a git tree on keke: compare generated headers by `LC_ALL=C sort` hash lists.

**Done in leg 2.**
* Tests run (jobs sl-headers-unshipped-rest-t1, -t2, both on older heads: SUPERSEDED for the hand-back):
  rows test green after the fix below; P6 `conformance/run.sh --niji-headers s25_mqueue s11_private_descriptors
  s18_fork_reset x_self_bind`: 4 passed; P4 C++ compiles: 0 FAIL every -std.
* Rows test found `struct td_thrhandle` opaque: the typedef emitter (typedefs.rs, linkage branch) looked layout rows
  up only under the typedef's name; now also under the tag. Fixed thread_db.h (6 opaque structures -> 0) and
  sys/procfs.h (struct user_fpregs_struct now has its members). Committed 2d347eaf8.
* mq_open fortify (4761f0be5): probe-settled on the reference (compile status and `nm -u` only, levels 1-3):
  run-time flags, no extra args -> __mq_open_2; constant flags without O_CREAT (O_TMPFILE too) -> mq_open; constant
  O_CREAT alone, 1 or 3 extra args -> compile error; 2 extra -> mq_open. New fortify.tsv form `mode2` (s18.rs; the
  `mode` output byte-identical), row `mq_open mode2 $0, $1 ($1 & 0100) != 0 0`, `@@fortify` in mqueue.h.in.
  Test: 7 mq_open cases in s18.rs `an_open_without_its_mode_calls_the_checked_entry` (green on keke).
* header_check: 39 FAIL, all P1 aio.h: `_SIGSET_NWORDS`, `_STRUCT_TIMESPEC` (reserved.tsv omit) and
  `_POSIX_C_SOURCE` (== 200809 false under xopen500/600). Cause: macros.tsv makes aio.h their owner (first header),
  and aio.h now has p1 1. Fix in WIP commit: `spec/tools/header_checks/s14.py` p1_skip for the three. NOT RERUN.
* gdb-shaped probe (WIP commit): `crates/core/tests/oracle/thread_db_client.c` + test
  `a_debugger_shaped_client_builds_on_libniji_headers_and_matches_the_reference` in
  crates/core/tests/thread_db_products.rs (package `niji`). On the reference by hand: builds with -Wall -Wextra,
  prints `td_init ok`, `td_ta_new refused, looked up`, sizes 16/384/24/16/8, etc. The libniji side NOT RUN yet.

**Open: P4 part 2 (tables reproduced) FAILs**, `p4out/` on keke: 1819 differing lines in each of callsites.tsv
and noexcept.tsv. Most are `__x` names (math.h `__acos`..., argp.h `__argp_*`: `!error` on libniji, names omitted
by R-H2/R-H3): very likely the same on origin/next (check: run headers_p4.py on next's products, or ask whether P4
part 2 is gated). Mine: proc_service.h's ps_* rows appear in libniji's probe output and not in spec/abi's
callsites.tsv/noexcept.tsv (leg 1 extended prototypes.tsv, not those two): add the rows by rerunning the
reference probe `spec/tools/callsite_probe.py` for proc_service.h (and thread_db.h? check), as leg 1 did for the
other tables (PROVENANCE line). About 220 other non-`__` lines unclassified: classify (`diff ... | grep -v '	__'`).

**Next steps (leg 3), in order.**
1. Sync; one job: rows test, `cargo test -p niji --test thread_db_products a_debugger`, header_check (expect 0
   FAIL), the P4 part-2 classification above. Fix what they find.
2. aio C program (brief step 4): no conformance program uses an aio.h constant. Write a small self-checking
   `conformance/c/s11_aio_listio.c` (clause s11_11_c_0; lio_listio LIO_WRITE then LIO_READ with LIO_WAIT, a LIO_NOP
   entry, aio_cancel on a finished request = AIO_ALLDONE, aio_suspend/aio_error/aio_return); run it in both modes
   (`sh conformance/run.sh s11_aio_listio`, and `--niji-headers`), and by hand against the reference (system gcc).
   mqueue.h is covered by s25_mqueue.c (passed under --niji-headers). New file: `TESTS:` line in its commit.
3. Plants file `handoff/SL-HEADERS-UNSHIPPED-REST-plants.txt` (docs/plants.md format, `lit:` form):
   (a) drop td_thr_get_info's declaration in libc-include/thread_db.h; cmd `cargo test -p niji --test
   thread_db_products a_debugger`; msg `thread_db_client.c does not build`. (b) `enum { LIO_WRITE = 1 };` -> `= 0`
   in libc-include/aio.h; cmd `sh conformance/run.sh --niji-headers s11_aio_listio`; msg `FAIL`. (c) optional: mode2
   arity check `!= 2` -> `!= 1` in s18.rs; cmd the s18 fortify test. Run under jobrun (brief flow 6), dir
   `/dev/shm/claude/sl-headers-unshipped-rest/plants-1`.
4. math-vector-fortran.h: gfortran on shiki/kanon only. Not shipped this story: shipping needs a Fortran output form
   of the generator (P5 compiles every slice row as C). Hand back as RULING NEEDED / follow-up; keep it in
   validation/headers_unshipped.tsv with that reason. If budget allows, probe which `_ZGV*` the reference's file
   vectorizes (Fortran loops -O3 -ffast-math, nm) vs libniji's libmvec (versions.tsv libmvec.so.1).
5. Release note: docs/release-notes.d/ fragment naming what is left unshipped: complex.h, tgmath.h (their slice),
   math-vector-fortran.h (generator story).
6. `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo xtask static` on the role,
   precheck `--jobs <labels> --head <sha>`, report json (handoff/reports/SL-HEADERS-UNSHIPPED-REST.json),
   handback-check. `affected` escalates to FULL (generator inputs): gate = precheck + named tests (kind `test`).

**Decisions in force.** D-X.168 (unshipped list), D-10.125, R-H7 (thread_db.h and proc_service.h ship). No new
D-row (mode2 and the event-set macros are probe-settled local details; reserve one with `ops reserve` if wanted).

**Plan rows to read.** plan/headers.md §4.1, slices 14/16/18; validation/headers_p4.py docstring (part 2).
