# DL-LD-PRELOAD: LD_PRELOAD and /etc/ld.so.preload in Liella's ld.so (handoff, run 1: CHECKPOINT)

State: branch `task/dl-ld-preload`, WIP commit on top of `db204dc19` (origin/next). Code written, **NOT
COMPILED, NOT TESTED** (run 1 stopped at the ~150k context checkpoint). Role keke:~/w/n3 (not synced yet).

## Decisions in force
- plan/ph14/threat-model-draft.md rule 1 (LD_PRELOAD secure exception), rule 5 (trusted directories);
  spec/behaviour/dynlink-secure.md (DL-0a probe: secure-mode facts). Nothing open needed a ruling so far.

## Probe-settled (reference glibc 2.39, Zen 1 host; jobs dl-ld-preload-probe1/-probe2; scripts in
## ~/work/DL-LD-PRELOAD/probe.sh, probe2.sh on keke, copies in the scratch dir)
- Scope order: program, LD_PRELOAD names (in order), /etc/ld.so.preload names (in order), then the
  program's DT_NEEDED breadth first. Init order = Liella's existing init_order with NO edge to a preload
  (two preloads pre2, pre + dep: ctors dep, pre, pre2, prog; file epre + env pre2: dep, epre, pre2, prog).
- Interposition + `dlsym(RTLD_NEXT, "strlen")` from the preload reaches libc's (strlen + 1000 seen).
- A preload that cannot be loaded never stops the program: `ERROR: ld.so: object '<name>' from
  <LD_PRELOAD|/etc/ld.so.preload> cannot be preloaded (<reason>): ignored.` Reasons: missing -> `cannot
  open shared object file`; non-ELF short file -> `file too short`; directory -> `cannot read file data`;
  truncated ELF (100 bytes) -> `cannot read file data`.
- LD_PRELOAD separators: space and colon only (tab and newline are part of the name). `$ORIGIN` in an
  LD_PRELOAD path expands (program's directory). Same lib twice, or the program's own dependency: loaded once.
- Bare names (env and file) use the program's search (its RPATH found libpre.so).
- /etc/ld.so.preload: separators space, tab, newline, colon (comma and CR are not); `#` starts a comment
  to the end of the line, also glued to a name (`a.so#b.so` loads a.so only); no trailing newline needed;
  empty file = nothing. Every dynamic start does `access("/etc/ld.so.preload", R_OK)` (one syscall).
  Liella does one `openat` instead (ENOENT when absent): same count.

## What the WIP code does (crates/ld/src/start.rs, search.rs, dlopen.rs; crates/boundary/src/ld.rs)
- start.rs: `preload_file_names`, `PreloadRule` + `preload_rule` (secure+env+slash = Skip; secure+env
  bare = TrustedSetUid; else Search), `preload_mode_admits` (S_ISUID only), `preload_ignored(name, source,
  reason)` (7 parts), `PRELOAD_ENV`, `PRELOAD_MISSING`. Old 1-arg `preload_ignored` replaced.
- dlopen.rs: `Refusal::at_preload` (Directory -> bare `cannot read file data`).
- search.rs: `trusted_dirs::<A>(each)`.
- boundary ld.rs: secure mode reads LD_PRELOAD before `clear_unsecure` (`secure_preload`); `List.preload`;
  `run` opens `<prefix>/etc/ld.so.preload` in the PT_DYNAMIC branch and links when DT_NEEDED or any
  preload name exists; `Preloads` (names(), ignored()); `link(.., &preloads)` feeds preloads first at
  q == 0 (no edges, parent 0, ignored-with-message on missing/refused, Skip silently); `file_mode(fd)`.

## Next step (exact)
1. Sync (`sync_role.sh --story DL-LD-PRELOAD --base origin/next <wt> keke n3`, check rc), then
   `cargo clippy --workspace --all-targets --all-features -- -D warnings` on the role; fix compile errors
   (likely spots: the `pre_names` iterator's Ok type needs `Ok::<_, ElfError>`; borrow of `q` in from_fn;
   `try_open` closure borrowing `found`/`refused`/`at_path` mutably then reading them after).
2. Update tests: crates/ld/tests/ldso_run.rs `ldso_run_reads_ld_settings` (now: names missing ->
   `(cannot open shared object file)`), `ldso_run_secure_child_sees_unsecure_cleared` (stderr is now exactly
   the `object 'x' from LD_PRELOAD ... (cannot open shared object file): ignored.` line: reference does this),
   `preload_names` unit test. New tests: interposition + RTLD_NEXT + order + ignored texts + file syntax
   (build like crates/ld/tests/dlsym.rs `setup`), secure (set-group-ID corpus as ldso_run.rs secure test:
   slash skipped silently, bare trusted-dir lib without S_ISUID -> message), file via `unshare -r -m`
   bind over the prefix's etc (or a test-only prefix).
3. spec/behaviour/dynlink-preload.md (the probe record above) + tools/probe script; docs/install-packages.md.
4. fakeroot: add `fakeroot` to swap-system.sh's `--include` and fakeroot cases to swap-cmds.sh, run as
   uid 65534 inside the root (perl drop: `$( = $) = 65534; $< = $> = 65534`), expected `0` / `0 0` inside
   and `65534` outside; tar with --numeric-owner shows 0/0. Mirror http://miko.mgmt.internal/debian,
   KEYRING ~/keyrings/debian-archive-keyring.gpg (state/MIRRORS.md in liella-ops).
5. Plants (cargo xtask plants): preload_rule arms, preload_mode_admits, preload order (no edge / before
   DT_NEEDED), secure_preload capture.
6. Gate + start-up checks (`cargo xtask test`; LIELLA_FULL_PROFILE hot_text_products): this touches the loader start.

## Open / for the coordinator
- The "Not available yet" list is in liella-ops release/snapshot-2026-11-15-notes.md (read-only for agents):
  remove `LD_PRELOAD` there when this lands.
- INFORMATIONAL: `ld.so --list`/LD_TRACE_LOADED_OBJECTS does not show preloads yet; `ld.so --preload` option not done.
