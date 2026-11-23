# SHSTK-ENABLE handoff (leg 3 -> leg 4)

**State.** Branch `task/shstk-enable`, stacked on SHSTK-OPT-IN (a2cee937a; merge base with next
0495c4684). **Code head 73d2ad2f6** (synced to mei:~/w/s2). Everything is done except collecting the
final-head jobs and handing back.

## Leg 4: exact steps

1. `~/bin/jobwait shstk-enable-lint4 shstk-enable-plants4 shstk-enable-xtest4 shstk-enable-hottext4`
   on mei (started 16:04Z; chain lint4 -> xtest4 -> hottext4, ~55 min; plants4 parallel, ~10 min).
   Expected: lint4 clean; plants 3 CAUGHT (copy `/mnt/zram/claude/shstk-enable/plants-4/results.txt`
   to handoff/SHSTK-ENABLE-plants.txt); xtest4 = 6 host-locale failures only (catalogs_products,
   locale_data x2, locale_products, printf_corpus_products, s7_2_f_0: the same in mei CI gate logs
   ci-gate-691c188e3 / ci-gate-a176ecda8; xtest3 on f4e60abca: 7192 run, 7186 passed, 6 failed).
2. Layout (§9): `diff ~/wl/src/a2cee937a08b555d1a70cf33b5fa7be4a00c70b7/target/product/fast/layout-bins.tsv
   ~/w/s2/target/product/fast/layout-bins.tsv`. On f4e60abca bin 0 had runtime::start 5658 -> 5799 and
   cpu::init 50 -> 60; 73d2ad2f6 moves the decision cold (start should be back near 5658). Explain
   what remains (cpu::init +10: Tunables gained `shstk`/`x86_shstk`?) in the report.
3. Optional (1 job, ~1 min): `sh ~/work/SHSTK-ENABLE/startup.sh ~/wl/src/a2cee937a08b555d1a70cf33b5fa7be4a00c70b7/target/niji
   ~/w/s2/target/niji` as `shstk-enable-su7` after hottext4 (target/niji must be the fast build:
   run it before hottext4 rebuilds, or rebuild `cargo xtask so`); update the static +8 row.
4. Update report/json (head, jobs, gate line, layout result), commit, push, sync, `cargo xtask precheck
   --jobs <labels> --head <sha>` on mei, `ops handback-check`, hand back. FULL tier escalates: write
   `FULL: the train's (csys machine .S files)`. SUPERSEDED: xtest3, plants3, lint3 (f4e60abca),
   hottext3 (killed, old head).
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
