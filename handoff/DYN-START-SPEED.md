# DYN-START-SPEED (handoff after DYN-START-SPEED-2)

## State
Branch `task/dyn-start-speed-2` (= `task/dyn-start-speed` ef1efc732 + merge of origin/next 90f71362a + this run's
commits); final head = the commit carrying this note. DYN-START-SPEED's code commits (8ab452436 RELACOUNT prefix loop,
4cb8daf5b verdef record scan, 348a4f666 version-name memo, fa6338555/3be912577 bounds, tests, plants) are unchanged.
This run: merge (clean), b849c7ce2 + 1b976ec1e (whole-file views placed 64 KiB-aligned by the loader, `map_view` in
crates/boundary/src/ld.rs; new test `init_dynamic_startup_fault_count_does_not_depend_on_the_file_system`), then the
Zen 1 fast instruction bound and this note. Full report: handoff/DYN-START-SPEED-2-report.md.

## The CI red (68 > 67 release faults under the gate) - cause and fix
Probe `~/work/DYN-START-SPEED-2/dynprobe.sh` on mei (the test's staging, argv, environment; perf fault addresses):

| release products | /tmp ext4 | /tmp tmpfs (gate) |
|---|---|---|
| origin/next 90f71362a | 67 | 67 |
| merged head before the fix | 67 | 68 |
| merged head with `map_view` | 67 | 67 |

Only libc.so.6's whole-file read-only view differed (4 faults per process on ext4, 5 on tmpfs; pages touched by
reloc::pass1 / reloc::target). The kernel places that view 2 MiB-aligned on ext4 and page-aligned on tmpfs, so the
same file pages fell in different 64 KiB fault-around windows; this branch's loader changes read view pages that
straddle a window boundary at tmpfs's phase. Not ldso-text.list: that list is traced at every build (never stale).
Fix (also step 4's file-system part): every whole-file view (start-up search, open_file, `ld.so PROGRAM`, list mode,
dlopen) is mapped at a loader-chosen 64 KiB-aligned address (reserve oversized PROT_NONE, MAP_FIXED the file, trim
both ends; same `munmap(at, len)`). One bound holds on both file systems; DYN_TRUE_BOUND stays [73, 67].

## Measurements on the final code (same mei-built products on both hosts; median of 21, ASLR off)
| host | fast instr | release instr | host C library | faults fast (ext4/tmpfs) | faults release (ext4/tmpfs) |
|---|---|---|---|---|---|
| Skylake-SP (mei) | 855,582 | 803,431 | 101,863 | 73 / not probed | 67 / 67 |
| Zen 1 (keke) | 855,428 | 803,280 | 96,002 | 73 / 73 | 67 / 67 |
Zen 1 fast bound set to 872,600 (level + 2%, was 0 = unchecked); Zen 1 release 812,400 now measured (1.1% over).

## Not done (next steps, in order)
1. Step 3 RELR (measurement only, coordinator ruling #6786): libc.so.6 link line in xtask/src/so.rs (ld.so already
   uses `-z pack-relative-relocs`); check `readelf -V` for a new version need and which loaders load libc.so.6 in each
   shipped configuration (system, coexist, interposed); measure instructions, faults, file size.
2. Fault gap 67 vs glibc 51 (release): the view's own faults (4 per process: file pages 0, 0x1d, 0x20, 0x3ed) could
   go if symbol and relocation reads went through the mapped segments instead of a separate view (the first R
   segment takes no fault today); then libc.so.6's relocated data pages and the start frame.
3. Profile left (from DYN-START-SPEED, still current): pass1 general loop, Object::new symbol counting, lookup's NUL
   scan, program headers decoded repeatedly (see the report).
Scripts on mei: `~/work/DYN-START-SPEED-2/` (dynprobe.sh, zen.sh, kit under /mnt/zram/claude/DYN-START-SPEED-2/kit),
`~/work/DYN-START-SPEED/` (execcount, parity.sh, prof.sh).
