# TB-FOREIGN-WORDS-LOW: errno, TID, dtor, heap and node words (0x80-0x9b) in a block Liella did not make

State: branch `task/tb-foreign-words-low-12`, code head a049c2f4f (-11's 63ce23318 plus the fix below); later
commits touch handoff/ only. Decisions in force: D-X.58, D-X.60, D-X.62, rulings #4835, #4849, #4851, #4779 (<= +2
per one-access call vs next, judged on Release #4889), #5325, #5378, #4347, #5697. The owner's safety requirement
(words 0x80-0x9b) was finished in -1..-10. This run fixed the layout cost CI train 709 found. Older reports:
handoff/TB-FOREIGN-WORDS-LOW-report.md (-1..-4), the git history of this file (-5..-11).

## -12 (host mei, Skylake-SP, the CI host class; RAM copies, fast products)
- Ruling #5697 asked whether the GLIBC_TUNABLES name table is a start-up key in next's mei build. **No.**
  The table is `config::GLIBC_TUNABLE_TYPES` (41 x 40 = 1640 bytes of .data.rel.ro). Only
  `config::warnings<runtime::start::{closure}>` and `config::glibc_tunable_type` address it, and neither is a
  start-up text key. It lies past the start-up relro cluster: next 0x851c08 > cluster end 0x851ab8, and this
  branch's head built alone 0x862938 > 0x861ab0. Jobs `tb-foreign-words-low-12-meinext` and `-meihead` ran
  with lowered budgets in scratch copies only.
- The trace environment is not the cause. The text trace runs `page_faults true` with exactly `LANG=C`
  (trace_gdb.py starts from `unset environment`), the job environment has no GLIBC_TUNABLES, and an
  `anon.*` constant is never a key (startup_data.py). A gdb read watch on the table's first word was not hit
  with GLIBC_TUNABLES unset.
- The access is static, from inlining in the merged tree. The merged tree is next f6cab48d7 plus this
  branch's diff, built from `git archive` + `git apply` in RAM (no git merge). There the compiler inlines
  `warnings`, and through it `glibc_tunable_type`, into `runtime::start<false>`, which is a fast start-up text
  key. Its operand then addresses the table, so startup_ro.rs moves the table's section to the head of the
  relro cluster (0x850030) and STARTUP-DATA-WINDOWS counts it: 8 written pages > 7. The objects and
  addresses are the same as in CI 709 (job `-meim0`). Start-up never reads the table while GLIBC_TUNABLES is
  unset.
- Fix a049c2f4f: `#[inline(never)]` on `glibc_tunable_type` (the one function whose code addresses the table)
  and on `warnings` (runs only when GLIBC_TUNABLES is set). With the fix, the merged tree (job `-meim1`) gives
  `so` rc 0 and STARTUP-DATA-WINDOWS at 7 of 7 pages (77/77 objects). The cluster is next's own,
  0x850000-0x851ab8, with 1352 bytes to spare on its second page. No budget changed (ratchet kept).
- Plants (handoff/TB-FOREIGN-WORDS-LOW-12-plants.txt, job `-plants`): both are EQUIVALENT by design, because
  either guard alone keeps the table off start-up code. The m0/m1 pair is what proves the pair of guards.
- Gates: step 0 `-lint0` (xtask clippy + static on the inherited head) rc 0. The CI gate `-cigate` runs on
  the role synced at a049c2f4f: static, `cargo xtask test`, then the release `so` + hot_text_products.
  Static passed. `cargo xtask test` was still running at hand-back (cold role build), so the dispatcher
  reads the result from jobs.tsv. The probe jobs (`-meinext`, `-meihead`, `-meim0`, `-meim1`) are evidence only.

## Next step
Land. Still open from -10, outside this story: the text before the 32 KiB-aligned string kernels is about
9 KiB over the boundary next sits under (LAYOUT-RETHINK's question), and the remaining `OwnPages`
monomorphisations (memalign family, `first_cache`).

## Not to redo
-1's audit; the registry/FOREIGN/OwnPages/asm design; -7's SPARE split; -8's Haswell-EP comparison; -10's mei
fault-set comparison; -11's keke page lists. -12's mei probes: scripts in mei ~/work/TB-FOREIGN-WORDS-LOW-12/
(lay12.sh, lay12m.sh, ana12.py, lower12.py, m0.diff, m1.diff); outputs in
/mnt/zram/claude/TB-FOREIGN-WORDS-LOW-12/out-{next,head,m0,m1}/ (ana.txt, drr-refs.txt, layout-check.txt).
