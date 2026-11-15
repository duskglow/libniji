# DL-TLS-DTOR-PIN: a pending thread_local destructor keeps its object loaded (spec s1_2_c_6)

## State

Current: branch `task/dl-tls-dtor-pin-3`, code head `c5f978407` (origin/next merged twice, see -3 below), role
keke:~/w/r1; the record is D-27.53 (records/decisions/D-27/D-27.53.toml, amends D-27.50).
Before: branch `task/dl-tls-dtor-pin-2` (continues `task/dl-tls-dtor-pin`, from origin/next 2d5625a3e), role keke:~/w/n2.
Code commits: `fd1e4791c` (the pin, the test, the plants file, the pending record), `4c7534584` (`LOADER_PIN`
declared in crates/libc-so/startup-state.list), `239ea5ccc` (DL-TLS-DTOR-PIN-2: the test's race fixed, below).
This note, handoff/reports/DL-TLS-DTOR-PIN.json and handoff/reports/DL-TLS-DTOR-PIN-2.json follow.

## DL-TLS-DTOR-PIN-3: the merge with next

Branch `task/dl-tls-dtor-pin-3` (from `task/dl-tls-dtor-pin-2` 44d39d9f6), role keke:~/w/r1. CI could not merge
origin/next: conflict in crates/boundary/src/process.rs. Next's side (TB-FOREIGN-WORDS-LOW, D-X.60) reads the
calling thread's destructor list through `crate::thread::words()` instead of `tb::current()` (the word is never
written in a thread block another library made); this branch's side takes the pin in `thread_atexit` and drops it
in `run_thread_dtors`. Merge commit `1fc388112` keeps both: the pin token goes into the list reached through
`thread::words()`, the callback still drops it after the destructor ran. No other file conflicted, and next changed
none of this branch's other files (exit_handlers.rs, dlopen.rs, startup-state.list, dlclose.rs). No product code
beyond the resolution changed in -3. Jobs: see "Jobs (-3)" below.

Second merge `c5f978407`: while the first merge's jobs ran, DL-ATEXIT-OWNER's opt-in registrations (D-27.29, ruling
#4862) landed on next and conflicted in two files. crates/boundary/src/ld/dlopen.rs: next looks up the C library's
`__liella_owner_unload` (`Finis::owner_unload`) when a `dlclose` unloads something; this branch had split that
collection step into `collect`, shared by `dlclose` and the last-pin unload (`unpin`). The lookup now ends `collect`,
so an object unloaded by its last thread_local pin also gets the opted-in unload step while its code is mapped
(`unpin` runs `Finis::run` exactly as `close_entry` does). crates/legacy/src/exit_handlers.rs: next's
`finalize_range` kept as is, then this branch's pin-carrying `thread_atexit`. process.rs merged cleanly.
Not covered by a test of its own: an opted-in registration of an object unloaded by its last pin (the two
features' tests each cover their own path; the shared `Finis::run` is the same code).

## DL-TLS-DTOR-PIN-2: why gate1 was red

`dl-tls-dtor-pin-gate1` failed the new test in `threads` mode, kernel-started: killed by SIGALRM (the program's
`alarm(120)`) with output `a c `. The cause was the test program, not the pin: threads `b` and `c` both waited on
one semaphore `go`; `main` posted it once and joined `b`. `sem_post` wakes an unspecified waiter (POSIX), so under
the gate's load it woke `c` (its destructor ran, the object correctly stayed loaded under `b`'s pin), `b` stayed
blocked and the join hung. Fix `239ea5ccc`: each held thread has its own release semaphore (`struct hold`), so the
order `b` then `c` is fixed. No product code changed in -2. Repeat evidence: `dl-tls-dtor-pin-2-rep1`, the built
test 60 times from a RAM copy at load ~20 on keke: 0 failures.

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

## Jobs (-3) on the final code head c5f978407 (keke role r1, chain ~/work/DL-TLS-DTOR-PIN-3/chain3.sh)

* `dl-tls-dtor-pin-3-lint2`: rc 0 (clippy --all-features -D warnings, `cargo xtask static`, `records check`).
* `dl-tls-dtor-pin-3-tests2`: rc 0 (`cargo xtask so`, then crates/ld/tests/dlclose.rs: 5 passed, s1_2_c_6 ok).
* `dl-tls-dtor-pin-3-rep2`: rc 0, the built s1_2_c_6 test 60 times from a RAM copy, load ~14: 0 failures.
* `dl-tls-dtor-pin-3-plants2`: rc 0, 5 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG.
* `dl-tls-dtor-pin-3-gate2` (`affected --run --base origin/next`, escalates to FULL, so it runs `cargo xtask test`
  with its products step) and then `dl-tls-dtor-pin-3-release2` (`LIELLA_FULL_PROFILE=1` so + `hot_text_products`):
  RUNNING at hand-back (gate ~104 min on this host), left for the dispatcher.

## Jobs (-3) on the first merge head 1fc388112 (superseded by the second merge; chains chain.sh, chain2.sh)

* `dl-tls-dtor-pin-3-lint0`: rc 0 on the inherited head 44d39d9f6 (clippy --all-features -D warnings, static,
  `records check`, 0 pending): nothing left red by -2.
* `dl-tls-dtor-pin-3-lint1`: rc 0 (the same three on the merge head).
* `dl-tls-dtor-pin-3-tests1`: rc 0 (`cargo xtask so`, then crates/ld/tests/dlclose.rs: 5 passed, s1_2_c_6 ok).
* `dl-tls-dtor-pin-3-rep1`: rc 0, the built s1_2_c_6 test 60 times from a RAM copy, load ~22: 0 failures.
* `dl-tls-dtor-pin-3-plants1`: rc 0, handoff/DL-TLS-DTOR-PIN-plants.txt: 5 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY,
  0 WARN-NO-MSG (one WARN-DEV-SHM note: the copy is under /dev/shm, no test here mounts its own /dev/shm).
* `dl-tls-dtor-pin-3-gate1`: `affected --run --base origin/next`; it escalated itself to FULL (startup-state.list is
  a product linker input), so it runs the workspace clippy and `cargo xtask test` (the start-up products check).
  rc 0: 32 suites ok, 11877 passed / 0 failed (1397 ignored), conformance 137 / 0, products step ran,
  peak 13G, oom_kill 0 (104 min).
* `dl-tls-dtor-pin-3-release1` (`LIELLA_FULL_PROFILE=1` so + `hot_text_products`), run after gate1 by chain2.sh:
  rc 0: 12 layout contracts hold, STARTUP-DATA-WINDOWS 8 of 8 written pages, hot_text_products 26 passed.
* No separate `xtest1`: chain.sh queued one after gate1; its shell was stopped before it started because the
  escalated gate1 already runs `cargo xtask test` (it never ran; no jobs.tsv row).
* Builds on next now include trace-at-build: `cargo xtask so` took ~17 min on keke at load ~22.

## Jobs (-2) (keke role n2, all on code head 239ea5ccc, chain ~/work/DL-TLS-DTOR-PIN-2/chain.sh)

* `dl-tls-dtor-pin-2-lint0`: rc 0 on the inherited head 4c7534584 (clippy --all-features -D warnings, `cargo xtask
  static`, `records check`): nothing left red by the first run.
* `dl-tls-dtor-pin-2-lint1`: rc 0 (the same three on 239ea5ccc).
* `dl-tls-dtor-pin-2-tests1`: rc 0 (`cargo xtask so`, then crates/ld/tests/dlclose.rs: 5 passed).
* `dl-tls-dtor-pin-2-rep1`: rc 0, the built s1_2_c_6 test 60 times from a RAM copy, load ~20: 0 failures.
* `dl-tls-dtor-pin-2-plants1`: rc 0, handoff/DL-TLS-DTOR-PIN-plants.txt: 5 CAUGHT, 0 SURVIVED, 0 WARN.
* `dl-tls-dtor-pin-2-gate1`: rc 0, direct tier `--base origin/next`: 6587 passed / 0 failed (interposed 5206 / 0),
  peak 13G, oom_kill 0; the new test passed in it (0.48 s).
* `dl-tls-dtor-pin-2-xtest1` (`cargo xtask test`), `dl-tls-dtor-pin-2-release1` (`LIELLA_FULL_PROFILE=1` so +
  `hot_text_products`): both rc 0 (xtest 6587 / 0 failed, interposed 5206 / 0; release: 12 layout contracts ok,
  STARTUP-DATA-WINDOWS 8 of 8 written pages, hot_text_products 26 passed).
* Superseded (first run, head 4c7534584): `dl-tls-dtor-pin-gate1` rc 1 (the test race above);
  `dl-tls-dtor-pin-lint2`, `-tests2`, `-plants1`, `-xtest1`, `-release1` (older head); `dl-tls-dtor-pin-lint1`,
  `-tests1` (fd1e4791c).

## Next step

Land (-3 head c5f978407) once `dl-tls-dtor-pin-3-gate2` and `-release2` pass. DL-ATEXIT-OWNER's registrations
landed on next before this branch and are merged in (second merge above). Possible follow-up: drop the pins of threads gone in a fork child (needs every thread's destructor list).
