# SL-HEADERS-S08: slice 8, signals, threads, contexts (handoff after run 2, SL-HEADERS-S08-2)

**State.** Branch `task/sl-headers-s08-2` (stacked on `task/sl-headers-s08`). Head: see `handoff/reports/SL-HEADERS-S08-2.json`.
INCOMPLETE (context checkpoint, budget not spent: 63 of 110 calls). `headers-gen` is clean (38 headers written,
`libc-include/` has the 11 slice 8 headers and regenerated stdio.h, stdlib.h, sys/types.h). Say "slice 8" in commits.

**Done in run 2.**
* Ruling #6390: `spec/tools/type_probe.py` has a pointer-constant class (`__builtin_classify_type == 5` and an address
  that is a small or negated small number); `macros.tsv` +18 rows of type `pointer` (value = signed address): SIG_DFL 0,
  SIG_IGN 1, SIG_ERR -1, SIG_HOLD 2, PTHREAD_CANCELED -1, SEM_FAILED 0, and (generic, not asked) FE_DFL_ENV, FE_DFL_MODE,
  FE_NOMASK_ENV, LC_GLOBAL_LOCALE, MAP_FAILED, MM_NULL*, RTLD_DEFAULT, RTLD_NEXT, TD_SIGNO_MASK. Full probe rerun
  (24 s): typedefs.tsv unchanged, no earlier macros.tsv row changed. `NULL` is excluded (C++ spells it `__null`).
  `abi_types.rs` accepts type `pointer`; `literal("pointer", v)` gives `2L` / `(-1L)`; the templates write
  `((__sighandler_t) @@value(SIG_DFL))`, `((void *) @@value(PTHREAD_CANCELED))`, `((sem_t *) @@value(SEM_FAILED))`.
  No hand-typed constant is left in these templates. Two unit tests added (literal, table shape).
* The 10 headers-gen problems are cleared: `members_nested` is now `members_why`/`member_lines`/`member_decl` (reason in
  the error text): members of anonymous union/struct type are emitted from the rows below them, an unnamed
  `<anon>` row (`size N rows K`, struct sigcontext, rusage) becomes a C11 anonymous union, function-pointer members come
  from the new `headers/fnmembers.tsv` (POSIX/man page declarators, 5 rows), a declared alignment above the members'
  natural one goes on the first member (`struct rseq`, 32). Typedef of an array of scalar or struct (`__jmp_buf`,
  `gregset_t`, `jmp_buf`), `typedef struct ucontext_t` with an empty C++ spelling, `__sigval_t` (union named by tag),
  `LANGUAGE_MODE` condition for `max_align_t` (compiler language version, not a level). signal.h.in defines
  `sa_handler`, `sa_sigaction`, the `si_*` member macros and `TEMP_FAILURE_RETRY` (visible in signal.h under GNU levels).
* `header_check.py` HEADERS and SLICE6 lists include the 11 headers. Proof script (scratch, not committed): header_check,
  `header_probe.py --include libc-include --headers <11>`, `probe_compare.py visibility`, `callsite_probe.py`, `probe_compare.py callsites`.

**Measured on the last tree (job sl-headers-s08-2-proof3, keke n2).**
* P1/P5 `header_check.py`: 49 failed of 2305 (was 485 on the first tree). All shown are pthread.h in C++:
  `libc-include/pthread.h:223: extern int pthread_attr_destroy (pthread_attr_t *) __NIJI_NOTHROW;` parsed as a variable, so
  `pthread_attr_t` is not a type there (check its typedef/union emission order or guard in C++; only cxx configs fail). Only the
  last 4 lines of the log were read: rerun `HC_TAIL=60` and group by message.
* P2: 627 differences of 3493 rows, not classified. Seen: `_SIGSET_NWORDS` missing in ucontext.h/sys/ucontext.h,
  `_STRUCT_TIMESPEC` missing in threads.h, `thread_local` condition differs (table `!c2x,gnu2x,time64only,cxx*`), `timespec`
  struct in threads.h (table `!time64only`, ours `!c89,c99,isoc99,time64only`). Many are probably the same pthread.h
  cause in C++ (everything including pthread.h).
* P3: `callsite_probe.py` aborts: "pthread.h.cxx98.addr.cc: the unit failed outside its own test lines" (same pthread.h C++ error).
  setjmp/sigsetjmp P3 rows not yet seen.
* NOT RUN: clippy, `cargo xtask static`, the headers unit tests (`cargo test -p xtask headers`, abi_types tests), P6, plants,
  the gate, `cargo xtask test` and the start-up checks (not needed: no start-up code touched). No plants file written.
* `libc-include/stdio.h`, `stdlib.h`, `sys/types.h` changed (members emitted instead of opaque bytes where the new
  layout rows allow; `__sigset_t`, `__pthread_cond_s`, `_G_fpos_t`): review that sizes still match (P1 passes for them).

**Not to redo.** The probe rerun, the generator member rework, the 10-problem fixes. `headers-gen` takes about 3.5 min
on keke (xtask build); use one job for gen plus rsync back (`rsync -e "ssh -o BatchMode=yes" claude@<keke>:w/n2/libc-include/ libc-include/`).

**Next step.** (1) Find why `pthread_attr_t` is not a type in C++ in pthread.h; (2) rerun the proof script and classify
the P2 differences (fix the templates, or `@@owns`; `_SIGSET_NWORDS` and `_STRUCT_TIMESPEC` are macros that other
headers' includes show); (3) P3 for setjmp/sigsetjmp; (4) P6: `cargo xtask so`, `conformance/run.sh --niji-headers`;
(5) plants (suggested: `@@init` table byte, cleanup macro's `__sigsetjmp` call, `SIGRTMIN` call, `thread_local`
condition, a CPU_SET word index, the pointer-constant literal, the fnmembers declarator, the `align` attribute);
(6) `records/pending/SL-HEADERS-S08-1.toml` (D-NEW, series D-X: `@@init`, typedef/anonymous members, fnmembers.tsv,
pointer macros class, LANGUAGE_MODE); a `spec/abi/PROVENANCE` line for the 11 layout types; (7) clippy, static, gate.
Open from run 1 still: `cargo xtask static` passed with a template that did not generate (`--check` should fail it).

**Plan rows.** plan/headers.md slice 8, ruling #6390 (pointer constants in macros.tsv).
