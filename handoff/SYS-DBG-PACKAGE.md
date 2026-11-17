# SYS-DBG-PACKAGE(-2): libc6-dbg from the system build

Branch `task/sys-dbg-package-2` (from origin/next 24dc5587a; carries SYS-DBG-PACKAGE's commits). Briefs:
`~/state/briefs/SYS-DBG-PACKAGE.md` (the change), `~/state/briefs/SYS-DBG-PACKAGE-2.md`. Build role mei:~/w/s4.
Job scripts and scratch on mei: `~/work/SYS-DBG-PACKAGE-2/`, `/mnt/zram/claude/SYS-DBG-PACKAGE-2/`.

## State

* libc6-dbg (SYS-DBG-PACKAGE): `tools/deb/system.sh` splits every libc6 shared object and libc-bin's four
  programs with `objcopy --only-keep-debug` into `/usr/lib/debug/.build-id/xx/yyyy.debug`; fields, checks
  (`check-system.sh`), install/purge (`root-system.sh`), swapped-root gdb proof (`dbg-system.sh`, `dbg-cmds.sh`).
* **The open item is fixed (SYS-DBG-PACKAGE-2).** Cause, found by listing the DWARF compile units of each
  step of `cargo xtask so`: libniji's crates emitted no DWARF at compile time. The products build in their
  own workspace (`crates/Cargo.toml`), whose `[profile.release]` has no `debug` key; the root Cargo.toml's
  `debug = "line-tables-only"` never applied. Nothing in the post-link pipeline drops DWARF.
* Fix: `xtask/src/so.rs` `RELEASE_DEBUG_RUSTFLAGS` = `-Cdebuginfo=line-tables-only`, added to every
  release-profile product build (`ProductBuild::command`). The `fast` tool profile is unchanged (no DWARF).
  NOT the profile key: tried first; cargo hashes profile settings into `-C metadata`, every Rust symbol
  hash changed and libc.so.6's .text/.rodata/.data/.dynsym bytes differed. Do not redo.
* Loaded image unchanged (job sys-dbg-package-2-final, `cmpalloc.py`, every SHF_ALLOC section's
  type/flags/address/size/SHA-256): 37 ELF products and every libc.a/libm.a member, 0 differences against
  the build without line tables; only the `.note.gnu.build-id` of 5 files differs (ld's build-id hashes
  the debug sections too). Layout contract lines identical (STARTUP-TEXT-WINDOWS 1 window, 100/100
  functions; STARTUP-DATA-WINDOWS 1 ro window, 8 rw pages). libc.so.6: 613 CUs, 10 of libniji's crates
  (0 before).
* gdb checks: `info line *FUNC` (entry address; the linespec form found no function entries) and
  `-iex "set debug-file-directory ..."` (with `-ex` gdb had loaded the file first and never read the
  debug file: that, not only the missing DWARF, made the earlier check fail). dbg-system.sh now stops at
  `exit` in /usr/bin/true and checks frame, `list` and `bt`.
* libc6 now also ships `libresolv.so.2` and `libBrokenLocale.so.1` (the products build them; without them
  the swap removed the distribution's copies and gdb could not start in the swapped root). This is a libc6
  content change (two files added); the existing objects are unchanged.
* Measured on c9378a18c products: reproducible (7 identical .debs twice), root-system PASS, coexist
  flavour builds and `check.sh` PASS, parity.sh (sample 300): the `d libc6-dbg` (package missing) gap row
  is gone; new rows are the Depends version and the content-addressed `.build-id/xx` directory names
  (cannot match by construction). libc6-dbg .deb 8.6 MB (was 0.27 MB without libniji DWARF).
* docs/install-packages.md: libc6-dbg row. docs/RELEASE-NOTES.md not touched (only released entries exist).

## Final head aac54df06 (job sys-dbg-package-2-final3, mei)

repro PASS (7 .debs twice), check-system PASS (7 packages), allocated sections of the packaged objects
identical to the build without line tables, root-system PASS. dbg-system in trixie: 6 PASS, 1 FAIL:
without libc6-dbg no line for setlocale; with it `Line 102 of "boundary/src/export.rs"` (gdb found the
debug file by build-id); `list` names the source. **FAIL: gdb in a running /usr/bin/true --version stops
only at `setlocale@plt`**, with "warning: Unable to find dynamic linker breakpoint function. GDB will be
unable to debug shared library initializers": gdb finds no rendezvous breakpoint function in libniji's
loader, so it never sees libc.so.6 load and cannot place the breakpoint inside it or unwind
through it. That is the loader's debugger interface (r_debug and its breakpoint function as gdb looks
them up), not packaging: a story of its own (loader), then rerun dbg-system.sh.
Started on aac54df06, not collected at hand-back: `sys-dbg-package-2-gate3` (direct tier),
`sys-dbg-package-2-plants2` (5 plants, --timeout 1800, dir /mnt/zram/claude/SYS-DBG-PACKAGE-2/plants-2).
Superseded: -lint0 (inherited head, green), -bisect1, -before/-after/-flag/-compare (experiments), -final
(c9378a18c, the -ex/libresolv failures), -gate/-gate2/-plants (killed: older heads), -final2 (exit-breakpoint).

## Next step

1. Loader story: gdb's shared-library tracking on libniji's ld.so ("Unable to find dynamic linker breakpoint
   function"); black-box probe what gdb needs, design it in the loader, then `KEYRING=~/keyrings/debian-archive-keyring.gpg
   sh tools/deb/dbg-system.sh <debs> trixie` must be all PASS (frame `#0 setlocale ... at file:line`).
2. Read gate3/plants2 (`~/bin/jobwait` on mei); a red there is this branch's.
3. Optional: line tables for the C layer (csys, e.g. printf in variadic.c) need `-g` on its compiles,
   checked the same way (cmpalloc.py in ~/work/SYS-DBG-PACKAGE-2 on mei).
