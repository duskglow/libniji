# SL-HEADERS-8: signals, threads, contexts

Id: SL-HEADERS-8
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-6
Model: sonnet
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 8; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. `pthread_cleanup_push` and `pthread_cleanup_pop` follow C.9 in both their C and C++ forms. `PTHREAD_STACK_MIN`, `MINSIGSTKSZ` and `SIGSTKSZ` follow C.9 under `__LIELLA_DYNSTACK`.

Headers (plan/headers-inventory.tsv, slice 8): `signal.h`, `sys/signal.h`, `setjmp.h`, `ucontext.h`, `sys/ucontext.h`, `pthread.h`, `semaphore.h`, `threads.h`, `sched.h`, `sys/single_threaded.h`, `sys/rseq.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P6 (the pthread, signal and setjmp programs). P3 for `setjmp` and `sigsetjmp`.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
