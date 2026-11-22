# ROOT-HARNESS-FOLLOWUP: contain the root and namespace launches outside the tests

State: branch `task/root-harness-followup` (from origin/next 5f07429d3), WIP commit, NOT compiled yet (no local
cargo; the role could not be synced: see "Count check" below). Role: mei `~/w/s1` (Skylake-SP class).
Brief: `~/state/briefs/ROOT-HARNESS-FOLLOWUP.md`. Predecessor: `handoff/ROOT-HARNESS-CONTAIN.md`.

## Done (in the WIP commit, unverified)
- `xtask/src/contain.rs` (new, `mod contain` in main.rs): `user(program)` (same argv as the tests'
  `support/contain.rs`), `root(cmd)` (contain.sh + `env K=V` args) and `output(cmd)`: stdin = read end of a
  process-lifetime pipe (`static HOLD`, `std::io::pipe`, close-on-exec), so contain.sh's watcher ends the namespace
  when xtask dies. Unit tests: kill test per kind (re-exec helper, SIGKILL, nothing carrying the token remains;
  root skips without `sudo -n`), and `root_form_keeps_arguments_and_variables`.
- `xtask/src/bench.rs`: `pub(crate) fn bench_output(&mut Command)` + `is_root_route` (program == "sudo");
  every C-bench spawn (bench.rs 6 sites, abbench.rs 2, startpair.rs 1) goes through it. Wrapped at spawn, not in
  `private_shm`, because callers add CBENCH_COUNTERS / CBENCH_ROWS after building the command. `private_shm`: no
  `exec` before `setpriv` (the bench stays a child of the root sh, never PID 1); its `Command::new("sudo")` is
  marked `contain: inside(...)`. `namespace_command`: `crate::contain::user("unshare")`, and no `exec` before the
  nested `unshare -U` (bench not PID 1).
- `xtask/src/transcript/run.rs`: namespaced transcripts spawn through `crate::contain::user(unshare)`; the nested
  exec in the helper is marked `contain: inside(...)`. The transcript program becomes PID 1 (96 namespaced
  transcripts: getent, locale, ldconfig; none expects a signal outcome).
- `tools/contained.sh` (new, sourced prologue): kinds `root` (re-run through contain.sh; FIFO held by the outer
  shell; CONTAINED_KEEP vars + HOME passed), `rootopt` (root where `sudo -n` works, else uncontained with a stderr
  note), `user` (exec setpriv --pdeathsig KILL -- unshare --user --map-current-user --pid --fork --kill-child
  --mount-proc). NIJI_CONTAINED marks the re-run; `none` = caller's opt-out.
- Prologue added to: validation/profile.sh, profile6.sh (root), validation/tmpfs-run.sh (user), tools/deb/lint.sh,
  swap-system.sh, dbg-system.sh, proof.sh (rootopt: mmdebstrap --mode=unshare needs newuidmap ranges, which a
  nested single-id user namespace cannot map; mmdebstrap 1.4.3 keeps no parent-death tie, checked in
  /usr/bin/mmdebstrap on mei), tools/deb/root-system.sh, tools/probe/util_surface.sh, ldcache_loader.sh,
  dl/preload/run.sh, run2.sh (user), tools/probe/dlsecure/run.sh (rootopt: `--map-users=auto`),
  crates/core/tests/oracle/specman_dns.sh (user).
- `xtask/src/harness_contain.rs`: + xtask sources (line rule, `Command::new(unshare)` too, marker
  `contain: inside(`), + scripts (validation/**/*.sh, tools/**/*.sh, oracle/*.sh no test names): must source
  tools/contained.sh, root/rootopt kind if `sudo `. Unit test `xtask_spawns_and_tool_scripts_are_named`.

## Count check (brief step 1)
- Job `root-harness-followup-count-before` (kind count) started 09:53Z on mei, QUEUED behind release-snapshot
  slots: `~/work/root-harness-followup/count.sh before` = 5 x `cargo xtask bench --counters-only --only
  cbench_netdb,cbench_nscache` on `~/w/s1` (origin/next as synced by the dispatcher), copies
  `target/niji/bench/counters-<bench>.tsv` to `~/work/root-harness-followup/counts-before/<bench>-<i>.tsv`.
  DO NOT sync the role until it has ended (`~/bin/jobwait root-harness-followup-count-before`).
- Then: sync, `count.sh after` as job `root-harness-followup-count-after`, compare instructions per row
  (before vs after, 5 each). Also check the counters columns are present for the shm benches at all (sudo resets
  the environment: if CBENCH_COUNTERS does not reach the bench today, say so; it is the same before and after).
  Only one CPU class (no second role).

## Next steps (in order)
1. jobwait count-before; sync (`bash ~/src/liella-ops/tools/sync_role.sh --story ROOT-HARNESS-FOLLOWUP --base
   origin/next <worktree> mei s1`, check rc); on the role: `cargo clippy --workspace --all-targets --all-features
   -- -D warnings`, `cargo xtask static` (harness-contain must be clean: fix any script it names), fix compile.
2. Start count-after; run `cargo test -p xtask contain:: harness_contain bench::` (kind test) and the namespaced
   transcripts (`cargo xtask transcript` on validation/transcripts/{locale,getent,ldconfig}, check its usage).
3. Shell kill tests: add to `crates/core/tests/harness_containment.rs` one test per script kind (write a temp
   script that sets contained_kind=root|user, sources tools/contained.sh, starts a token-carrying loop + forked
   child (root: under `sudo -n`); SIGKILL the script's pid; nothing carrying the token remains).
4. Plants file `handoff/ROOT-HARNESS-FOLLOWUP-plants.txt` (docs/plants.md format): xtask user without
   --kill-child (contain.rs USER), xtask root output without the held stdin, bench_output bypass (is_root_route ->
   false), contained.sh user without --pdeathsig, contained.sh root not closing fd 8 for the child, static: a
   script prologue removed (validation/profile.sh), static: transcript marker removed. Run via `cargo xtask
   plants` under jobrun (`/mnt/zram/claude/root-harness-followup/plants-1`).
5. Doc `docs/testing/harness-containment.md`: replace "Not contained yet" with the new inventory (xtask kinds,
   scripts + kinds, rootopt reason, PID-1 note for transcripts), static rule text, count table.
6. Gate (`cargo xtask affected` likely FULL: xtask files -> precheck + changed crates' tests), report json,
   handback-check.
