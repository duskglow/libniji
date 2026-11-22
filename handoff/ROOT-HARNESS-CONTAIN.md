# ROOT-HARNESS-CONTAIN: no test harness can leave root (or user-namespace) processes behind

State (ROOT-HARNESS-CONTAIN-3, merge-only continuation): branch `task/root-harness-contain-3`
(ROOT-HARNESS-CONTAIN's head 6fad2e155 plus a merge of origin/next). The CI conflict was
`crates/ld/tests/runpath_search.rs`: DL-RUNPATH-SECURE-TEST moved the namespace run into a shared
`in_namespace` helper used by two secure-mode tests. Resolved by taking next's version and building
that helper's command with `contain::user("unshare")`, so both its cases and the new caller-origin
case run contained. Plants: `handoff/ROOT-HARNESS-CONTAIN-3-plants.txt` (the five below plus
`static-runpath-namespace`). Jobs on keke role n4: `root-harness-contain-3-gate1` (direct tier,
157 targets), `root-harness-contain-3-plants1`. Report: `handoff/reports/ROOT-HARNESS-CONTAIN-3.json`.
Next step: none in this chain; ROOT-HARNESS-CONTAIN-2 (held) owns the "Open / next" tooling items.

Earlier state: branch `task/root-harness-contain`, code head 16231c5cc (later commits: handoff/report only).

## Done
- `crates/core/tests/support/contain.rs` (new; also `oracle::contain` through `support/oracle.rs`):
  - root kind: `output(&cmd)` / `status(&cmd, out, err)` run the command through `oracle/contain.sh`
    (it stays where it is: every user is under `crates/*/tests`, and the helper finds it from any
    crate). The stdin pipe is held until the child ends.
  - user kind: `user(program)` = `setpriv --pdeathsig KILL -- unshare --user --map-current-user
    --pid --fork --kill-child --mount-proc -- program`. The program keeps the caller's ids, and its
    own `unshare -r` nests.
- Every sudo harness is contained: aio, catalogs (nscache runs), dns, fstree, mallocdbg and
  patterns (nscache_mode runs only), mntent, netdb, nscache, posixipc, process, syslog,
  terminals, users.
- Every user-namespace harness run is contained: 15 in core, 2 in boundary, 6 in ld (the full
  list is in the doc). Probes are marked `contain: probe(...)` or run `true`.
- `crates/core/tests/harness_containment.rs`: one SIGKILL-mid-run test per kind. The root test
  moved here from dns_products (`a_killed_run_leaves_no_process` became
  `a_killed_root_run_leaves_no_process`).
- `xtask static` gains `harness-contain` (`xtask/src/harness_contain.rs`, with a unit test).
- Inventory and rule: `docs/testing/harness-containment.md`.
- Probe on mei (by hand, scratch only, no sudo), user kind:
  - SIGKILL of the caller leaves nothing (nested `unshare -r` plus a forked child).
  - uid is kept; pid 1 inside; exit status 7 passes through; SIGSEGV passes through as a signal.
  - pid 1 ignores a self-sent SIGSEGV. This is documented as a semantic of the helper.

## Jobs (mei, role s4)
- `root-harness-contain-tests1` (kind test: `cargo xtask so`, then every changed test target plus
  `xtask harness_contain`). The affected tier escalates to FULL (xtask files), so FULL is the
  train's.
- `root-harness-contain-plants1`: `handoff/ROOT-HARNESS-CONTAIN-plants.txt` (5 plants: user without
  pdeathsig, user without the PID namespace, root without its watcher, a direct sudo spawn, an
  uncontained script user).

## Open / next
- Not contained (tooling, not tests; listed in the doc):
  - xtask bench `private_shm` (root `sudo` waits on the user's bench; containing it changes the
    counted bench's PID namespace: count cbench_netdb/cbench_nscache before and after)
  - validation/profile*.sh (sudo perf)
  - xtask transcript/bench `unshare -r -m`, tools/deb, tools/probe
- Static rule B is file-level (one `contain::` use in a file satisfies it); rule A is per line.

## Do not redo
- `unshare -r --pid` alone is not enough: the `unshare` outlives a SIGKILLed test without the
  parent-death signal (plant user-no-pdeathsig).
