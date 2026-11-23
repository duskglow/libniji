# SHSTK-ENABLE handoff (final, leg 4)

**State.** Branch `task/shstk-enable`, stacked on SHSTK-OPT-IN (a2cee937a; merge base with next
0495c4684). **Code head 73d2ad2f6**; later commits are handoff/report only. Done; hand-back sent.

## Final-head jobs (mei, code head 73d2ad2f6)

`shstk-enable-lint4` (clippy --all-features -D warnings + `cargo xtask static`: rc 0),
`shstk-enable-plants4` (3/3 CAUGHT, 0 WARN), `shstk-enable-hottext4` (33 passed),
`shstk-enable-su7` (start-up rows); precheck GREEN on 853640137. FULL tier: the train's (csys
machine `.S` files). SUPERSEDED: `shstk-enable-xtest4` (rc 1: 7192 run, 7186 passed, 6 failed,
the host-locale six: mei has had no locale-archive since 11-23 06:32Z; same in mei CI gates
ci-gate-691c188e3 / ci-gate-a176ecda8), xtest3, plants3, lint3 (f4e60abca; lint3's rc 2 was a
trailing `cargo xtask affected` without a mode), hottext3 (killed, old head). Earlier-head evidence
whose code paths are unchanged: percall3 (6e647e491), nozomi n100e and cost1, su6.

## Results that matter for landing

* Start-up default mode vs a2cee937a (su7): static +7 instr:u, dynamic +30..+43 (0.02%, stable in
  the job, cause not isolated), faults unchanged.
* Layout: bin 0 `runtime::start` 5,658 -> 5,808 bytes (+150), `cpu::init` +10. The cold move did not
  shrink `start`; `&aux` passed to the cold function causes register churn (report, Layout).

## Decisions in force

D-21.6 (tunables, start-up decision, dlopen policy; amends D-21.3), D-21.7 (raw SSP, `uc_flags`,
signal-ucontext setcontext, makecontext shadow stacks kept; amends D-21.5), D-22.29 (clone3 with
`stack_size`). Differences rows: static `x86_shstk=off` veto; `uc_flags` with the shadow stack on.

## Open questions (RULING NEEDED)

1. makecontext shadow-stack leak: recommend a registry keyed to `uc_stack.ss_sp`.
2. Reference-static `x86_shstk=off` difference: recommend keep the veto.
3. -2 instr/call get/swapcontext fix-forward would write `uc_flags` in default mode: recommend not.

## Do not redo

Tunable probes, N100 functional and control runs, the cost1 table, percall, start-up counts, plants,
the `start` disassembly diff (mei `/mnt/zram/claude/shstk-enable/dis/`).

## Next step

Landing. Fix-forwards (each its own small story): `start` passes only `&tunables` and the cold
function reads AT_PHDR/AT_PHNUM via `process::auxval` (start -150 B target); inline `rdssp` in thread
create (about -5 of +13.2); CPUID test before the static opt-in image read (asked path on CPUs
without the feature); setcontext's FP restore from a kernel signal frame (INFORMATIONAL).
Full report: `handoff/SHSTK-ENABLE-report.md`.
