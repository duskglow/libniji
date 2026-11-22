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
checks that no process of the run remains. `cargo xtask static` (`harness-contain`,
`xtask/src/harness_contain.rs`) has two rules:

- A test line that spawns `sudo` or `unshare` directly must be a probe: it runs `true`, or it is
  marked `contain: probe(<why>)`.
- A test file that runs an oracle script that uses `sudo` or `unshare` must use `contain::`.

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

**Not contained yet: outside the tests.** These are not test harnesses (follow-up):

- `xtask bench`'s `private_shm` route (`cbench_netdb`, `cbench_nscache`): `sudo -n unshare -m sh -c
  '... exec setpriv ... bench'`. Its only root process is `sudo`, which waits for the bench, a
  process of the caller's own. If xtask is killed, the bench runs on to its end, and `sudo` with
  it. Containing it puts the counted bench in a PID namespace, which needs a count check of those
  two rows first.
- `validation/profile.sh` and `validation/profile6.sh` (`sudo -n perf stat|record ... sh -c "$c"`):
  by-hand profiling tools. The profiled command runs under root `perf` until it ends.
- User-namespace tooling outside the tests:
  - `xtask bench` `namespace_command` and `xtask transcript` (`unshare -r -m`)
  - `validation/tmpfs-run.sh`
  - `tools/deb/*.sh` (mmdebstrap `--mode=unshare`)
  - `tools/probe/*`
  - `crates/core/tests/oracle/specman_dns.sh` (run by hand)
