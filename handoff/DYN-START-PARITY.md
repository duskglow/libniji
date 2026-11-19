# DYN-START-PARITY (handoff after leg 2)

## LEG 3: do exactly this (code is done; only the final checks remain)
Head 8a2301180 synced to mei `~/w/s1` (tree 935eaef5...). At 18:10Z leg 2 started `~/work/DYN-START-PARITY/final.sh`
(detached), which runs, in order, the jobs `dyn-start-parity-lint3` (cargo xtask clippy + static), then
`dyn-start-parity-gate` (direct tier, `affected --base origin/next --run`, fast products), then
`dyn-start-parity-startup` (cargo xtask test; NIJI_FULL_PROFILE=1 so + hot_text_products, release).
Plants are done and green: `dyn-start-parity-plants` 9 CAUGHT, 0 SURVIVED, 0 WARN (run on 77fb93789; the ld code
is identical at the final head, later commits only touch tests' doc lines, records, handoff).
1. `~/bin/jobwait dyn-start-parity-lint3 dyn-start-parity-gate dyn-start-parity-startup` (10-min calls).
   lint3 red: fix, commit, push, sync (check rc; 4 = BUSY), rerun final.sh. Gate red: only failures in the direct
   tests of this change are ours (`reloc_ldso_kernel_started_without_proc` ETXTBSY is a known pre-existing race:
   INFORMATIONAL). hot_text_products bounds: DYN_TRUE_BOUND [77, 73], DYN_START_INSTR (Skylake-SP fast 1,752,700 /
   release 1,630,700; Zen 1 release 1,630,600): a Zen 1 gate can read the fault bound differently; report it.
2. `cargo xtask precheck --jobs dyn-start-parity-plants,dyn-start-parity-lint3,dyn-start-parity-gate,dyn-start-parity-startup --head <sha>` on the role.
3. Fill `gate.affected` in handoff/reports/DYN-START-PARITY.json, push, `ops handback-check`, hand back with
   JOBS: mei:dyn-start-parity-plants, mei:dyn-start-parity-lint3, mei:dyn-start-parity-gate, mei:dyn-start-parity-startup
   SUPERSEDED: ldtest1 (test bug, fixed), ldtest2 (ETXTBSY race), lint/lint2 (clippy casts; lint2 ran on an unsynced
   tree: sync BUSY), lint3's first run at 18:10 (trace markers), dyntests/dyntests2 (measurement runs), so2/so3 (builds).
   RULING NEEDED: RELR for libc.so.6 (see report). Verdict: COMPLETE with "target (parity) not reached".

State: branch `task/dyn-start-parity` on origin/next 255a6901d. Role: mei `~/w/s1` (Skylake-SP). Host scratch
`/mnt/zram/claude/DYN-START-PARITY/` (rel2, rel3 = release products of this branch; prog-* = test programs),
scripts `~/work/DYN-START-PARITY/parity.sh` (execcount, `GLIBC=1` adds the reference) and `prof.sh` (perf on the
exec'd program only). Decisions: D-27.63 (lookup view / relocation pass read each table once), D-27.64 (16-entry
inline start tables). Plants: `handoff/DYN-START-PARITY-plants.txt`. Report: `handoff/reports/DYN-START-PARITY.json`.

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
- Leg 1 used one heredoc (a python edit script); leg 2 ran one empty no-op heredoc by mistake. Both reported.
- sync_role.sh returns rc 4 (BUSY) while a plants job runs on the role: check rc before starting the next job.
