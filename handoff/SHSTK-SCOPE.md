# SHSTK-SCOPE handoff (planning story; no code lands)

**State.** Branch `task/shstk-scope`. The report is `plan/reviews/SHSTK-SCOPE.md`, cost table first. Evidence is
in `handoff/SHSTK-SCOPE/`:

* `prototype.patch`: a **prototype, not for landing**. It covers the env-gated enable in `entry.S` (ld.so entry
  and `_start`), SSP in `setjmp.S`, the `vfork.S` skip and the `context.S` swap.
* `bench/`: the sources and scripts.
* `tables.md`: medians of 3 passes.
* `measure-m1.log` and `measure-m2.log`: the raw `perf` rows.
* `probe-ref.log` and `probe-niji.log`: the black-box and functional probes.

The worktree tree is back at its base; the prototype exists only as the patch.

**Jobs.**
* `keke:shstk-scope-so`: product build of the prototype tree, rc 0.
* `keke:shstk-scope-marking`: CET-marking survey of Ubuntu 24.04, rc 0.
* `nozomi:shstk-scope-n100-m1` and `nozomi:shstk-scope-n100-m2`: N100 counts, 1 and 2 passes, rc 0, run
  under timing booking 185, which is now cancelled.

Nothing is left running on keke or nozomi.

**Findings in force (no decision ids taken; the owner decides).**
* Cost: +1 syscall, +1 fault and +8-33 k kernel instructions per process. Per thread: +13.3 k kernel
  instructions, +1 fault, 4 KiB resident and 8 MiB of address space. User mode: +9 (longjmp), +12 per
  swapcontext switch and +45 per C++ throw through 10 frames.
* libniji's `ld.so`, `libc.so.6`, `libm.so.6` and most `libc.a` members are **not** SHSTK-marked, so static
  libniji programs are unmarked. Item A of the report is the precondition for any mode.
* The reference (Ubuntu 2.39) is off by default. It turns on only with `glibc.cpu.hwcaps=SHSTK`
  (`x86_shstk=on` alone does nothing). It locks, and refuses `dlopen` of unmarked objects unless
  `x86_shstk=on` (stays on) or `=permissive` (turns off).
* Required asm fixes, proven by a negative control: longjmp, swapcontext/setcontext and vfork. Thread exit,
  cancellation, signals and C++ unwinding work unchanged.

**Open questions.**
* Why libniji's three-library start adds +33 k kernel instructions against +8 k for the empty start
  (hypothesis: enabling before the libraries are mapped).
* posix_spawn's `clone3_run` under the shadow stack; untested.
* Freeing makecontext shadow stacks; the API has no hook.
* Disabling for the whole process when it has several threads (disable is per thread).
* Zen 3 numbers. To get them: copy `~/work/shstk-scope/run/` and `~/work/shstk-scope/niji/` from the N100 host
  (about 30 MB, no build) to natsumi, then run `bash ~/work/shstk-scope/measure.sh` 3 times (about 1 min each).
  This needs the coordinator's ledger claim.

**Do not redo.**
* The reference's tunable probes (logged).
* The asm survey: only `setjmp.S`, `context.S` and `vfork.S` break call/ret pairing; there is no Rust inline
  assembly.
* `LD_LIBRARY_PATH` is needed to run a test program on a non-installed libniji libc (R14-3 pins `libc.so.6` to
  the loader's prefix). It must go inside `env`, never on a host tool: perf, timeout.

**Next step.** The owner chooses a policy (report section 5; recommendation: option 2, opt-in matching the
reference). Then one implementation story for items A, B, D-H and M, and small ones for C and K.

**Plan rows to read.** D-21.1, D-21.3 (`plan/07-decisions/s21-contexts.md`), D-X.144, L14-12 / R14-9
(`plan/ph14/`).

**INFORMATIONAL (outside this story).** libniji's `pthread_create`+`join` costs 40.2 k user instructions against
the reference's 2.3 k (N100, shadow stack off), and `pthread_exit` from 5 frames 132 k against 48 k. This is
worth a performance story.
