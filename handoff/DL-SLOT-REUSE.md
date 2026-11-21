# DL-SLOT-REUSE: reuse freed loader slots below the top (run 1, leg 2: final)

**State.** Branch `task/dl-slot-reuse` from next `d5764940d`; role mei:~/w/s4. Code done; decision D-27.74
(amends D-27.73); full report with the churn table in `handoff/DL-SLOT-REUSE-report.md`; plants in
`handoff/DL-SLOT-REUSE-plants.txt`.

**What the change does (`crates/boundary/src/ld/`).**
- `dlopen.rs` `load_set`: the set is found and staged at the top as before; when `Loader::holes` (freed slots below
  `n`, kept by `reclaim` and `unwind`) >= the set's size, cold `settle()` moves it into the first run of freed
  slots that fits (first fit; `freed` = past the readers' grace period). Validated before anything moves; nodes,
  records, reader entries (`readers::restage`), dependency lists (`table.rs` `Arena::rewrite`) renumbered.
- Load order kept (P-12): `Loader::reordered` / `readers::REORDERED` make the chain tail, exit order and
  `dl_iterate_phdr` follow the chain. Hot cost: `dl_iterate_phdr` +4 instructions a call; `load_set` one compare.
- `reclaim` keeps the name arena's top over paths of unloaded objects in their grace period (`!freed`).
- `crates/ld/tests/dlclose.rs` s27_1_c_4 check 72: was "the opens wait at three quarters of the table behind an
  ever-present reader" (the backstop for holes never reused); with reuse the top never gets there (failed 10/10 in
  `cargo xtask test`). Now: the 600 pairs get <= 128 distinct handles. The three-quarters wait remains for sets no
  run of holes holds; no test reaches it now (RULING NEEDED in the report).

**Done (do not redo).** Churn table base/head/ref (K = 8, 100, 600; N to 100k), count rows, layout diff (libc
layout identical modulo LLVM hash suffixes; ld.so text +7,656 B), plants (6, incl. the room check), direct gate,
`cargo xtask test` (red on check 72 -> fixed above), D-27.74.

**Final jobs (mei).** lint4, gate3, plants2, counts2, hottext1 green. `cargo xtask test`: xtest2 red only on
`niji::dns_products resolv_equals_the_reference` hung in the native pass (ended after 29 min); xtest3 is the rerun
(pending at hand-back). If xtest3 is red on the same test, check it on origin/next before touching this branch.

**Open.** RULING NEEDED: (1) `dl_iterate_phdr` +4 instructions a call within 512 objects (brief asked "same
counts"; avoiding it needs the readers' table kept in load order); recommend accept. (2) the three-quarters wait
has lost its only test; recommend a follow-up test with sets larger than the holes.

**Next step.** Path and dependency spans: first-fit gap placement with the same grace-period rule, so the address
space under churn is bounded too (head still grows ~100 B per open; base ~2 KB). A follow-up story.

Plan rows to read: D-27.73, D-27.74.
