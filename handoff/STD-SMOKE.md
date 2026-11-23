# STD-SMOKE: a Rust `std` program under libniji, and the docs corrected

Branch `task/std-smoke` (from `origin/next` e3b0caf60). Host keke (Zen 1, 60 vCPU), role n4. No decision records
added; uses the native pass of PASS2-NATIVE-RUNNER (D-X.135), which had landed, so no new set-up code.

## What changed

* `crates/boundary/tests/std_smoke.rs` (new target, not in `nextest::INTERPOSED`, so it runs in pass 1 and the
  native pass). The test re-executes `current_exe()` as "the program", which prints six `smoke: ` lines and exits
  with status 7: 8 threads (barrier, mutex, channel, `thread_local!` counted per thread), a caught panic (payload
  and a destructor run while unwinding) plus a panic seen through `join`, a child started through `std`'s
  fork-then-exec path (forced by `CommandExt::gid(<own gid>)`; the child is the binary again, exits 3 and reports
  its pid and loader), a file written, appended, read back, listed (sorted) and removed, an environment variable
  read and found in `env::vars()`, and the loader the process runs on. The parent compares the lines exactly and
  the status. In the native pass (`NIJI_TEST_PASS=native`) the program and its forked child must map the prefixed
  `ld.so` and libniji's `libc.so.6`; elsewhere neither may.
* `README.md` (state table row) and `docs/toolchain.md` section 8: host-built `std` programs run under libniji
  (what the test shows, shown with the prefixed loader only); linking `std` against libniji stays "not tried";
  `libgcc_s.so.1` comes from the GCC toolchain (`cc -print-file-name=libgcc_s.so.1`, Debian/Ubuntu `libgcc-s1`),
  not from libniji. Section 5's "What blocks `std`" paragraph (about linking) is unchanged.

## Probe-settled

* `gid()` makes `std` fork: `strace -f -e trace=clone,clone3,vfork,execve` of the program (pass-1 binary, host
  C library) shows `clone(... SIGCHLD)` with no `CLONE_VM|CLONE_VFORK`, then `execve` of the binary. The test
  itself cannot see which path `std` took (a `std` change back to posix_spawn would go unnoticed); not planted
  for that reason.
* No `std` feature failed under libniji: no limitation beyond the two stated (libgcc_s, linking untried).

## Results

* Try run `std-smoke-try1` (head 7f6f65e52, before lint fixes): pass 1 and native pass both PASS.
* Lint: `std-smoke-lint3` (clippy `--workspace --all-targets --all-features -D warnings` + `cargo xtask static`)
  rc 0 on 792090995. Earlier `std-smoke-lint`, `-lint2` red (3 clippy lints; fs-order), fixed.
* Gate `std-smoke-gate` (direct tier, `--base origin/next`, on 792090995): 8 suites, 628 passed, 0 failed;
  peak 2.0G, rc 0.
* Plants `std-smoke-plants` (`handoff/STD-SMOKE-plants.txt`, on 792090995): 5 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY
  or WARN-NO-MSG. `std-smoke-not-native` (the test expects the host's loader) is caught only by the native pass,
  so that pass really ran the program as a libniji process. Each plant also printed "caught by no test the log
  names" (the message matched; the tool did not find a test name in the `affected --only` log).

## Next step

None for this story. Possible follow-ups: a `std` program run with libniji installed at `/` (not the prefixed
loader); linking `std` against `libc.so.6`/`libc.a` (section 5).
