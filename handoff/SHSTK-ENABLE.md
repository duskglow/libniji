# SHSTK-ENABLE handoff (final, leg 3)

**State.** Branch `task/shstk-enable`, stacked on SHSTK-OPT-IN (a2cee937a; merge base with next
0495c4684). Code head f4e60abca (the later commits are handoff/ only). Role mei:~/w/s2.
Full report: `handoff/SHSTK-ENABLE-report.md` (probe table, N100 runs, per-call, start-up and N100
cost tables, rulings). Plants: `handoff/SHSTK-ENABLE-plants.txt` (3 CAUGHT).

## Decisions in force

* D-21.6: tunables (`hwcaps=SHSTK`, `x86_shstk=on|off|permissive`), start-up decision (AND of every
  start object's marking; static from its own note), dlopen refuse/load/permissive. Amends D-21.3.
* D-21.7: raw SSP in `__ssp[0]`; `uc_flags` = 0 with a shadow stack only; setcontext on a kernel
  signal ucontext pops to the innermost signal frame's interrupted SSP; makecontext shadow stacks stay
  mapped (ruling requested). Amends D-21.5.
* D-22.29: threads use `clone3` with `stack_size` while the creator has a shadow stack.
* Differences rows: static `x86_shstk=off` veto; `uc_flags` with the shadow stack on.

## Jobs (final code head f4e60abca, mei unless named)

`shstk-enable-xtest3` (cargo xtask test), `shstk-enable-hottext3` (full-profile so + hot_text_products),
`shstk-enable-plants3` (3/3 CAUGHT), `shstk-enable-lint3` (clippy --all-features, static, records
check: clean), `shstk-enable-percall3` (6e647e491, same code paths), `shstk-enable-su6`,
`nozomi:shstk-enable-n100e`, `nozomi:shstk-enable-cost1`. Affected tier escalates to FULL
(context.S, entry.S, setjmp.S, thread_start.S): the train's.

## Open questions (RULING NEEDED)

1. makecontext shadow-stack leak: recommend a registry keyed to `uc_stack.ss_sp`.
2. Reference-static `x86_shstk=off` difference: recommend keep the veto.
3. -2 instr/call get/swapcontext fix-forward would write `uc_flags` in default mode: recommend not.

## Do not redo

Tunable probes, N100 functional and control runs, the cost1 table, the percall and start-up counts.
Kit scripts: mei/nozomi `~/work/SHSTK-ENABLE/{build.sh,run.sh,startup.sh,cost/cost.sh}`.

## Next step

None for this story beyond landing. Follow-ups: default-on data for the owner (#154) is the cost
table here; fix-forwards: inline `rdssp` in thread create (about -5 of +13.2); setcontext's FP
restore from a kernel signal frame (INFORMATIONAL in the report).
