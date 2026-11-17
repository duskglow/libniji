# SYS-DBG-PACKAGE(-2, -3): libc6-dbg from the system build

Branch `task/sys-dbg-package-3` (SYS-DBG-PACKAGE-2's branch plus a merge of origin/next d70315d05). Briefs:
`~/state/briefs/SYS-DBG-PACKAGE.md` (the change), `-2.md`, `-3.md`. -3 build role mei:~/w/s2; job scripts
`~/work/SYS-DBG-PACKAGE-3/` (final.sh, plants file), scratch `/mnt/zram/claude/SYS-DBG-PACKAGE-3/`.

## State (code head 08be3477a)

* libc6-dbg: `tools/deb/system.sh` splits every libc6 shared object and libc-bin's four programs with
  `objcopy --only-keep-debug` into `/usr/lib/debug/.build-id/xx/yyyy.debug`; fields and paths checked by
  `check-system.sh`, install/purge by `root-system.sh`, swapped-root gdb proof `dbg-system.sh` + `dbg-cmds.sh`.
* Line tables in the release products: `xtask/src/so.rs` `RELEASE_DEBUG_RUSTFLAGS` =
  `-Cdebuginfo=line-tables-only` (the product workspace `crates/Cargo.toml` has its own release profile;
  the profile `debug` key changes cargo's `-C metadata` and so every symbol hash and loaded byte: tried
  in -2, do not redo). The `fast` profile has no DWARF.
* -3 merge of origin/next (ruling #6149): libresolv.so.2 / libBrokenLocale.so.1 now come from next
  (SYS-LIBRESOLV-3, SYS-DEV-LINKS-2); this branch keeps next's object lists (libmvec.so.1 included) and
  adds only libc6-dbg (its control/path checks, the install-packages.md row, root-system's count: seven
  packages + three stand-ins = 10).
* -3 `dbg-system.sh`: the running-program checks (stop at setlocale in `/usr/bin/true --version` with a
  `file:line` frame; `list` there) print **PENDING** exactly when gdb warns "Unable to find dynamic linker
  breakpoint function" and shows no source-level `#0`. Cause (#6151): the loader has no debugger rendezvous
  (`crates/ld/src/debug.rs` on next d70315d05 is still "Not yet implemented"); SYS-COMPAT-VERSIONS-REST
  builds it. Once it lands the two checks are judged PASS/FAIL again with no change here.

## Proofs on the merged head (job sys-dbg-package-3-final, mei, rc 0)

* Allocated sections (cmpalloc.py, every SHF_ALLOC section's type/flags/address/size/SHA-256): build of the
  head vs the same tree with the flag disabled: 39 products/archives, 0 differences (5 differ only in the
  build-id note); same for the packaged libc6/libc-bin/libc6-dev. libc.so.6 CUs 613 (603 without).
  Layout contracts ok (STARTUP-TEXT-WINDOWS 1 window, 100/100; STARTUP-DATA-WINDOWS 1 ro, 8 rw pages).
* repro PASS (7 identical .debs twice); check-system PASS (7 packages); root-system all PASS (incl. next's
  devlinks, resolv, krb5 checks); coexist flavour builds, `check.sh` PASS (2 packages).
* dbg-system trixie: 5 PASS, 0 FAIL, 2 PENDING (loader rendezvous). With libc6-dbg:
  `Line 102 of "boundary/src/export.rs"` for setlocale; without: no line information.
* libc6-dbg .deb 8.6 MB (0.27 MB without line tables), 17 .debug files.
* Gate `sys-dbg-package-3-gate` (affected --run --base origin/next; escalated to FULL because xtask changed):
  12073 passed, 0 failed, conformance 137/137, rc 0. Plants `sys-dbg-package-3-plants`
  (handoff/sys-dbg-package-3.plants: the five -2 plants on the merged head plus dbg-pending-key, which
  breaks the PENDING key and turns the run into FAIL): 6 CAUGHT, 0 SURVIVED. `sys-dbg-package-3-lint`
  (clippy --all-features -D warnings + static) rc 0. Only handoff/ changed after these jobs' tree (08be3477a).
* Superseded/inherited: -2's gate3 and plants2 (rc 0, pre-merge head), -3 lint0 (inherited head, rc 0).

## Next step

1. When SYS-COMPAT-VERSIONS-REST lands the rendezvous: rerun
   `KEYRING=~/keyrings/debian-archive-keyring.gpg sh tools/deb/dbg-system.sh <system debs> trixie` and see
   7 PASS (frame `#0 setlocale ... at file:line`). No code change expected here.
2. Optional: line tables for the C layer (csys) need `-g` on its compiles; check with cmpalloc.py
   (mei:~/work/SYS-DBG-PACKAGE-2/) that allocated sections stay identical.
