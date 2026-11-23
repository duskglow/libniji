# LD-TABLES-LAYOUT (branch task/ld-tables-layout)

State: done; fix, tests, plants, decision row and report on the branch (base next 9d12e7ee3). Full report:
handoff/LD-TABLES-LAYOUT-report.md.

## Decisions in force
* D-27.89 (new, amends D-27.75): the program the kernel started and `ld.so` own only their segment runs in
  `_dl_find_object`, `dladdr`/`dladdr1` and the address index; objects libniji maps keep D-27.75's whole span.
* Probe R-6 (spec/behaviour/dynlink.md, `tools/probe/dl/holes/probe.c`) settles the reference's behaviour.

## Not to redo
* The cause is found and proven (report): a library mapped into a hole of a kernel-mapped object (here
  `ld.so`'s 60 KiB text-to-rodata hole) was attributed to that object. It explains train 929/932.
* The reproduction (d258cd270 + task/ld-relr-loop-2) and the same merge with the fix are measured; the
  diagnostic worktree in the run's scratch is removed at the end of the run.

## Open
* INFORMATIONAL: `ld.so`'s 60 KiB hole between its text and rodata is a layout matter now (LAYOUT-LDSO-*),
  no longer a correctness one.

## Next step
Land through the train. Nothing else is owed by this story.
