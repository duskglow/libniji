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

## Pending at hand-back (final head)

Jobs on mei: `sys-dbg-package-2-gate2` (direct tier), `sys-dbg-package-2-final2` (package from the final
products, check-system, root-system, dbg-system in trixie with KEYRING=~/keyrings/debian-archive-keyring.gpg),
`sys-dbg-package-2-plants` (handoff/sys-dbg-package-2.plants; results into handoff/SYS-DBG-PACKAGE-2-plants.txt
by the tool if it writes there).

## Next step

If final2 or plants are red: read `~/logs/sys-dbg-package-2-final2.log` (dbg outputs under
`/mnt/zram/claude/SYS-DBG-PACKAGE-2/dbglog2/dbg-trixie/`). Possible follow-up: line tables for the C layer
(csys, e.g. printf in variadic.c) need `-g` on its compiles, checked the same way (cmpalloc).
