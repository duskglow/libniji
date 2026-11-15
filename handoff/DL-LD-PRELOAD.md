# DL-LD-PRELOAD: LD_PRELOAD and /etc/ld.so.preload in Liella's ld.so (handoff, run 2: DL-LD-PRELOAD-2)

State: branch `task/dl-ld-preload-2` (run 1's two WIP commits + run 2), role keke:~/w/n2. Code, tests,
docs and the fakeroot case are committed and pushed. The run stopped at the context checkpoint with the
proof jobs running detached on keke (below); their verdicts were not seen by the agent.

## Decisions in force
- plan/ph14/threat-model-draft.md rule 1 (LD_PRELOAD secure exception), rule 5 (trusted directories);
  spec/behaviour/dynlink-secure.md (secure-mode facts, line 39: a bare trusted-dir name without the bit
  gets the "cannot open shared object file" line); spec/behaviour/dynlink-preload.md (new: run 1's probe
  record; the probe scripts moved to tools/probe/dl/preload/). No ruling was needed.

## Done (run 2)
- Inherited head: clippy (all targets, all features) and `cargo xtask static` clean (jobs
  dl-ld-preload-2-clippy1, -static1): the WIP code compiled as written.
- crates/ld/tests/preload.rs (4 tests, all pass, no SKIP on keke; job dl-ld-preload-2-t2): interposition
  (`f` over libdep's, `strlen` over libc's via `dlsym(RTLD_NEXT)`), constructor order, path / bare
  (RPATH) / `$ORIGIN` / repeated / dependency / separators-only names, `ld.so PROGRAM`; the four
  "cannot be preloaded" reasons; secure mode on a set-group-ID program (slash skipped silently, bare
  only from trusted dirs, a trusted-dir library without set-user-ID refused: libz.so.1 etc.); the
  preload file through `unshare -r -m` binding a private /etc (syntax, order after LD_PRELOAD, the
  message naming /etc/ld.so.preload).
- ldso_run.rs: the two LD_PRELOAD tests expect the real reasons; unit test covers the file syntax,
  `preload_rule`, `preload_mode_admits`, `preload_ignored`.
- fakeroot: swap-system.sh installs Debian's fakeroot in both roots; swap-cmds.sh fakeroot-1 (uid 65534:
  `inside 0 0 0:0`, `outside 65534 65534 65534:65534` expected) and fakeroot-2 (tar under fakeroot 0/0,
  outside 65534/65534), compared with the stock root; Liella's outputs printed as INFO lines.
- docs/install-packages.md: preloading paragraph.

## Things not to redo
- Do not probe the reference again. A segfault seen while testing was the HOST's glibc programs
  (`unshare`, `timeout`, `gdb`) inheriting LD_PRELOAD of a Liella-linked test library, not Liella's loader;
  the file test now sets LD_PRELOAD only for the final `exec`.
- Names from the preload file point into its mapping, which `link` unmaps with the objects' own file
  mappings; nothing keeps a name past that point (START_OBJS keeps numbers only).

## Jobs running at hand-back (keke)
- dl-ld-preload-2-fakeroot1: `tools/deb/build.sh --flavour system` (LIELLA_DEB_COMMIT/SOURCE_DATE_EPOCH:
  the role is not a git tree) then swap-system.sh trixie (MIRROR miko, KEYRING ~/keyrings). Script
  ~/work/DL-LD-PRELOAD-2/fakeroot.sh. Read `grep '^swap:' ~/logs/dl-ld-preload-2-fakeroot1.log`.
- dl-ld-preload-2-plants: handoff/DL-LD-PRELOAD-2-plants.txt (10 plants: secure arms, mode check, secure
  capture, order before DT_NEEDED, file after env, file comment, missing silent, directory text).
- dl-ld-preload-2-gate: affected --run, then `cargo xtask test`, then the LIELLA_FULL_PROFILE
  hot_text_products check (loader start-up code changed).

## Next step (exact)
1. Read the three jobs' verdicts. Any red: fix the cause, rerun only that job on the new head.
2. If fakeroot-1/-2 differ in the Liella root: the faked daemon uses SysV IPC; read the diff under
   target/deb-system/swap-trixie/ on the role.
3. Start-up cost: one `openat` of the preload file per dynamic start (the reference's one `access`);
   a start-up page-fault/window bound red here touches start-up code (ALWAYS FORWARD applies).

## Open / for the coordinator
- Remove `LD_PRELOAD` from the "Not available yet" list in liella-ops release notes when this lands.
- INFORMATIONAL: `ld.so --list` / LD_TRACE_LOADED_OBJECTS does not show preloads; `ld.so --preload`
  is not done.
- Plant not run: an edge from the program to a preload is equivalent for the order (the root is placed
  last by init_order), so no plant covers "no edge".
