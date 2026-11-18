# SL-HEADERS-S08: slice 8, signals, threads, contexts (handoff after run 4, SL-HEADERS-S08-4)

**State.** Branch `task/sl-headers-s08-4` (stacked on run 3, origin/next merged once). Head: `handoff/reports/SL-HEADERS-S08-4.json`.
`headers-gen` writes 78 headers; `header_check.py` 0 failed of 5366 (all slices). Say "slice 8" in commits.

**Done in run 4.**
* Moved onto the registry layout (steps 1-4 of handoff/SL-HEADERS-REGISTRY.md): the slice's headers are `headers/slices/s08.tsv`; headers.rs keeps the
  slice generator work (member emitter, `@@init`, fnmembers, union tag, pointer literals; D-X.128), merged by hand with slice 9's additions in the same
  functions (coverage mask `is_covered` kept as next has it plus the union-tag exception; structure-tag members, enum members and packed structures added
  to the member emitter). It is a shared-generator edit in its own commit (the merge commit) and is named in the hand-back.
* Gate was red at `stubs`-class causes: `__sigset_t` layout rows collided with slice 9's `sigset_t` (`abi_sigset_t` defined twice; removed my two rows).
* threads.h `thread_local`: the macro is not defined from `__STDC_VERSION__` above 201710L (the reference compiler's -std=c2x says 202000L).
* Rulings #6467/#6468: reserved names are excluded by reserved.tsv (`_SIGSET_NWORDS`, `_STRUCT_TIMESPEC` omit; `bsd_signal` omit, reason now C 7.1.3 and #6468,
  not a prototypes row). Differences rows and D-X.129; accept file `handoff/SL-HEADERS-S08/p2-accept.txt` (siginfo_t, union sigval at POSIX 1/2, `__sigsetjmp`).
* P2 on slice 8 after this: only those 6 accepted rows. P3: `longjmp`/`_longjmp`/`siglongjmp` -> `__longjmp_chk` in fortify configurations (slice 18's).
* P6 (`conformance/run.sh --niji-headers`): 42 passed, 5 failed (see below).

**Not done / open (for the next run).**
* P6 failures. Three are SPEC GAPS of the name tables: `sig_atomic_t` (C 7.14, signal.h) and the mutex kinds `PTHREAD_MUTEX_NORMAL`, `_RECURSIVE`, `_ERRORCHECK`,
  `_TIMED_NP` (pthread.h) have no row in visibility.tsv/typedefs.tsv/macros.tsv at all (the candidate list of the table builder misses them), so headers-gen cannot
  emit them: `s20_pthread_kill`, `s22_errno`, `s22_symbols` do not build against libniji's headers. Fix = add the names to the extraction's candidate list and
  regenerate the three tables (a shared spec tool, outside this slice's files), then `@@emit` does the rest. `s02_assert` (the program name carries the `.lh`
  suffix the mode adds) and `s02_chk_fail` (fortify, slice 18) are not slice 8.
* `sigevent_t` typedef: the reference declares it (also in C++); no table row names it, so P2 cannot see it. Same table-builder fix; the 5th fnmembers row waits for it.
* Not run: unit test for `union_tag`; `spec/abi/PROVENANCE` line for the layout types of this slice; the `static` check that a template which does not generate fails.
* INFORMATIONAL: P2 over utmpx.h shows 35 names (sys/types.h contents) that the reference's utmpx.h does not show; stdio.h `fwrite_unlocked` under fortify and
  stdlib.h `WIFCONTINUED` at XSI differ too. Those headers are slices 6 and 9's, their generated text differs from next's only in layout of unnamed structs.

**Jobs of the final head** are in the report json. Plants file: `handoff/SL-HEADERS-S08/plants.txt` (two patterns updated for the merged code).
`headers-gen` on mei takes about 4 minutes cold.
