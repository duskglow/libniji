# Test harness containment

A test that starts processes in namespaces must not leave them running when it is killed
(SIGKILL: no cleanup code runs) or hangs and its runner ends it. A root orphan cannot be
signalled by the developer or by `timeout`. A user-namespace orphan can be, but it holds files,
ports and memory until someone notices. The helpers are in `crates/core/tests/support/contain.rs`
(also `oracle::contain` for the files that include `support/oracle.rs`). Both kinds put the run in
a PID namespace of its own: when that namespace's first process ends, the kernel kills every
process in it (pid_namespaces(7)).

| Kind | Helper | How the run is tied to the test |
|---|---|---|
| Root (`sudo -n` anywhere below the command) | `contain::output(&cmd)` / `contain::status(&cmd, out, err)` through `crates/core/tests/oracle/contain.sh` | The command runs as the caller, as process 1 of a PID namespace made by root (the root processes below it are in it). Its standard input is a pipe whose write end only the test holds; a root watcher kills the namespace when that pipe ends. |
| User namespace (`unshare -r`, `--map-root-user`, a script that runs one) | `contain::user(program)` | `setpriv --pdeathsig KILL -- unshare --user --map-current-user --pid --fork --kill-child --mount-proc -- program`: the program keeps the caller's ids and its own `unshare -r` nests. The `unshare` dies with the spawning thread, and `--kill-child` takes the namespace with it. |

The command is process 1 of its namespace. A default-action signal sent to it from inside (its own
`raise`, a `kill` from a child) is ignored. A fault, or a signal from outside, still ends it, and
the exit status passes through (checked on a Skylake-SP host, util-linux 2.39.3).

**Proof.** `crates/core/tests/harness_containment.rs` runs one test per kind. Each test re-runs its
own binary as a helper that starts a run (a process and a forked child), SIGKILLs the helper, and
checks that no process of the run remains. The same file has one kill test per kind of
`tools/contained.sh` (below): a script with the prologue starts a run, the test SIGKILLs the
script's own process, and no process of the run may remain. `xtask/src/contain.rs` has a kill test
per kind for xtask's own runs. `cargo xtask static` (`harness-contain`,
`xtask/src/harness_contain.rs`) has these rules:

- A test line that spawns `sudo` or `unshare` directly must be a probe: it runs `true`, or it is
  marked `contain: probe(<why>)`.
- A test file that runs an oracle script that uses `sudo` or `unshare` must use `contain::`.
- An xtask source line (`xtask/src/**/*.rs`) that spawns `sudo` or `unshare` directly
  (`Command::new("sudo")`, `Command::new(unshare)`, ...) is a probe, or is marked
  `contain: inside(<why>)` when it runs inside a contained run.
- A script under `validation/` or `tools/`, or an oracle script that no test names (run by hand),
  whose code runs `sudo` or `unshare` sources `tools/contained.sh`, with `contained_kind=root` or
  `rootopt` when it runs `sudo`.

## Inventory (ROOT-HARNESS-CONTAIN, 2026-11-22)

**Root, contained by `contain::output` / `contain::status`.** Each test runs its oracle script
(the scripts run `sudo -n ... unshare`), most of them under `nscache_mode.sh` (`sudo -n unshare -m`,
then `setpriv` back to the caller):

- `aio_products`
- `catalogs_products` (the nscache runs; `catalogs_unstable` runs no sudo)
- `dns_products`
- `fstree_products`
- `mallocdbg_products` and `patterns_products` (the `nscache_mode.sh` runs only)
- `mntent_products`
- `netdb_products`
- `nscache_products`
- `posixipc_products`
- `process_products`
- `syslog_products`
- `terminals_products`
- `users_products`

Until this change only `dns_products` was contained. The others ended with the script, or with
the test's `timeout`, which cannot signal the root processes.

**User namespace, contained by `contain::user`:**

- `crates/core/tests`:
  - `admin_products` and `userns_paths` (their namespaced runs)
  - `gai_down_products`, `langpack_products` and `time_gaps_probe` (their scripts run `unshare -r`)
  - `hostconf_env_products`
  - `iconv_charmaps_products`
  - `no_proc_lane`
  - `noproc_products`
  - `s12_stdio_clauses`, `s13_filesystem_clauses`, `s24_system_clauses`, `s25_ipc_clauses`
  - `sigev_thread`
- `crates/boundary/tests`: `s24_system_symbols`, `s25_ipc_symbols`
- `crates/ld/tests`: `cache_write`, `object_names`, `preload`, `reloc`, `runpath_search`,
  `search_cache`

**Not contained: probes.** These are `unshare ... true`, `sudo -n true`, or a fixed mount and exit
marked `contain: probe(...)`. Each ends by itself and waits for nothing.

## Outside the tests (ROOT-HARNESS-FOLLOWUP, 2026-11-22)

**xtask's own runs, `xtask/src/contain.rs`.** The same two kinds as the tests' helpers:
`contain::output(&cmd)` (root, through `oracle/contain.sh`; the pipe that ties the run to xtask
is held for xtask's lifetime) and `contain::user(program)`.

- `xtask bench`, the private-/dev/shm route of `cbench_netdb` and `cbench_nscache` (`sudo -n
  unshare -m sh -c '... setpriv ... bench'`): every bench spawn goes through
  `bench::bench_output`, which runs this route (and only it) through `contain::output`. It is
  wrapped at the spawn, not where the command is built, because the callers add `CBENCH_COUNTERS`
  and `CBENCH_ROWS` afterwards. `sudo` is process 1 of the namespace; the bench is its descendant,
  never process 1. Count check: below.
- `xtask bench` `namespace_command` (the `names` and `locale` stress profiles, advisory):
  `contain::user("unshare")`.
- `xtask transcript`, the namespaced transcripts (`getent`, `locale`, `ldconfig`):
  `contain::user(unshare)`; the nested `unshare` exec in its helper is marked `contain:
  inside(...)`. The transcript's program may be process 1 of the namespace; none of these
  transcripts expects a signal outcome.

**Scripts, `tools/contained.sh`.** A script sources it before it starts anything, with
`contained_kind=root|rootopt|user` (the header of `tools/contained.sh` has the mechanism):

| Script | Kind | Why |
|---|---|---|
| `validation/profile.sh`, `validation/profile6.sh` (`sudo -n perf ...`) | `root` | the profiled command runs under root `perf`; the script re-runs through `contain.sh` |
| `validation/tmpfs-run.sh`, `tools/deb/root-system.sh`, `tools/probe/util_surface.sh`, `tools/probe/ldcache_loader.sh`, `tools/probe/dl/preload/run.sh` and `run2.sh`, `crates/core/tests/oracle/specman_dns.sh` | `user` | their own `unshare -r` nests in a user namespace mapping only the caller |
| `tools/deb/proof.sh`, `lint.sh`, `swap-system.sh`, `dbg-system.sh` (mmdebstrap `--mode=unshare`), `tools/probe/dlsecure/run.sh` (`--map-users=auto`) | `rootopt` | they map subordinate id ranges (newuidmap), which a user namespace that maps one id cannot hold, so the `user` kind cannot nest them; where `sudo -n` works they run as `root` (the PID namespace is root's, the script still runs as the caller); elsewhere uncontained, with a note on stderr |

`rootopt` is the one remaining uncontained case: a host without passwordless `sudo` runs those
five tools uncontained, and they say so. They run there by hand (Debian packaging and probes),
never under a test runner. mmdebstrap 1.4.3 keeps no parent-death tie to its namespace.

**Count check (ruling #7469).** COUNTS-TO-FILL
