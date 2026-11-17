# SL-HEADERS-S08: slice 8, signals, threads, contexts (handoff after run 3, SL-HEADERS-S08-3)

**State.** Branch `task/sl-headers-s08-3` (stacked on run 2). Head: see `handoff/reports/SL-HEADERS-S08-3.json`. Say "slice 8" in commits.
clippy, `cargo xtask static`, `headers-gen --check`, `cargo test -p xtask headers` green on the code head (mei role s2).

**Done in run 3.**
* P1/P5: 49 failed -> 0 of 2305. Cause of the pthread.h C++ failures: in C++ `pthread_attr_t` is a union TAG (typedefs.tsv `cxx` empty; black-box
  probe of the reference, `union pthread_attr_t *` and the bare name compile after sys/types.h, pthread.h, signal.h, stdlib.h, aio.h), so visibility.tsv
  has no typedef row for it in cxx configurations. The generator (`union_tag`, `cxx_mask`, `typedef()`) now declares the tag in C++ and the typedef in C.
* An included header covers a name only where it shows it (`covered` is a mask; `fully_covered`): `pid_t`, `struct timespec` in sched.h/pthread.h/threads.h.
  An include right after an `#if` line is conditional (not counted as cover). sched.h includes time.h only for `!MISC && POSIX < 200112` (table);
  pthread.h no longer includes limits.h or setjmp.h (declares `__sigsetjmp` only); endian.h conversion macros follow be16toh's row.
* P2: 627 -> 21 differences. Classes of the 21: 17 glibc-internal macros `_SIGSET_NWORDS` (9) and `_STRUCT_TIMESPEC` (8) in the reference's headers, not
  defined by libniji (source-compat rule of the brief: no new glibc-internal names; an `--accept` file and a differences row are still to write);
  `bsd_signal` x2 (signal.h, sys/signal.h; exported, but spec/abi/prototypes.tsv has no row for it: SPEC GAP, add the prototype row);
  `siginfo_t` typedef and `sigval` union x4 (ours also visible in posix1, posix2, xopen, xopenext: emitted as dependencies of `struct sigaction`'s
  `sa_sigaction` member; unfixed); `thread_local` in threads.h x1 (table says visible in isoc2x and gnu, not in c2x, gnu2x, cxx; template now uses the
  compiler language version; one row left, check it); `__sigsetjmp` extra in pthread.h x1 (the cleanup macro needs it).
* P3: 10 -> 6. `pthread_equal`/`thrd_equal` are compared in place under `__OPTIMIZE__` (gnu_inline). Left: `longjmp`, `_longjmp`, `siglongjmp` -> `__longjmp_chk`
  in the fortify configurations (slice 18). setjmp/sigsetjmp rows (P3) match.
* fnmembers.tsv (#6448): each row cites its POSIX or man page source (`source` column). `handoff/SL-HEADERS-S08/fnprobe.sh ref|niji`: against the reference
  all 5 rows: exact-type assignment compiles, member type equals the declarator, a wrong parameter type is rejected. libniji: 4 of 5 pass; `sigevent_t`
  (the typedef) is NOT declared by our signal.h in C++ (the reference has it; like pthread_attr_t it has no typedef row in C++, probably a tag). NEXT.
* Added `sigev_notify_function` / `sigev_notify_attributes` macros (sigevent(3type)); they are not in visibility.tsv.
* Plants file `handoff/SL-HEADERS-S08/plants.txt` (9 plants, all `headers-gen --check`); proof script `handoff/SL-HEADERS-S08/proof.sh` (mei role s2).

**Not run / open.** P6 (`cargo xtask so`, `conformance/run.sh --niji-headers`); `records/pending/` D-NEW (`@@init`, member emitter, fnmembers.tsv,
pointer class, LANGUAGE_MODE, union tag); `spec/abi/PROVENANCE` line for the 11 layout types; `--accept` file + differences row for the 17 internal
macros; the 4 siginfo_t/sigval rows; `sigevent_t` in C++; thread_local row; the unit test for `union_tag`; the `static` check should fail a template that
does not generate. Gate and plants jobs: see the report json.

**Next step.** Fix `sigevent_t` in C++ (rerun `fnprobe.sh niji`), then the P2 rows above, P6, the records, then plants and the gate on the final head.
`headers-gen` on mei takes about 2 minutes; `proof.sh` there writes full outputs to `~/work/SL-HEADERS-S08-3/{hc,p2,p3}.out`.
