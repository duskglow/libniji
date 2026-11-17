# SL-HEADERS-S08: slice 8, signals, threads, contexts (handoff after run 1)

**State.** Branch `task/sl-headers-s08`, cut from origin/next 20e47e117 (slice 6 landed). Head: see
`handoff/reports/SL-HEADERS-S08.json`. INCOMPLETE: the 11 templates exist and `headers-gen` generates the slice's
simple headers, but it still reports 10 problems (below), so `libc-include/` has none of this slice's output yet
and none of P1, P2, P3, P5, P6, P9 has run. Say "slice 8" in commits and handoffs.

**Done.**
* Templates (`headers/`): `signal.h`, `sys/signal.h`, `setjmp.h`, `ucontext.h`, `sys/ucontext.h`, `pthread.h`,
  `semaphore.h`, `threads.h`, `sched.h`, `sys/single_threaded.h`, `sys/rseq.h`.
  * `setjmp`/`sigsetjmp` expand to `_setjmp`/`__sigsetjmp` (C.9; callsites.tsv agrees).
  * `signal.h`: `SIG_DFL/IGN/ERR/HOLD` (0, 1, -1, 2 typed from POSIX and the kernel's values: no table holds
    them), `SIGRTMIN/MAX` call `__libc_current_sigrtmin/max`, `MINSIGSTKSZ/SIGSTKSZ` are `__sysconf (_SC_..)`
    under `__NIJI_DYNSTACK`, the `macros.tsv` constants otherwise.
  * `pthread.h`: C and C++ `pthread_cleanup_push/pop` (and the `_np` defer/restore pair under `__NIJI_GNU`) per C.9,
    initializers by `@@init`, `PTHREAD_CANCELED` (-1, typed), includes `limits.h` (PTHREAD_STACK_MIN), `sched.h`,
    `time.h`, `setjmp.h`.
  * `sched.h`: the CPU_SET(3) macro set, written from the man page, `__sched_cpucount/cpualloc/cpufree` calls.
  * `threads.h`: `thread_local`, `ONCE_FLAG_INIT`. `semaphore.h` includes `sys/types.h` (FD_*, endian names).
* Generator (`xtask/src/headers.rs`): new marker `@@init(NAME)` (brace initializer of an `initializers.tsv` row built
  from the layout rows: a union by its first member); structure members that are typedefs of aggregates or arrays
  (`__sigset_t`, `__jmp_buf`, `__pthread_list_t`) are emitted by name, with their typedef first; a member with
  dotted sub-rows is "whole" when it is not anonymous (this fixed `struct __pthread_mutex_s`, `__pthread_cond_s`,
  `__jmp_buf_tag`, `__cancel_jmp_buf_tag`).
* Tables: `spec/tools/abi_extract.py --layouts-only` with 11 more types (`__cancel_jmp_buf_tag`, `sigevent_t`,
  `struct sigcontext`, `_libc_fpstate/fpxreg/xmmreg`, `__pthread_mutex_s`, `__pthread_rwlock_arch_t`,
  `__pthread_cond_s`, `__pthread_internal_list`, `__sigset_t`): `layouts.tsv` +109 rows, no earlier row changed.
  `spec/abi/PROVENANCE` has no line for this yet (add one, as slice 3 did).
* `headers/reserved.tsv`: `emit` rows for the C.9 callees and types, `omit` for `bsd_signal` (compat-only).

**Open (what `headers-gen` still says; job `sl-headers-s08-lint` on keke n1).**
1. `struct sigaction`, `struct sigevent`, `struct sigcontext`, `ucontext_t`, `struct _pthread_cleanup_buffer`: members
   of anonymous-union type (`union <anon>` with `sa_handler`/`sa_sigaction`) or of unnamed function-pointer type
   (`fn(...) *`) that `layouts.tsv` cannot spell. Needs either layout rows with the function type spelled (the probe
   in `abi_extract.py` loses the prototype) or generator support for anonymous aggregates; the latter is the better fix
   (siginfo_t has the same problem and is already generated? check how `siginfo_t` came out).
2. `signal.h`: `si_value`, `si_status` (and the other `si_*` member macros: define them from the `siginfo_t` member
   rows; the nested names are in `layouts.tsv`), `TEMP_FAILURE_RETRY` (unistd's name visible in signal.h: needs the
   include or an `@@owns`).
3. `sys/rseq.h`: `max_align_t` has no condition over the levels (visible in C11+ and C++11+, not in `c89,c99`; the
   probe shows it also in `cxx98`): own it with a hand condition or include `<stddef.h>` once that header exists.
4. Not started: header_check P5/P1 over the slice, `header_probe.py`/`callsite_probe.py` P2/P3 (setjmp and sigsetjmp
   are the P3 rows named in the brief), `conformance/run.sh --niji-headers` for the pthread/signal programs (P6),
   `cargo xtask plants` file (suggested plants: the `@@init` table byte, the cleanup macro's `__sigsetjmp` call, the
   `SIGRTMIN` call, `thread_local` condition, a CPU_SET word index), a decision record
   (`records/pending/SL-HEADERS-S08-1.toml`, `id = "D-NEW"`, series D-X: `@@init`, typedef members, the 11 layout types).
5. `cargo xtask static` was green with these problems present: `--check` did not fail on a template that does not
   generate. Worth a look: a slice that cannot generate should not be able to pass static.

**Things not to redo.** The layouts extraction (rerun gives the same 109 rows); the generator changes (stdlib.h and
sys/types.h regenerate unchanged: they broke once when only the layout rows were added and were fixed by the typedef
member branch). Typed ABI facts to recheck against a probe: `SIG_HOLD` 2, `PTHREAD_CANCELED` -1, `SEM_FAILED` 0
(no table holds them).

**Next step.** Item 1 (anonymous aggregates in `members_nested`), then 2 and 3, then `cargo xtask headers-gen`,
`header_check.py`, P2/P3 probes, P6, plants, gate.
