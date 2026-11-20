# CRT-START-FILES (GitHub #27): start files hold only `_start`, the notes and the usual symbols

Branch `task/crt-start-files` (from origin/next ed38a2b3d). Decision: D-X.144 (records/decisions/D-X/D-X.144.toml).
Plants: handoff/CRT-START-FILES-plants.txt. Role keke:~/w/n2 (Zen 1 host).

## What changed
- `csys/src/machine/entry.S`: the start object (crt1/Scrt1/rcrt1, and gcrt1/grcrt1 through gcrt1.S) no longer carries
  the empty 16-64 KiB-aligned anchors (`.rodata1`, `.data1`, `.data.rel.ro.local`, `.text.sorted.h9z.niji_group_end`).
  It now has `_start` starting with endbr64, `.note.gnu.property` (IBT, SHSTK, x86-64 baseline) and `.note.ABI-tag`
  (Linux 3.2.0) (macros in `csys/start/notes.inc`), `__data_start` (global, `.data`), weak `data_start`, and
  `_IO_stdin_used` (global 4-byte object, 0x20001, value/size/binding from a readelf/nm probe of the installed
  Scrt1.o). crti.o/crtn.o carry the property note; `__gmon_start__` starts with endbr64.
- The anchors are now libc.a's first two members, `niji_static_anchor.o` and `niji_static_pie_anchor.o` (entry.S with
  `-DNIJI_STATIC_ANCHOR`, inserted by `insert_static_anchors` in `finish_archive`, so the traced and final archives
  both get them). A static link takes the first index entry defining a still-undefined symbol; the start object
  leaves only `__libc_start_main` (crt1/gcrt1) or `niji_start_main_static_pie` (rcrt1/grcrt1), which each anchor
  defines weakly as `jmp __niji_libc_<name>`. Map of a static link: the anchor is libc.a's first input; static-PIE
  links take only the PIE anchor. Cost: one jmp per static start, at the group end (the program-text window).
- `cargo xtask so` checks each start object (`check_start_file`: no section aligned above 64, both notes, endbr64 at
  `_start`, the three symbols, no other exported definition) and the anchors' place. STARTUP-TEXT-WINDOWS exempts the
  anchor members like crt1's `_start` (`so::static_anchor_member`).
- Tests: xtask `start_file_check_names_each_missing_or_extra_piece`, `start_files_and_static_anchors_assemble_as_checked`,
  layout fixture case; `crt_products.rs` `programs_get_one_executable_mapping_and_keep_their_properties` (oracle
  `crt_maps.c`): no start file reported by `-z cet-report` in any link; dynamic PIE and -no-pie: one executable LOAD,
  no LOAD aligned above a page, IBT/SHSTK kept, one r-xp mapping of the program at run time (the lsfd
  `column-xmode` property; the trixie root was not used).

## Measurements (Zen 1 host, keke; same libc.so.6/ld.so, only crt1 differs; medians of 101 runs)
Start files: crt1.o 67,136 -> 1,816 bytes (rcrt1 1,784, gcrt1 2,744, grcrt1 2,712; reference Scrt1.o 1,656); largest
section alignment 65536 -> 64. Empty main through the driver: PIE 142,680 -> 15,968 bytes (reference 15,776), 2 -> 1
`R E` LOAD, LOAD aligns 0x10000 -> 0x1000, IBT/SHSTK kept (before: none); same for -no-pie.

| program | user instr old -> new (glibc) | kernel instr old -> new (glibc) | faults old -> new (glibc) | syscalls |
|---|---|---|---|---|
| empty main | 832,290 -> **837,222** (100,881) | 895,669 -> 892,187 (701,365) | 62 -> 61 (48) | 43 -> 43 |
| 3 libraries | 1,095,776 -> 1,087,780 (136,032) | 1,343,422 -> 1,323,381 (997,048) | 78 -> 76 (64) | 88 -> 88 |
| clock_gettime | 867,398 -> 859,779 (103,018) | 935,379 -> 926,115 (708,173) | 67 -> 65 (50) | 43 -> 43 |


Empty main, user instructions +4,932: the cost is per program header, not the property. Probe (keke,
~/work/crt-start-files/m0probe3.sh, medians of 101 runs, same libc.so.6/ld.so): crt1 without the property note
832,383 (13 program headers); the same plus a dummy 48-byte note of another type (one more PT_NOTE) 834,805 (14);
the new crt1 (PT_NOTE + PT_GNU_PROPERTY) 837,253 (15). So libniji's start (loader or libc) spends about 2.4k user
instructions per program header of the program; the property note adds two. The only runtime property reader in
crates/ld (`needs_indirect_extern_access`) reads the defining library's notes, not the program's.
NOT MEASURED: the Skylake-SP class; static programs' start-up.
Static programs still have two `R E` LOADs, before and after: `.init` (crti) and then libc.a's hot text group at a
64 KiB boundary, more than a page away. That is libc.a's layout, which this story leaves as it is.

## Leg 2 (fixes of the leg-1 gate's two hot_text_products reds; commit c9d0df2f5)
1. TEXT-PREFIX (`init_no_library_text_precedes_the_hot_group`): the anchor's `jmp` stub was the libc.a input
   `.text.startup.niji_static_anchor`, ahead of the hot group. It now sits inside `.text.sorted.h9z.niji_group_end`
   after its `.balign 65536`, at the start of the program-text window; `layout.rs startup_text` skips the anchor
   members' functions (no window, not ungrouped); fixture: the jump in a window of its own adds none.
2. `init_startup_data_pages_hold_at_every_data_phase`: with the program's relro padded 0x4d0/0x800/0xc18 bytes the
   libc.a relro anchor (`.data.rel.ro.local`, 64 KiB) came after the pad: 0xeb38-0xfb38 bytes of fill in relro.
   A real regression for any static program with its own relro (a -fPIE object's pointer tables). The relro
   anchor is now an empty `.preinit_array` (the default script puts it ahead of `.init_array` and `.data.rel.ro`,
   in relro). Probe (keke, ~/work/crt-start-files/relro_exp.sh, the page_faults probe padded as the test pads it,
   static and static-PIE): fill 0x3f0-0xb38 (< a page), cluster page-aligned, static PIE one RW LOAD, runs (rc 0).
   Padded non-PIE static had 2 RW LOADs with the experiment's anchor (the `.data1` 16 KiB alignment after a
   shifted `.data`); not compared with the pre-story crt1 (the same section order); no test bounds it.
D-X.144 records both.

## Jobs (keke)
Final head: see the hand-back. Leg 2: crt-start-files-lint3 (clippy + static, green), crt-start-files-plants3,
crt-start-files-gate3, then crt-start-files-xtest3 and crt-start-files-fullprof3.
Superseded: leg 1's crt-start-files-gate (red: the two tests above), crt-start-files-plants2 (6/6 caught, before the
layout.rs and entry.S change), crt-start-files-so1, crt-start-files-plants, crt-start-files-xtest,
crt-start-files-fullprof (stale head).

## Open
- RULING NEEDED: accept +4,932 user instructions (+0.6%) for empty main (kernel -3.5k, faults -1) as the cost of the
  property note the brief requires (two program headers at about 2.4k each in libniji's start), with a follow-up
  story to make libniji's start cost independent of the program's header count.
- Next step: that follow-up (trace which start-up code walks the program headers per header: dummy-note pair
  `noprop-m0x`/`dummy-m0x` in /dev/shm/claude/crt-start-files/sc on keke).
