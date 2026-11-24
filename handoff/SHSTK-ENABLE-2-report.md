# SHSTK-ENABLE-2 report

Branch `task/shstk-enable-2`, code head 3461ce9b4 (merge of origin/next 237dbda92 at d5888c793).

## Commits

* d5888c793 Merge origin/next (the CI conflict: entry.rs, config.rs, s01_startup_clauses.rs,
  so.rs; both sides kept, see handoff).
* 0533b7a7e Unmap a makecontext shadow stack when its stack memory is reused (ruling #8100;
  D-21.7 (5) rewritten with the residual; D-21.6 cites ruling #8101).
* f72cdb513 Plants file `handoff/SHSTK-ENABLE-2-plants.txt`.
* 3461ce9b4 The fork child resets the registry lock only when makecontext used it
  (`fork::USED_CONTEXT`, D-18.8), so other children do not read the lock's page.

## Design (D-21.7 (5))

Registry: open-addressing table of `(ss_sp, base, len)` in anonymous memory (128 slots in one
page first, doubled when one more key would pass half), one `RawMutex`. makecontext, after mapping a
shadow stack, records it; a known key returns the earlier shadow stack, which is unmapped outside
the lock unless the caller's SSP is in it. Only registry records are unmapped, never
`__ssp[1..2]`. Residual: memory freed and never given to makecontext again keeps its shadow
stack; a stack the caller runs on is left mapped; an `ss_sp` offset into the same memory is a
different key; a new key when the table cannot grow is unrecorded. Default mode (no shadow
stack): makecontext's path unchanged (the registry is inside the cold `shadow_stack`); fork child:
one more test of the `USED` word it already loads.

## Evidence

* Unit test `context::tests::makecontext_shadow_stack_registry` (keke kit job
  `shstk-enable-2-kit1`, on 0533b7a7e; also in xtest2).
* **N100** (`nozomi:shstk-enable-2-n100a`, Intel N100, kernel 6.8, user_shstk; static
  s21_shadow_stack linked against 0533b7a7e's libc.a on keke): `shadow stack on: all steps
  passed`, exit 0. **Control** (same tree, makecontext never reaches `registry::replace`):
  `FAIL ... kb_last == kb_first`, 8,576 kB after the first reuse, 12,608 kB after 64 (63 x 64 KiB
  leaked), exit 1. Not re-run on 3461ce9b4: that commit adds only the `USED` bit (one atomic
  `fetch_or` the first time, a test in the fork child); the registry logic is unchanged.
* Final-head jobs on keke (3461ce9b4): see the hand-back `JOBS:` line and the JSON report.
* Layout: not diffed against the merge base this run (no start-up function changed by this run's
  code apart from the fork child's bit test; the merge adds next's own code). hottext2 checks the
  hot-text contract.

## Rulings / informational

* #8101 provisional (owner asked); #8102 nothing to change.
* INFORMATIONAL: one local edit used `sed -i` on D-21.7's `tests` line (thread rules require the
  Edit tool); the result was checked and is as intended.
