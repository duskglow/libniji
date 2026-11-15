# DL-TLS-DTOR-PIN: a pending thread_local destructor keeps its object loaded (spec s1_2_c_6)

## State

Branch `task/dl-tls-dtor-pin` (from origin/next 2d5625a3e), role keke:~/w/n1. Code commits: `fd1e4791c` (the pin,
the test, the plants file, the pending record), `4c7534584` (`LOADER_PIN` declared in
crates/libc-so/startup-state.list). This note and handoff/reports/DL-TLS-DTOR-PIN.json follow.

RESULTS: see "Jobs" below.

## What was done

Found by AQOURS-SMOKE-RERUN (cxx_dlclose / plug_tl.cc): a plugin's C++ `thread_local` destructor, registered through
`__cxa_thread_atexit_impl(fn, obj, dso)`, ran after `dlclose` had finalised and unmapped the plugin (SIGSEGV at exit).
Liella code only (an unreleased development defect: fixed in the open per the owner's 2026-11-14 note in the brief).

* **Loader** (crates/boundary/src/ld/dlopen.rs): `Node::pins`; `pin_entry(op, arg)` (`StartInfo::dl_pin`):
  `(0, dso)` pins the `dlopen`ed object holding `dso` (index+1 token; 0 for the program, initial objects, `ld.so`,
  NULL, no object), under `STATE` only (a constructor under `CALL` may register); `(1, token)` drops it and, at the
  last pin with no open and not `nodelete`, unloads what is no longer live exactly as `dlclose` (`collect`, split out
  of `close_begin`; finalisers under `CALL`; `close_end`). `live_set` roots `pins > 0`. A saturated count sets `nodelete`.
* **libc** (crates/boundary/src/process.rs): `thread_atexit` takes the pin and stores the token in the node word that
  held `dso`; `run_thread_dtors` drops it after the destructor returned (crates/legacy/src/exit_handlers.rs: the loop
  hands `(node, pin)` to its callback after the call). `LOADER_PIN` is set in `runtime::early_init` and declared in
  startup-state.list (unlisted it sat alone on an 8th written start-up page: STARTUP-DATA-WINDOWS broke, job tests1).
* **Record:** records/pending/DL-TLS-DTOR-PIN-1.toml (D-NEW, series D-27, amends D-27.50: removes its known edge
  "thread_local destructors a plugin registered do not pin it"). CI assigns the id.

## Behaviour (test `s1_2_c_6_thread_local_destructor_keeps_its_object_loaded`, crates/ld/tests/dlclose.rs)

`exit` mode (the smoke scenario in C): register on main, `dlclose`, still loaded, nothing finalised; `exit` prints
`m F ` (destructor, then the static destructor). `threads` mode: `a b k F c F x F ` - a destructor before the close;
two threads pending across the close; a fork child (registers its own, exits: `k F`); the last destructor unloads and
unmaps (no leak: `!loaded && !mapped`); a destructor that NOLOAD-reopens and closes its own last open runs to its end
in the object, which unloads after it returns. Both modes kernel-started and as `ld.so PROGRAM`.

## Known edges (in the record)

fork child: pins of threads that do not exist in the child are never dropped there; such an object stays loaded in
the child until it exits (safe direction; its exit's `fini_loaded` finalises it). Pinning an object while its own
`dlclose` finalisers run keeps it (its destructors then run again later), the same edge as D-27.50's re-open.
Not probed against the reference here (the brief and spec s1_2_c_6 state the behaviour; the reference order was
observed by AQOURS-SMOKE-RERUN).

## Jobs (keke, all on head 4c7534584, chain ~/work/DL-TLS-DTOR-PIN/chain.sh)

* `dl-tls-dtor-pin-lint2`: rc 0 (clippy --all-features -D warnings, `cargo xtask static`, `records check`).
* `dl-tls-dtor-pin-tests2`: rc 0 (`cargo xtask so` with every layout contract, then all 5 tests of
  crates/ld/tests/dlclose.rs, the new one included). (`cargo test -p liella-legacy exit_handlers` matched no test:
  the legacy crate has no unit test of the thread destructor loop; the C test covers it.)
* `dl-tls-dtor-pin-plants1`: rc 0, 5 plants: 5 CAUGHT, 0 SURVIVED, 0 WARN (handoff/DL-TLS-DTOR-PIN-plants.txt).
* `dl-tls-dtor-pin-gate1` (direct tier, `--base origin/next`), `dl-tls-dtor-pin-xtest1` (`cargo xtask test`),
  `dl-tls-dtor-pin-release1` (`LIELLA_FULL_PROFILE=1` so + `hot_text_products`): see the hand-back for their state
  (queued one after another; read `~/logs/jobs.tsv` on keke).
* Superseded: `dl-tls-dtor-pin-lint1` (fd1e4791c: clippy clean, record skeleton), `dl-tls-dtor-pin-tests1`
  (fd1e4791c: rc 1, STARTUP-DATA-WINDOWS broken by the unlisted `LOADER_PIN`; fixed in 4c7534584).

## Next step

Land. Possible follow-up: drop the pins of threads gone in a fork child (needs every thread's destructor list).

## Process notes

Two rule slips in this run, both on my own files: one heredoc (a local `python3 - <<EOF` edit of dlopen.rs) and one
`sed -i` on the pending record. Neither touched a shared file or a host.
