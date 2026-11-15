# DL-ATEXIT-OWNER: registering exit/fork handlers without libc_nonshared.a (owner rulings #4860-#4862)

**State (run 3, DL-ATEXIT-OWNER-3, 2026-11-15).** Branch `task/dl-atexit-owner-3` (continues `-2` and `-1`), role
mei:~/w/s3. Head and job verdicts: handoff/reports/DL-ATEXIT-OWNER-3.json. Read first: handoff/DL-NONSHARED-OPTOUT.md
(design, probes P1-P3), this note, D-27.29 (records/decisions/D-27/D-27.29.toml: the whole design in force).

## Built (runs 1-3)
* Exports `atexit`/`at_quick_exit`/`pthread_atfork@LIELLA_OWNER_1` (non-default; `csys/owner/owner.S`), attribution
  through `_dl_find_object` (`crates/boundary/src/owner.rs`), the unload net on `dlclose` (`__liella_owner_unload`).
* Run 3: (1) the red `dl-atexit-owner-2-check3` (FN-ALIGN-EXPORTS): the shims were `.p2align 4` in one `.text`; each
  now starts a 64-byte line in its own section (as once.S / compat stubs). Verified: `dl-atexit-owner-3-so1`, all 12
  layout contracts hold. (2) The opted-in PROGRAM link failed (never seen before: check3 stopped earlier): the
  three-symbol `libc-owner.so` and `libc.so.6` share a soname and ld loads one object per soname, so every other libc
  name was undefined. `libc-owner.so` is now libc.so.6 linked a second time (stripped) with the three entries at the
  DEFAULT version (`owner.S -DLIELLA_OWNER_LINK_STUB`, xtask/src/so.rs `link_libc`); the script names only it;
  `csys/owner/stub.S` is gone. (3) #4861: `LIELLA_NONSHARED_NOTICE=1 cargo xtask so` adds `.gnu.warning.<name>` and an
  allocated retained `.note.liella.nonshared` (owner `Liella`, type 1) to the three registration members
  (`csys/nonshared/notice.h`; checked both ways by `nonshared_notice_errors`); `LIELLA_LD_REPORT_NONSHARED` (non-empty,
  not secure) makes ld.so name each loaded object carrying the note, at start and at dlopen
  (`liella_ld::start::uses_nonshared_stubs`, `report_nonshared` in crates/boundary/src/ld.rs). (4) User docs:
  docs/install-packages.md (opt-in, migration path, nothing deprecated by default); spec/behaviour/dynlink.md paragraph.
* Tests: `crates/ld/tests/atexit_owner.rs` (2), products `s1_2_c_5_owner_optin_link_and_reference_refusal`,
  `s1_2_c_5_nonshared_notice_warning_and_loader_report` (new; passed in `dl-atexit-owner-3-t2` before fix (2)).
* Plants: handoff/DL-ATEXIT-OWNER-2.plants (7) and handoff/DL-ATEXIT-OWNER-3.plants (5: notice on by default, report
  missing at start / at dlopen, report without the setting, link interface not default). Both run in
  `dl-atexit-owner-3-plants`.
* Records: D-27.29 (body updated run 3), differences row, records/pending/DL-ATEXIT-OWNER-2-1.toml (COMPAT-NEW).

## Open (next run, in order)
1. Read `dl-atexit-owner-3-check` and `dl-atexit-owner-3-plants` on mei (`~/logs/<label>.log`). Fix any red from
   the logs. The check runs clippy, static, so, the story's tests, `cargo xtask test` and the FULL_PROFILE
   hot_text_products check.
2. Not planted: a registration under the loader lock that deadlocks (module_of is lock-free by construction). Find a
   one-site plant or record it as not plantable. owner.S cannot be planted (`xtask plants` takes only
   `liella_machine_` labels); plant `link-interface-not-default` covers its `@@` build through xtask.
3. `liella_legacy::owner_proto` + crates/core/tests/nonshared_optout_proto.rs (prototype, selftest) still there: drop
   them once the product tests are green (removing tests: say so in the hand-back).
4. The direct-tier gate (`cargo xtask affected --run --base origin/next`) on the final head.

## Not to redo
Probes P1-P3, the prerequisite check, the compat-export registration, the alignment fix, the link-interface design.
