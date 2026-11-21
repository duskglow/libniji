# VALGRIND-LDSO-SYMBOLS: valgrind starts programs under libniji (GitHub #13)

Branch `task/valgrind-ldso-symbols-2` (VALGRIND-LDSO-SYMBOLS-2: `task/valgrind-ldso-symbols` d6f56c344 with origin/next
71597d2a4 merged, 8188445e6). Briefs `~/state/briefs/VALGRIND-LDSO-SYMBOLS.md` (the change) and `-2.md` (the merge).
Build role keke:~/w/n2 (leg 1-2: keke:~/w/n3); job scripts and probe outputs `~/work/VALGRIND-LDSO-SYMBOLS-2/` on keke
(vg-probe.sh, vg-probe2.sh, vg-cmds.sh, bad.c, thr.c, probe4.sh as in legs 1-2; final.sh, chain1.sh this run).

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

## The change (commits c74eb73fb, 68b92caed, ba8bbb4ec, 8e3640133, 1eae04250)

* `xtask/src/so.rs`: the interpreter link adds `--defsym=strcmp=niji_machine_ld_strcmp` and
  `--defsym=strlen=niji_machine_strlen` (LD_TOOL_ALIASES; the version script keeps them local) and `csys/src/machine/ld_names.S` after the archive; after the link
  `size_tool_aliases` copies the target's st_size to the alias in .symtab (not loaded: no loaded byte
  changes); `check_interp_tool_names` refuses an interpreter whose `strcmp`/`strlen` are not LOCAL FUNC,
  sized, at the alias target, or are in .dynsym. Unit test `ld_tool_names`.
* `csys/src/machine/ld_names.S`: `niji_machine_ld_strcmp`, a 23-byte byte-wise strcmp (C17 7.24.4.2),
  hidden, section retained (`R`),
  linked last so its text is the end of .text (it landed at .text's end: no loader function moved). Nothing
  calls it. `crates/boundary/build.rs` keeps the file out of the C layer archive (libc.so.6/libc.a unchanged).
* `tools/deb/check-system.sh`: libc6-dbg's debug file of the loader must have LOCAL FUNC strcmp and strlen.
  Since the merge of next (8188445e6) that check is a `case` on the loader inside SYS-DBG-DWARF's per-debug-file
  loop (every debug file: .symtab, .debug_info, .debug_line); the branch's old libc.so.6 line-table case is gone
  (next's loop covers it).
* No exported ABI change: .dynsym of the stripped loader has no strlen/strcmp; the stripped loader is unchanged
  in kind (no .symtab).

## Proof

* Symbol listing, head's libc6-dbg (job valgrind-ldso-symbols-final5, rc 0; check-system PASS, 7 packages):
  `393: 0000000000052340 966 FUNC LOCAL DEFAULT 1 niji_machine_strlen`, `417: ...53910 23 FUNC LOCAL ... strcmp`,
  `436: ...52340 966 FUNC LOCAL ... strlen`; stripped loader: 0 .symtab, 0 dynsym strlen/strcmp rows.
* Before (20.4 debs, job valgrind-ldso-symbols-probe1): memcheck /bin/true and ./bad rc 1, the fatal strcmp
  message; helgrind rc 1 (`hg_main.c:5421 Assertion 'found' failed`); drd rc 0 without libc6-dbg.
* Stock (same job): memcheck /bin/true rc 0, 0 errors; ./bad rc 0, "Invalid read of size 4 at main (bad.c:11)",
  1 error from 1 context; helgrind and drd on ./thr rc 0, 0 errors.
* After, head debs with only the loader's debug file installed (job valgrind-ldso-symbols-probe4; repeated on
  the final debs as valgrind-ldso-symbols-probe6, same result): memcheck
  starts and runs: /bin/true rc 0, ./bad rc 0 with the same Invalid read at bad.c:11. The error summaries differ
  from stock by one start-up report inside the loader (1 vs 0, 2 vs 1): "Conditional jump or move depends on uninitialised value(s)" at
  `niji_boundary::ld::link::link (link.rs:356)` (the `cache.get()...map_or` of the cache unmap), not yet diagnosed.
  drd rc 0, 0 errors; helgrind still `Assertion 'found' failed`.
* Without libc6-dbg (final5): the stripped loader has no names; memcheck stops with the strcmp message, as
  before. With the whole libc6-dbg (final3, final5): every tool aborts in valgrind's debug reader:
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

## State at hand-back (leg 3, VALGRIND-LDSO-SYMBOLS-2)

The last leg's CI landing conflicted in `tools/deb/check-system.sh` with SYS-DBG-DWARF (75af35f6). This run merged
origin/next 71597d2a4 into the branch (8188445e6, the conflict resolved as described under "The change") and pointed
the packaging plant at the merged head's products (d3fffeda1). No other code changes. Results, all on keke:~/w/n2 at
d3fffeda1 (code identical to 8188445e6):

* lint0 (inherited head d6f56c344: clippy + static) rc 0; lint1 (merged head: `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` + `cargo xtask static`) rc 0.
* final1 rc 0: system debs from the merged head; `check-system.sh` PASS (7 packages, 2.39+niji20261121.1-1);
  libc6-dbg 19 debug files; loader debug file has .debug_info/.debug_line and `419: ...53b10 23 FUNC LOCAL ...
  strcmp`, `438: ...52500 966 FUNC LOCAL ... strlen` (at niji_machine_ld_strcmp / niji_machine_strlen); stripped
  loader: 0 .symtab, 0 dynsym strlen/strcmp; unit `ld_tool_names` ok. Valgrind in the swapped trixie root, same as
  legs 1-2: stock all rc 0; libniji without libc6-dbg: memcheck stops on the strcmp redirect (expected: the
  stripped loader has no names); libniji with only the loader's debug file: memcheck /bin/true rc 0, ./bad rc 0 with
  its Invalid read reported, drd rc 0, helgrind `Assertion 'found'`; with the whole libc6-dbg every tool aborts in
  `find_rodata` (Not fixed, item 1).
* plants1 rc 0: 6 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY / WARN-NO-MSG (one WARN-DEV-SHM: the RAM dir is under
  /dev/shm, as the brief prescribes for this Zen 1 host); results in the plants file.
* gate1 (direct tier, `affected --run --base origin/next`): rc 0, 14 suites ok, 8363 passed, 0 failed, 1230 ignored, peak 12G.
* startup1 (`cargo xtask test`, then NIJI_FULL_PROFILE so + hot_text_products, section 9): still running at hand-back (about 700 of 6973 tests
  at 08:30 UTC, all PASS so far); named in PENDING JOBS for the dispatcher. If it is red: fix that suite, rerun
  it alone under a new label, precheck, hand back.

Superseded: legs 1-2's jobs (pre-merge heads). Exact next step: none for this story if the CI train lands it; then
the next story is Not fixed item 1 (libc6-dbg readable by valgrind: libc.so.6's .rodata1/.rodata pair, layout
measured per section 9), then items 2 and 3.

## Plants and gates

Plants file `handoff/VALGRIND-LDSO-SYMBOLS-plants.txt` (six, results appended: three on the check's unit test,
build-no-alias and size-alias through `cargo xtask so`, pkg-debug-drops-strcmp through build.sh --products +
check-system.sh, now on the merged head's products in /dev/shm/claude/VALGRIND-LDSO-SYMBOLS-2/products-final);
job valgrind-ldso-symbols-2-plants1, 6 CAUGHT.
