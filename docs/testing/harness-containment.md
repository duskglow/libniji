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
`contain::output(&cmd)` (root, through `oracle/contain.sh`; the socket pair that ties the run to
xtask is held for xtask's lifetime) and `contain::user(program)`.

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

**Count check (ruling #7469).** `cbench_netdb` and `cbench_nscache` are the only benches on the
root route. Until this change neither printed counters on it, for two reasons: `cbench_netdb`'s
`MEASURE` never started the counters (it does now, once per row outside the timed rounds, as the
other C benchmarks), and `sudo`'s environment reset dropped `CBENCH_COUNTERS` and `CBENCH_ROWS`
(now passed as `env` arguments, `bench::past_sudo`). `cbench_nscache` includes a libniji-only
header, so it has no host build and no counted mode; it was not counted.

`cbench_netdb`, `cargo xtask bench --compare --counters --runs 1`, the same products, uncontained
(`NIJI_CONTAINED=none`) and contained runs alternating. Skylake-SP, one CPU class only (no Zen 1
role in this run). Instructions per operation, all modes (the counted group's pinned
`instructions` event, kernel included), median (min-max). Five runs each way:

| Row | libniji uncontained | libniji contained | diff % | glibc uncontained | glibc contained | diff % |
|---|---:|---:|---:|---:|---:|---:|
| `inet_pton_v4` | 398.4 (398.3-398.7) | 398.6 (398.4-398.6) | +0.05 | 361.4 (361.3-361.8) | 361.4 (361.4-361.7) | +0.00 |
| `inet_pton_v6` | 834.8 (834.6-835.3) | 835.0 (834.8-835.2) | +0.02 | 692.8 (692.7-693.7) | 692.8 (692.7-693.5) | +0.00 |
| `inet_ntop_v4` | 550.5 (550.4-550.9) | 550.7 (550.5-550.9) | +0.04 | 1611.5 (1609.7-1612.1) | 1611.7 (1611.4-1613.4) | +0.01 |
| `inet_ntop_v6` | 651.5 (651.5-652.4) | 651.7 (651.5-651.9) | +0.03 | 3390.4 (3389.6-3403.3) | 3390.4 (3390.1-3394.1) | +0.00 |
| `inet_aton` | 418.3 (418.3-418.6) | 418.4 (418.4-418.6) | +0.02 | 638.6 (638.5-640.0) | 638.6 (638.6-639.0) | +0.00 |
| `getaddrinfo_numeric_v4` | 2646.6 (2646.3-2651.1) | 2648.1 (2646.5-2649.5) | +0.06 | 1102.1 (1102.0-1104.2) | 1102.4 (1102.3-1103.0) | +0.03 |
| `getaddrinfo_numeric_v6` | 2273.1 (2272.9-2276.3) | 2274.5 (2273.1-2276.1) | +0.06 | 1082.2 (1082.1-1084.4) | 1082.3 (1082.2-1083.1) | +0.01 |
| `getaddrinfo_localhost` | 23665.6 (23662.1-23676.9) | 23669.4 (23660.5-23675.4) | +0.02 | 37813.9 (37736.3-37919.5) | 37732.0 (37709.5-37868.0) | -0.22 |
| `getnameinfo_numeric` | 1404.2 (1404.1-1407.4) | 1405.4 (1405.3-1406.4) | +0.09 | 2206.8 (2205.9-2208.9) | 2206.1 (2206.1-2207.5) | -0.03 |
| `getpwnam_r_root` | 4818.2 (4818.0-4825.0) | 4818.7 (4818.3-4822.0) | +0.01 | 20303.3 (20243.7-20425.1) | 20251.1 (20246.4-20269.1) | -0.26 |
| `getgrgid_r_root` | 4018.5 (4018.1-4022.2) | 4019.2 (4018.5-4021.9) | +0.02 | 20098.6 (20062.1-20225.9) | 20071.7 (20062.8-20120.0) | -0.13 |
| `getservbyname_r_http` | 11210.4 (11203.9-11210.8) | 11212.2 (11205.3-11214.3) | +0.02 | 45930.4 (45847.1-45989.5) | 45837.7 (45808.7-45840.1) | -0.20 |
| `getpwnam_r_named` | 4825.0 (4824.5-4828.9) | 4828.1 (4824.5-4831.9) | +0.06 | 20294.3 (20237.7-20364.3) | 20264.3 (20217.4-20429.1) | -0.15 |
| `getgrnam_r_named` | 4059.7 (4059.2-4063.1) | 4060.1 (4059.7-4063.2) | +0.01 | 20061.3 (20037.0-20085.5) | 20112.1 (20024.7-20158.6) | +0.25 |
| `getgrouplist_named` | 8870.5 (8861.5-8874.4) | 8874.9 (8861.5-8877.8) | +0.05 | 64060.1 (64000.2-64071.0) | 64004.6 (63994.6-64247.4) | -0.09 |
| `res_mkquery` | 10431.7 (10427.5-10453.1) | 10432.4 (10430.7-10439.8) | +0.01 | 8626.3 (8623.0-8631.7) | 8624.0 (8619.1-8628.4) | -0.03 |
| `dn_comp` | 1433.3 (1433.2-1436.0) | 1433.6 (1433.4-1434.7) | +0.02 | 850.7 (850.7-851.4) | 850.8 (850.7-851.1) | +0.01 |
| `dn_expand` | 641.6 (641.5-641.9) | 641.6 (641.5-642.0) | +0.00 | 563.4 (563.3-563.6) | 563.4 (563.3-563.6) | +0.00 |

Every contained range overlaps the uncontained one except glibc's `getservbyname_r_http`
(contained 0.2 % lower, outside the range by 7 of about 45 800 instructions). The medians differ by at most 0.09 %
(libniji) and 0.26 % (glibc, whose files-database rows spend most of their count in the kernel).
The syscall-free rows (`inet_*`, `dn_*`) are 0.00 to 0.05 % higher contained (libniji) and
0.00 to 0.01 % (glibc). In
those loops only kernel work done during the row (interrupts, scheduling) can differ, so the shift
is attributed to the kernel side. That attribution is an inference: it was not measured. The
user-only column (`niji_instr_user` in `measure.tsv`) cannot settle it. That event is outside the
pinned group and is multiplexed: one row spread from 926 to 7221 instructions across runs. Some
runs shared the host with other builds.
