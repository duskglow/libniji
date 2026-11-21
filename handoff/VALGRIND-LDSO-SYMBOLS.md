# VALGRIND-LDSO-SYMBOLS: valgrind starts programs under libniji (GitHub #13)

Branch `task/valgrind-ldso-symbols` (from origin/next 305065b07). Brief `~/state/briefs/VALGRIND-LDSO-SYMBOLS.md`.
Build role keke:~/w/n3; job scripts and probe outputs `~/work/VALGRIND-LDSO-SYMBOLS/` on keke (vg-probe.sh,
vg-probe2.sh, vg-cmds.sh, bad.c, thr.c, final.sh, probe4.sh). Code head ba8bbb4ec.

## What valgrind needs (black-box probes, trixie root, valgrind 3.24.0, Debian package)

* Under libniji (snapshot 20.4 swapped in) memcheck aborts at start-up: "Fatal error at startup: a function
  redirection which is mandatory for this platform-tool combination cannot be set up ... A must-be-redirected
  function whose name matches the pattern: strcmp in an object with soname matching: ld-linux-x86-64.so.2 was
  not found"; it suggests installing libc6-dbg.
* Trixie's valgrind package pulls in libc6-dbg: the stock root had libc6-dbg 2.41-12+deb13u4 installed. Stock
  trixie ld.so is fully stripped (no .symtab, no strlen/strcmp in .dynsym); its libc6-dbg debug file has LOCAL
  FUNC `strlen`, `strcmp` (and index, strchr, memcpy, memmove, mempcpy, memset, memcmp, stpcpy, strncmp,
  strnlen). So names in libniji's libc6-dbg debug file are what stock relies on; the stripped loader needs none.
* Valgrind passes over a size-0 symbol: with `strlen` as a plain `--defsym` alias (size 0) the message moved
  from strcmp to strlen. With the alias sized (966, niji_machine_strlen's size) both are found.

## The change (commits c74eb73fb, 68b92caed, ba8bbb4ec)

* `xtask/src/so.rs`: the interpreter link adds `--defsym=strlen=niji_machine_strlen` (LD_TOOL_ALIASES; the
  version script keeps it local) and `csys/src/ld/ld_names.S` after the archive; after the link
  `size_tool_aliases` copies the target's st_size to the alias in .symtab (not loaded: no loaded byte
  changes); `check_interp_tool_names` refuses an interpreter whose `strcmp`/`strlen` are not LOCAL FUNC,
  sized, at the alias target, or are in .dynsym. Unit test `ld_tool_names`.
* `csys/src/ld/ld_names.S`: a 23-byte byte-wise strcmp (C17 7.24.4.2), local, section retained (`R`),
  linked last so its text is the end of .text (it landed at .text's end: no loader function moved). Nothing
  calls it. `crates/boundary/build.rs` keeps the file out of the C layer archive (libc.so.6/libc.a unchanged).
* `tools/deb/check-system.sh`: libc6-dbg's debug file of the loader must have LOCAL FUNC strcmp and strlen.
* No exported ABI change: .dynsym of the stripped loader has no strlen/strcmp; the stripped loader is unchanged
  in kind (no .symtab).

## Proof

* Symbol listing, head's libc6-dbg (job valgrind-ldso-symbols-final3, rc 0; check-system PASS, 7 packages):
  `347: 0000000000053910 23 FUNC LOCAL DEFAULT 1 strcmp`, `395: ...52340 966 FUNC LOCAL ... niji_machine_strlen`,
  `436: ...52340 966 FUNC LOCAL ... strlen`; stripped loader: 0 .symtab, 0 dynsym strlen/strcmp rows.
* Before (20.4 debs, job valgrind-ldso-symbols-probe1): memcheck /bin/true and ./bad rc 1, the fatal strcmp
  message; helgrind rc 1 (`hg_main.c:5421 Assertion 'found' failed`); drd rc 0 without libc6-dbg.
* Stock (same job): memcheck /bin/true rc 0, 0 errors; ./bad rc 0, "Invalid read of size 4 at main (bad.c:11)",
  1 error from 1 context; helgrind and drd on ./thr rc 0, 0 errors.
* After, head debs with only the loader's debug file installed (job valgrind-ldso-symbols-probe4): memcheck
  starts and runs: /bin/true rc 0, ./bad rc 0 with the same Invalid read at bad.c:11. The error summaries differ
  from stock by one start-up report inside the loader (1 vs 0, 2 vs 1): see the hand-back, not yet diagnosed.
  drd rc 0, 0 errors; helgrind still `Assertion 'found' failed`.
* After, head debs with the whole libc6-dbg (final3): every tool aborts in valgrind's debug reader:
  `m_debuginfo/readelf.c:3013 (find_rodata): Assertion 'di->rodata_size == sh_size' failed`. libc.so.6 has two
  read-only data sections, `.rodata1` (the start-up anchor, D-X.133, 64 KiB aligned) and `.rodata` after a gap;
  the loader and stock libc have one. With the loader's debug file alone (probe4) it does not happen, so
  libc.so.6's debug file is the trigger (not yet confirmed by a libc-only run).

## Not fixed (other mechanisms; next stories)

1. libc6-dbg + valgrind: the find_rodata assertion on libc.so.6's `.rodata1`/`.rodata` pair. Options to weigh:
   no `.rodata1` in libc.so.6's layout (start-up faults to measure), a different section name for the anchor,
   or merging them in the link. Layout change: needs §9 measurements.
2. helgrind: `hg_handle_client_request: Assertion 'found' failed` (with or without debug files): helgrind's
   pthread wrappers and libniji's threads; probe which client request (`--trace-redir=yes`, `-v`).
3. memcheck: one start-up report in the loader (hand-back has the detail).

## Plants and gates

Plants file `handoff/VALGRIND-LDSO-SYMBOLS-plants.txt` (six: three on the check's unit test, build-no-alias and
size-alias through `cargo xtask so`, pkg-debug-drops-strcmp through build.sh --products + check-system.sh).
Jobs on keke: valgrind-ldso-symbols-final3, -lint3, -gate3, -plants3 (results in the hand-back).
Superseded: final/final2/lint/lint2/gate/gate2/plants/plants2 (older heads; final2 did not compile).
