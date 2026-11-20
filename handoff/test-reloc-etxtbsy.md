# TEST-RELOC-ETXTBSY: handoff

Premise (none stated in the brief): the failure is a descriptor-inheritance race in the test process, not a loader defect.

## The race

`reloc_ldso_kernel_started_without_proc` copied the corpus and the loader into a chroot with `std::fs::copy`
and then exec'd them through `unshare --root`. A file written by `std::fs::copy` is open for writing in the
test process until the copy returns. The tests of one binary run on parallel threads; a sibling thread that
forks or spawns in that window gives its child the descriptor (`O_CLOEXEC` closes it only at the child's own
`exec`). An `exec` of the new file before that child has exec'd fails with ETXTBSY (finding #6787).

Evidence: `exec_stage` test, 200 rounds, 4 sibling threads spawning `true` in a loop (mei, loaded host):

- before (the same rounds with `std::fs::copy`): 10 of 10 runs failed with `Text file busy` (os error 26), at
  rounds 0 to 34;
- after (`copy_for_exec` and `write_for_exec`): 0 of 10 runs failed (each run is 2 tests x 200 rounds).

## Fix

`crates/core/tests/helpers/exec_stage.rs`: `copy_for_exec(from, to)` (through `cp`) and
`write_for_exec(to, bytes, mode)` (bytes to a sibling file, then `cp`). The writing descriptor lives only in a `cp`
process that has exited before the call returns. Included with `#[path]`.

Sites (grep of `fs::copy`, `fs::write` and set_permissions/0o755 in `crates/*/tests`, then each checked for an exec):

- fixed: `ld/tests/reloc.rs` (the chroot copies, the failing test; the four earlier tmp+cp sites now use the helper),
  `ld/tests/preload.rs` (prog-sgid), `ld/tests/tls.rs` (execute-only loader copy), `core/tests/aio_engine.rs`
  (secure-copy), `core/tests/skip_precondition_fails.rs` (fake localedef script);
- already safe: `core/tests/hot_text_products.rs` (`stage()`, its own cp), `core/tests/s18_processes_clauses.rs`
  (`write_file` writes through `sh`);
- not an exec (a library is mapped, or data is read): `ld/tests/dlclose.rs`, `boundary/tests/reload.rs`,
  zoneinfo copies, `deb_stage_release.rs`.

The helper is in `helpers/`, not `support/`: a change under `support/` selects every core test (147 targets); here 12.

## Proof

`crates/core/tests/exec_stage.rs` (two tests) and the plant `exec-stage-inproc-copy` (helper uses `std::fs::copy`:
CAUGHT) in `handoff/TEST-RELOC-ETXTBSY-plants.txt`. An ignored raw-copy test used for the "before" run was dropped
(`static` long-sweeps wants every ignored test in a runner); the plant is the standing proof.

## Lesson

A test thread that panics inside `thread::scope` while sibling threads loop forever hangs the scope: the stop flag
is now set from a `Drop` guard.
