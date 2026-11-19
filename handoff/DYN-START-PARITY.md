# DYN-START-PARITY (handoff after leg 3: final)

## State
Branch `task/dyn-start-parity`, final head = the commit that carries this note (code head 503c206e0). Leg 3 found
that DL-23-2 had landed in next (17:20Z, next d496d4c25), which the brief says to merge first: merged origin/next
(7763acb6b, clean; reloc.rs, boundary ld.rs and ld/table.rs touched on both sides, no conflict). The merge brought
DL-23-2's +1 start fault (vDSO header page, recorded in its report) to the startup-only program: gate2 read fast 78
> 77. Bound set to the merged levels [78, 74] (503c206e0), still under next's [89, 86].
Final jobs on mei (role `~/w/s1`, Skylake-SP), head 503c206e0: `dyn-start-parity-release3` (clippy + static +
NIJI_FULL_PROFILE=1 so + hot_text_products, release), `dyn-start-parity-gate3` (direct tier, `affected --base
origin/next --run`), `dyn-start-parity-xtest3` (cargo xtask test), `dyn-start-parity-plants3` (9 plants).
Superseded: lint3 (trace markers, fixed in 8a2301180; final.sh then stopped), lint4/gate2/plants2 (merged head
7763acb6b; gate2 red on the fault bound, fixed), startup2 (stopped: its head was replaced), ldtest1 (test bug,
fixed), ldtest2 (ETXTBSY race), lint/lint2 (clippy casts; lint2 ran on an unsynced tree), dyntests/dyntests2
(measurement runs), so0-so3 (builds), chk1/chk2.

Decisions: D-27.63 (lookup view / relocation pass read each table once), D-27.64 (16-entry inline start tables).
Plants: `handoff/DYN-START-PARITY-plants.txt`. Report: `handoff/reports/DYN-START-PARITY.json`.
Host scratch `/mnt/zram/claude/DYN-START-PARITY/` (rel2, rel3 = release products of this branch; prog-* = test
programs), scripts `~/work/DYN-START-PARITY/parity.sh` (execcount, `GLIBC=1` adds the reference) and `prof.sh`
(perf on the exec'd program only).
RULING NEEDED: link libc.so.6 with packed relative relocations (RELR); see the report. Verdict: COMPLETE, target
(parity) not reached; the next steps below are the follow-up story.
Not re-measured after the merge: the execcount table below (pre-merge); DL-23-2 adds its own +1 fault to the
no-vDSO rows.

## Measurements (execcount, ASLR off, median of 101, release products, Skylake-SP)

| build | empty main instr | 3 libs instr | clock_gettime instr | faults (pie0/pie3/time) |
|---|---|---|---|---|
| glibc 2.39 (host) | 106,501 | 132,275 | 108,608 | 49 / 62 / 50 |
| origin/next 255a6901d | 8,151,153 | 8,374,348 | 8,194,191 | 75 / 91 / 80 |
| leg 1 (525af12ad) | 2,049,314 | 2,271,815 | 2,083,381 | 75 / 91 / 80 |
| leg 2 RELATIVE path + count once (rel2) | 1,605,172 | 1,827,135 | 1,638,997 | 75 / 91 / 80 |
| leg 2 + loop + INLINE 16 (rel3, ee554f0c2) | 1,576,900 | 1,798,519 | 1,610,698 | 65 / 81 / 70 |

In-test (hot_text_products, empty main, median of 21): Skylake-SP fast 1,718,270 / release 1,598,681 (host libc
101,863); Zen 1 release 1,598,530 (host libc 96,002; keke, release binaries copied from mei, run in /tmp).

## Profile now (rel2, release, by symbol; perf on the exec'd program only)
pass1 41% (the RELATIVE loop: still about 150+ instructions per entry: Rela decode with run-time byte order,
Loads check, store), version_name 11% (two linear verdef walks per symbolic lookup: requester's reference_want
and definer's def_version_name; libc has ~40 verdefs), read_phdr 8%, Object::new 6%, memmove 5%, lookup 4-6%,
target 3%, Dynamic::once 3%, ld_env 3% (environment scan), fill_sse2 3%, dlopen::check 2%.

## Faults (rel2 -> rel3)
By page (perf page-faults -d on pie0, rel2): 18 consecutive stack pages (the start's tables: fixed by D-27.64,
-10), two runs of ~10 pages in one mapping (libc.so.6's relocated data and GOT: COW writes of 1777 RELATIVE + 285
symbolic entries), the rest text/rodata of ld.so and libc.so.6. Remaining gap 65 vs 49: the relocated data pages
(layout work: fewer relocated words, or RELR-packed data grouped into fewer pages) and the stack frame still left.

## Next steps (in order)
1. Version name by index O(1): a per-object offset table outside `Object` (no room: size assert 0x128), e.g. in
   the loader's start arena, or a per-pass memo of (object, index) -> name passed to `target`.
2. pass1 RELATIVE loop: specialise for little-endian (no per-entry byte-order select), use DT_RELACOUNT for a
   prefix loop that skips classify, and check the range against one cached segment only.
3. libc.so.6 linked with packed relative relocations (RELR; pass1 already handles DT_RELR): fewer bytes, faster
   loop; a product/layout decision (RULING NEEDED in the report).
4. Fault gap 65 vs 49: relocated data pages of libc.so.6 (layout work), remaining stack frame size.
5. keke fast-profile instruction bound not measured (DYN_START_INSTR row has 0 = unchecked).

## Process notes
- Leg 1 used one heredoc (a python edit script); legs 2 and 3 each ran one empty no-op heredoc by
  mistake. All reported.
- Leg 2 left the brief's "merge next first" undone (DL-23-2 had landed); leg 3 merged. The merge commit has git's
  default message (no trailer); it was pushed before that was noticed and is not rewritten (no force-push).
- jobrun labels: a label that already ran cannot prove a new head; give every rerun a new label.
- sync_role.sh returns rc 4 (BUSY) while a plants job runs on the role: check rc before starting the next job.
