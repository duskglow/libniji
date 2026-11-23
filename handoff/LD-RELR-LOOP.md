# LD-RELR-LOOP: RELR applied a run at a time, words added in place; libc.so.6 packed

**State (LD-RELR-LOOP-3):** branch `task/ld-relr-loop-3` = `task/ld-relr-loop-2` (2b2645b6b) with origin/next
8e1837914 merged in (clean merge, no conflict, no code change of mine this run). Role mei `~/w/s2`.
Decision in force: D-27.87 (both earlier runs), ruling #7402. Full report: handoff/LD-RELR-LOOP-report.md
("LD-RELR-LOOP-3").

## The change (unchanged since LD-RELR-LOOP-2)
* `elf.rs` `Relrs::runs`; `reloc.rs` `pass1` -> `relr_entry` / cold `relr_checked`; `Image::add_bits` (default
  `add_bits_by_word`, niji-boundary `ProcImage` adds in place); `boot.rs` `Writer::add_bits` -> `Memory::add_bits`
  (`bootstrap::Mapped` adds in place); `xtask/src/so.rs` links libc.so.6 with `-z pack-relative-relocs`;
  `hot_text_products` `DYN_START_INSTR` tightened.

## The CI red of LD-RELR-LOOP-2 (train 932, 11 loader tests: the program missing from the address tables)
* On this branch merged with origin/next 8e1837914: all ten failing niji-ld tests (atexit_owner, dlclose,
  dl_find_object_index, dlopen_concurrent, dlopen_process_objects, runpath_search suites, 27 tests) and the
  `s27_dladdr` conformance program pass (jobs ld-relr-loop-3-repro2, -repro3).
* Reproduced on a scratch merge of `task/ld-relr-loop-2` into next d258cd270 (next just before
  LDSO-PROGRAM-OPTIONS-REST-2 landed, 22:17Z): atexit_owner fails as in CI, `got 'dA aX', want 'dA mainH aX mainC'`
  (ld-relr-loop-3-repro4).
* Same scratch merge with libc.so.6 NOT packed (so.rs flag removed): still fails (ld-relr-loop-3-repro5). So the
  packed libc is not the trigger.
* In that failing build: every one of ld.so's 415 RELR words holds file value + bias at run time (probe, 0
  mismatches); the test program has no DT_RELR (readelf); the helper libraries are built by the system compiler
  without it. The only RELR code that runs in the failing test is ld.so's self-relocation, verified correct.
* A probe (dladdr / _dl_find_object of `main` after dlopens, libc named by path as the tests do) finds the program
  in that build. Not reduced further (budget).
* Conclusion (evidence above, cause NOT found): the branch changes only ld.so's code layout on the failing path;
  a layout-sensitive defect in the loader's program/address tables on next d258cd270 shows with it (and with
  LD-OBJECT-ONCE-2's change), and is masked again on next 8e1837914. It may still be latent on next.

## Next step
Land (CI train). If the eleven tests fail again in some later combination: rebuild the failing merge (recipe in
the report), and debug ld.so in the atexit_owner `net` run (gdb on our own ld.so is allowed) to find what
overwrites or misreads the program's entry; that is a loader-tables defect, not RELR.
