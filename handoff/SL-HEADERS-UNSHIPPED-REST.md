# SL-HEADERS-UNSHIPPED-REST: thread_db.h, proc_service.h, aio.h, mqueue.h, math-vector-fortran.h (handoff)

**State.** Branch `task/sl-headers-unshipped-rest` (from origin/next 44304e709). Leg 1 commit 1115454a3 (code and
tables), this note on top. Build role keke:~/w/n3. Scratch on keke: `~/work/sl-headers-unshipped-rest/` (probe
scripts, `p1/` = the probe copy of spec/ and headers/; `tdmacros.c` = the event-set macro probe).

**Done (leg 1, 1115454a3).**
* Templates `headers/{thread_db,proc_service,aio,mqueue}.h.in` (@@emit over the tables), rows in
  `headers/slices/s16.tsv` (thread_db.h, proc_service.h; p1 1, uapi -) and `s14.tsv` (aio.h, mqueue.h; p1 1, uapi -:
  linux/mqueue.h defines struct mq_attr, so no P8 pairing). Generated `libc-include/*.h` committed.
* Tables extended on the reference (keke): abi_extract.py (proc_service.h in HEADERS; td_* tags and enum typedefs in
  TYPES) --layouts-only (+64 layout rows, +7 ps_err_e enumerators) and --prototypes-only (+15 ps_* rows);
  header_probe.py and type_probe.py --headers thread_db.h,proc_service.h (+22 visibility, +22 typedefs rows; nothing
  lost or changed). PROVENANCE line added. `headers/providers.tsv`: the 11 new ps_* callbacks `debugger`.
* thread_db.h's event-set macros written in the template; probe-settled on the reference (tdmacros.c): event N is bit
  (N-1)%32 of word (N-1)/32; td_eventismember yields the masked word (CREATE -> 128), td_eventisempty 0/1;
  fillset = all ones in both words. TD_SIGNO_MASK via `@@null` (macros.tsv type null).
* `validation/headers_unshipped.tsv`: the four rows removed (P4).
* Checked: headers-gen clean (job sl-headers-unshipped-rest-gen2); each header compiles alone and twice, gnu17, c99,
  C++17, -Wall -Wextra -Wsystem-headers -Werror. Nothing else run yet.

**Next steps (leg 2), in order.**
1. Tests on keke: `cargo test -p xtask headers::tests::every_table_row_is_emitted_and_matches_the_tables` (rows test),
   `python3 spec/tools/header_check.py` (P1/P5 for the new rows), `python3 validation/headers_p4.py` (P4 list),
   conformance P6: `conformance/c/s25_mqueue.c`, `s11_private_descriptors.c`, `s18_fork_reset.c`, `x_self_bind.c`
   include the new headers (`conformance/run.sh --niji-headers`). Fix what they find.
2. mq_open fortify: spec/abi/callsites.tsv has `mq_open call * __mq_open_2`; mqueue.h has no wrapper yet (fortify.tsv
   `mode` form refuses more than one variadic argument, and mq_open with O_CREAT takes two). See what the rows/P3
   checks say; a wrapper needs a s18.rs change (a slice file, allowed) or a rows-exceptions.tsv row with the reason.
3. gdb-shaped probe (brief step 1): C file including both headers, calling td_ta_new and td_thr_get_info and
   defining ps_pglobal_lookup (+ the other ps_* libniji's libthread_db imports: ps_pdread, ps_lgetregs, ps_lsetregs,
   ps_lgetfpregs, ps_lsetfpregs, ps_getpid), linked with -lthread_db against target/niji; add it as a test where one
   fits (crates/boundary/tests/niji_cc_headers.rs or a header_checks/s16.py unit), plus one C program per header
   (aio: aio_write/aio_suspend on a temp file; mqueue: mq_open/send/receive; compare with the reference).
4. Plants file `handoff/SL-HEADERS-UNSHIPPED-REST-plants.txt` (docs/plants.md format): drop a td_* prototype row
   (spec/abi/prototypes.tsv td_thr_get_info) and a wrong aio.h constant (enumerators.tsv LIO_WRITE value); run with
   `cargo xtask plants` under jobrun (brief, developer flow 6).
5. math-vector-fortran.h: gfortran exists on shiki and kanon only (not keke). Probe which intrinsics the reference's
   file vectorizes (Fortran loops at -O3 -ffast-math, `nm` for _ZGV*), compare with libniji's libmvec (versions.tsv
   libmvec.so.1). Shipping it needs a Fortran output form of the generator (the @@generated banner is a C comment; P5
   compiles every slice row as C): a generator story under the slice contract, so it stays in headers_unshipped.tsv
   with that reason; name it in the hand-back as RULING NEEDED / follow-up.
6. Release-notes fragment: edit docs/release-notes.d/SL-HEADERS-S20-2.md's list (unreleased) or add
   SL-HEADERS-UNSHIPPED-REST.md (known-limitations) naming what is left: complex.h, tgmath.h (their slice) and
   math-vector-fortran.h.
7. clippy + `cargo xtask static`, precheck, report json, handback-check. `cargo xtask affected --list` will escalate
   to FULL (headers/*.h.in are generator inputs, as in slice 20): gate = precheck + the named tests (kind `test`).

**Decisions in force.** D-X.168 (unshipped list), D-10.125, R-H7 (thread_db.h and proc_service.h ship: debuggers).
No new D-row yet (the event-set macro layout is probe-settled, a local detail: a one-line D-row may be reserved with
`ops reserve --story SL-HEADERS-UNSHIPPED-REST D-X` if a reviewer wants one).

**Plan rows to read.** plan/headers.md §4.1 (Adding a slice), slice 14 and 16 rows; validation/headers_p4.py header.
