# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State (run 4, checkpoint at context).** Branch `task/dl-canonical-plt-4` (= dl-canonical-plt-3 + origin/next
a67d30d2f merged + run-4 commit cc062fcfd, WIP). Role keke:~/w/n1. Scripts on keke ~/work/DL-CANONICAL-PLT-4/:
`lint.sh` (static, clippy), `step.sh <tag>` (builds the base = next a67d30d2f once in ~/work/DL-CANONICAL-PLT-4/base,
then the role's products, the canonical_plt tests, `count.sh`), `count.sh` (RAM copies at equal-length paths, 21 runs,
ASLR on/off, N=0/3/12; then every N=0 page fault mapped to object/perms/file page with `pfmap.py`, base vs branch).
Pending record `records/pending/DL-CANONICAL-PLT-2-1.toml` (D-NEW, D-27): its body still describes run 2; update it
to the design below and add the new test to `tests`.

**Run 4 finding (job dl-canonical-plt-4-step1, head 7ff6d42d4 = run 3 + next merged; ASLR off, 21 runs, zero spread).**
Instructions below base at every N (N=0 9027952 -> 9016192, -11.8k; N=3 -11.8k; N=12 -12.0k). Faults N=0 80 -> 85,
N=3 97 -> 100, N=12 133 -> 136 (ASLR on medians 82->85, 98->100, 134->136). layout-bins.tsv and hot-text.signatures
identical; libc.so.6's layout is identical (same section offsets). Every extra page comes from ld.so's growth (.text
289664 -> 310895, +21 KB): ld.so has no traced layout, so link order decides; the run-3 `#[cold]` functions are placed
in `.text.unlikely` AHEAD of `niji_machine_ld_entry` (entry moved from file 0x1000 to 0x3000), inlining shifted
(`relocate_first` +2984 with pass1 inlined, `link` +3142, `list` +2773, `exit_order` newly out of line +4453), and the
bigger ld.so image moves libc's address (ASLR off) into other fault-around windows (new fault: libc page 0x435000,
`.dynamic`, read by `Elf::dynamic`). Diff (N=0): ld.so r-xp 5 -> 6 pages, anon +2, stack +1, libc r--p +1.

**Design now (cc062fcfd, NOT YET MEASURED).** The plain path is next's: `pass1` (now `#[inline(never)]`, as next
generates it) and the deferred loop run unchanged; when `syms[0].has_canonical_plt()`, the cold `canonical_slots`
then visits the JUMP_SLOTs: `slot` reads the field back and skips any that does not point into the program (only a
canonical entry can); else `slot_target` re-resolves (one `resolve` call site, a 2-step loop: a first answer that is
an undefined symbol, the program's entry, is resolved again without the program) and writes / defers it, with the
protected check (`protected_canonical`) in pass 1. `pass1_canonical`/`tables_canonical` are gone.
`Object::program` is `#[inline(never)]` (was inline(always), which grew `link`). New test
`s27_1_g_0_canonical_plt_entry_for_ifunc_definition` (IFUNC definition: the slot is bound in the deferred pass;
probe-settled on keke: the reference passes all checks, lazy and LD_BIND_NOW).

**Next step.** (1) Read job `dl-canonical-plt-4-step2` on keke (lint, then step.sh m2): canonical_plt 5 tests
green? counts and the fault-page diff vs base. (2) If faults are still above base: reduce the remaining ld.so growth
(sizes: `nm -S --size-sort` of /dev/shm/claude/DL-CANONICAL-PLT-4/m2/{base,new_}/ld-linux-x86-64.so.2); consider
dropping `#[cold]` on protected_canonical/canonical_slots (`.text.unlikely` goes first in ld.so). If it cannot reach
base: RULING NEEDED with this diff (ld.so is ungrouped, so any loader growth moves its start-up pages; a fix is a
traced/ordered ld.so layout, outside this story). (3) Update the pending record body. (4) Plants:
`handoff/DL-CANONICAL-PLT-4-plants.txt` (13 plants, rewritten for this design) in a NEW dir
/dev/shm/claude/DL-CANONICAL-PLT-5/plants-1, `--timeout 900`. (5) Gate as ~/work/DL-CANONICAL-PLT-2/gate.sh on mei
did (static, clippy, full clippy line, records check, affected --base origin/next, xtask test, full-profile so +
hot_text_products, plain so): copy it to keke with `cd ~/w/n1`.
