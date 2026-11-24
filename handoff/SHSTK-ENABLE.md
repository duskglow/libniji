# SHSTK-ENABLE handoff (SHSTK-ENABLE-2, final)

**State.** Branch `task/shstk-enable-2` (continues `task/shstk-enable`, code head 73d2ad2f6 there),
origin/next 237dbda92 merged in (d5888c793). Code head **3461ce9b4**; later commits are
handoff/report only. SHSTK-OPT-IN (a2cee937a) is on next.

## This run

1. **CI conflict resolved** (merge of origin/next, d5888c793): both sides added a tunable and a
   NIJI_PRIVATE export; kept both everywhere (`Tunables` fields and parse arms, the loader's
   start-up reads in `ld/entry.rs` (shadow-stack request, then the static TLS surplus), the
   `s1_8_c_0` literal, `LIBC_PRIVATE` in `xtask/src/so.rs`).
2. **Ruling #8100 implemented** (0533b7a7e, 3461ce9b4): makecontext's shadow stacks are recorded
   in a library-owned registry keyed by `uc_stack.ss_sp` (`boundary/src/context.rs`, module
   `registry`); a reuse of the key unmaps the earlier shadow stack unless the caller runs on it.
   The fork child resets the registry lock only after makecontext used it (`fork::USED_CONTEXT`,
   D-18.8). Residual and reuse rule in D-21.7 (5).
3. **Ruling #8101** cited in D-21.6 (veto kept, provisional). **#8102**: nothing to change
   (default mode stays exact).

## Decisions in force

D-21.6, D-21.7 (amended this run: item (5)), D-22.29. Differences rows: static `x86_shstk=off`
veto; `uc_flags` with the shadow stack on.

## Open questions

None new. #8101 waits for the owner (provisional).

## Do not redo

The N100 runs (`nozomi:shstk-enable-2-n100a`: reuse step passes, control fails with +4,032 kB),
the earlier run's tunable probes, cost table, percall and start-up counts.

## Next step

Landing (FULL tier is the train's: csys machine `.S` files). Fix-forwards stay as SHSTK-ENABLE
listed them: `start` passes only `&tunables` to the cold function; inline `rdssp` in thread create;
a CPUID test before the static opt-in image read; setcontext's FP restore from a kernel signal
frame (INFORMATIONAL).
Report: `handoff/SHSTK-ENABLE-2-report.md`; earlier: `handoff/SHSTK-ENABLE-report.md`.
