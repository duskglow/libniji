# DL-ATEXIT-OWNER: registering exit/fork handlers without libc_nonshared.a (owner rulings #4860-#4862)

**State (run 5, DL-ATEXIT-OWNER-5, 2026-11-15).** Branch `task/dl-atexit-owner-5` (continues `-4`, `-3`, `-2` and
`-1`), role mei:~/w/s3. Head and job verdicts: handoff/reports/DL-ATEXIT-OWNER-5.json. Read first: handoff/DL-NONSHARED-OPTOUT.md
(design, probes P1-P3), this note, D-27.29 (records/decisions/D-27/D-27.29.toml: the whole design in force).

## Built (runs 1-4)
* Exports `atexit`/`at_quick_exit`/`pthread_atfork@LIELLA_OWNER_1` (non-default; `csys/owner/owner.S`, each entry on
  its own 64-byte line), attribution through `_dl_find_object` (`crates/boundary/src/owner.rs`), the unload net on
  `dlclose` (`__liella_owner_unload`).
* Link-time interface `target/liella/liella-owner/` (run 3): `libc-owner.so` is libc.so.6 linked a second time
  (stripped) with the three entries at the DEFAULT version (`owner.S -DLIELLA_OWNER_LINK_STUB`, xtask/src/so.rs
  `link_libc`), and a `libc.so` script naming only it. One soname: the link editor loads one object per soname.
* #4861 (run 3): `LIELLA_NONSHARED_NOTICE=1 cargo xtask so` adds `.gnu.warning.<name>` and `.note.liella.nonshared` to
  the three registration members; `LIELLA_LD_REPORT_NONSHARED` (non-empty, not secure) makes ld.so name each loaded
  object carrying the note, at start and at dlopen. User docs: docs/install-packages.md; spec/behaviour/dynlink.md.
* Run 4: (1) the red `dl-atexit-owner-3-check` (and the baseline of run 2's plants): crates/ld/tests/atexit_owner.rs
  still assembled the deleted `csys/owner/stub.S`. Module A is now linked against the product `libc-owner.so`, as a
  user's link would be; the program and S are linked against a soname-less copy of libc.so.6 (they name it by path:
  the bare soname is pinned to the installed prefix, R14-3 (a)), and the copy is replaced by the unmodified
  libc.so.6 before the run, so A's `libc.so.6` need and its `LIELLA_OWNER_1` version need match it by `DT_SONAME`
  (the loader's `provider` matches soname or loaded-by name). (2) The prototype `liella_legacy::owner_proto` and its
  8 selftest tests (crates/core/tests/nonshared_optout_proto.rs) are dropped; the product tests replace them.
* Run 5: CI could not merge next (conflict in xtask/src/so.rs with SL-SONAMES' forwarding aliases). Merged
  origin/next (8261e29b2): both sides kept; `link_libc` passes next's `--defsym` options (`forward_defsyms`), so
  libc-owner.so stays the same link as libc.so.6 apart from the owner object (the version script names the
  forwarding aliases at LIELLA_PRIVATE in both links). Gate, start-up checks and the run 4 plants file rerun on the
  merged head (labels in the report json).
* Tests: `crates/ld/tests/atexit_owner.rs` (2), products `s1_2_c_5_owner_optin_link_and_reference_refusal`,
  `s1_2_c_5_nonshared_notice_warning_and_loader_report`.
* Plants: handoff/DL-ATEXIT-OWNER-2.plants (7) and handoff/DL-ATEXIT-OWNER-3.plants (5), both in
  `dl-atexit-owner-4-plants`.
* Not plantable (run 4, item closed): "a registration under the loader lock that deadlocks". The loader lock
  (`Reentrant` in crates/boundary/src/ld/dlopen.rs) is re-entrant for its holder, and `module_of` takes no lock
  (`_dl_find_object` reader). No one-site change in the owner path can deadlock a registration made from a
  constructor under `dlopen`; the only one-site deadlock is in the lock's own re-entry check, which every nested
  loader call (constructors calling `dlopen`) already exercises. The `alarm(120)` hang guard in atexit_owner.rs stays.
* Records: D-27.29, differences row, records/pending/DL-ATEXIT-OWNER-2-1.toml (COMPAT-NEW).

## Open (next run, in order)
1. If any run-5 job is red, read its log on mei (`~/logs/<label>.log`) and fix it on the head.
2. Nothing else is open in the story. Possible follow-up (not in this story): an end-to-end test with a
   `-L liella-owner -lc` link run under Liella's loader once it searches `libc.so.6` beyond the pinned prefix.

## Not to redo
Probes P1-P3, the prerequisite check, the compat-export registration, the alignment fix, the link-interface design,
the deadlock-plant question above.
