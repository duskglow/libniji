# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State (run 7 done, leg 2).** Branch `task/dl-canonical-plt-7`; code head 2103dccb0, records/handoff commits on
top. Role mei:~/w/s4 (Skylake-SP). Full report: handoff/DL-CANONICAL-PLT-7-report.md (earlier runs:
handoff/DL-CANONICAL-PLT-6-report.md, -5-report.md). Record: `records/pending/DL-CANONICAL-PLT-2-1.toml`
(D-NEW in series D-27; its Cost sentence holds run 7's final counts). Plants: handoff/DL-CANONICAL-PLT-7-plants.txt
(8: run 6's seven re-aimed plus the SysV mark).

**Decisions in force.** Ruling #6659 (option b): the flag rides in the program view's hash table. It is a `u8`
`mark` in the tail padding of `GnuHash` / `SysvHash` (no spare `Hash` discriminant fits a GNU-carrying variant:
96 bytes against 88; a `bool` moved `Hash`'s niche and cost the walk +25k instructions at N=12), set in place by
`Object::mark_program` at start and in dlopen's adoption of the start's objects, read by `has_canonical_plt` only
in the cold `canonical_entry(i)` (non-exported candidates) and once per object in `relocate_first`. Compile-time
assertion: `Object` 0x128, `Scope` 0x30. `Scope` and the walk are base's.

**Measured (final code head, job dl-canonical-plt-7-step5, 101 runs vs next 1b7335a10):** plain PIE -635/-489/+1
instructions at N=0/3/12, faults equal ASLR off and on; canonical non-PIE +43.1k/+56.6k/+96.9k, +2 faults.

**Do not redo.** Run 6's Scope byte / `lookup_plain` / const-generic find (gone by ruling); the cold move alone
(a939e0bba, measured no change); counts on 2103dccb0.

**Open.** None for this story. INFORMATIONAL in the report: no spare discriminant; the bool-niche effect.

**Next step.** Coordinator: land after CI. If a later change touches `GnuHash`/`SysvHash`/`Hash`, recheck the
lookup closure and `Object::lookup` with the tracer (`~/work/DL-CANONICAL-PLT-7/sstep_run.sh` on mei).
