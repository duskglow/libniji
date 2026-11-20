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
  links take only the PIE anchor. Cost: one jmp per static start, beside `_start` in the same window.
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

Empty main, user instructions +4,932: new crt1 with both notes stripped 832,369; property note alone stripped
832,370; ABI tag alone stripped 837,273. So a program's PT_GNU_PROPERTY costs about 4,850 user instructions in
libniji's start (loader or libc). The cause was not found: the only runtime reader in crates/ld is
`needs_indirect_extern_access` (canonical PLT to a protected definition). The empty-main row is the one row worse;
every program with text after the old group end (the 3-library and clock_gettime rows, any real program) gains.
NOT MEASURED: the Skylake-SP class; static programs' start-up (their layout is unchanged apart from the jmp).
Static programs still have two `R E` LOADs, before and after: `.init` (crti) and then libc.a's hot text group at a
64 KiB boundary, more than a page away. That is libc.a's layout, which this story leaves as it is.

## Jobs (keke)
crt-start-files-crt (crt_products, 7/7 ok), crt-start-files-static (static clean; affected list),
crt-start-files-lint (clippy clean; its static failed on the trace marker, fixed in e292a64b6),
crt-start-files-gate, crt-start-files-plants2, crt-start-files-xtest, crt-start-files-fullprof (see the hand-back).
Superseded: crt-start-files-so1 (layout contract red, fixed by the anchor exemption), crt-start-files-plants (the tool
refused the .S plants: `_start`, the anchor stub and crti.S have no `niji_machine_<fn>:` label).

## STATE AT REFRESH (leg 1 ended 20:03Z; HEAD = the commit adding this section)
Gate crt-start-files-gate (head e292a64b6) RED in `niji::hot_text_products`, two tests:
1. `init_no_library_text_precedes_the_hot_group` (TEXT-PREFIX, hot_text_products.rs:1311): the anchor's stub section
   `.text.startup.niji_static_anchor` is a libc.a input ahead of the hot group (the group start did not move:
   _start 0x26 bytes + the 16-byte stub still end below the 64-byte-aligned group start 0x40, but the rule is
   "no libc.a input ahead"). Fix candidates: (a) put the stub after `.balign 65536` INSIDE the group-end section
   `.text.sorted.h9z.niji_group_end` (behind the group, in the window of the program's own text, which every
   program runs), then exempt the anchor members from STARTUP-TEXT-WINDOWS' window count as well as from its
   ungrouped list (layout.rs `startup_text`); (b) a `.text.sorted.h0...` section at the group head, but that shifts
   the bins by 16 bytes (bins.rs HEAD_ROOM / window-bin-bytes contract). Prefer (a).
2. `init_startup_data_pages_hold_at_every_data_phase` (hot_text_products.rs:1666, "startup data pages: ..."): message
   not read yet: `grep -n "startup data pages" -A14 ~/logs/crt-start-files-gate.log` on keke. Suspect: the probe's
   data-phase pad object puts its own `.data.rel.ro.local`/`.data1` ahead of the libc.a anchor (before, crt1's
   anchor came before the pad), so a pad shifts where the anchor's alignment applies. Read the message first.
Then: rebuild (`cargo xtask so`, ~20 min), rerun `cargo test -p niji --test hot_text_products` and crt_products, the
gate, then `cargo xtask test` and the NIJI_FULL_PROFILE pair (§9), precheck, handback-check. Plants:
crt-start-files-plants2 (started on e292a64b6) may still be running; rerun plants if so.rs/entry.S change.
Stopped at refresh: crt-start-files-xtest and crt-start-files-fullprof (stale head).

## Open
- RULING NEEDED: accept +4,932 user instructions (+0.6%) for empty main (kernel -3.5k, faults -1) as the cost of the
  property note the brief requires, with a follow-up to find what libniji's start does with PT_GNU_PROPERTY.
- Next step: that follow-up (trace ld.so on the empty-main pair `new-m0`/`nonote-m0`, scripts in
  ~/work/crt-start-files/ on keke: startcount.sh, m0probe.sh, m0probe2.sh).
