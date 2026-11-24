# SYS-THREAD-DB-3 report

Branch `task/sys-thread-db-3` (continues `task/sys-thread-db-2`), merge base origin/next `ebb670553`. Role mei:~/w/s1.
Code head 5d01b1463 (later commits: handoff/ only).

## The inherited red (SYS-THREAD-DB-2's gate2)

`xtask affected::tests::a_loader_change_does_not_select_the_math_suites`: a change to crates/ld/src/lib.rs selected
25 niji-math suites. Whose: the test passes on the merge base `ebb670553` (job sys-thread-db-3-selbase, a tree from
`git archive` of the base) and fails on the inherited head ca10bcf0d (sys-thread-db-3-lint0): this branch's.
Cause: SYS-THREAD-DB-2 put `const _: () = assert!(niji_rt::thread::DEBUG_DTV_HEADER == niji_ld::tls::DTV_HEADER);`
in `crates/boundary/src/data.rs`. data.rs is compiled by every test build of niji-boundary, including niji-math's
`selftest` build, so the selector's onward edge boundary -> niji-ld became real for niji-math. The edge is not right
(niji-math's tests never run loader code), so the test's expectation stands.
Fix (commit 5d01b1463): the assertion moved to `crates/boundary/src/bootstrap.rs` (compiled only with the `runtime`
or `ld` feature, i.e. the libc and ld.so products, and already a niji-ld user). The check still runs in both
product builds; the test builds gain no loader edge.

## Jobs (mei, final code head 5d01b1463)

| label | what | result |
|---|---|---|
| sys-thread-db-3-lint0 | inherited head: the selector test, `cargo xtask clippy`, `cargo xtask static` | SUPERSEDED (diagnosis): test FAILED as reported; clippy and static ok |
| sys-thread-db-3-selbase | the selector test on the merge base | SUPERSEDED (diagnosis): ok |
| sys-thread-db-3-lint1 | selector test, all 456 xtask unit tests, `cargo xtask clippy`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`, `cargo xtask so`, the gdb proof `thread_db_products` | rc 0; proof ran (no skip note) |
| sys-thread-db-3-gate | direct tier `cargo xtask affected --run` | rc 0: 30 suites, 12051 passed, 0 failed, 1397 ignored; conformance 137/137; peak 16G (page cache), oom_kill=0 |
| sys-thread-db-3-plants | `cargo xtask plants handoff/SYS-THREAD-DB-2-plants.txt` | rc 0: 6 CAUGHT, 0 SURVIVED, no WARN (results: handoff/SYS-THREAD-DB-3-plants.txt) |
| sys-thread-db-3-startcheck | `cargo xtask test` + FULL-profile so + hot_text_products | rc 0 (layout contracts hold; hot_text_products 26 passed) |
| sys-thread-db-3-startrows | start-up rows | SUPERSEDED: rc 127, script bug (below) |
| sys-thread-db-3-startrows2 | same, traced | SUPERSEDED: rc 127, same bug, cause found |
| sys-thread-db-3-startrows3 | start-up rows, fixed script | rc 0, rows below |

## Start-up rows (ruling #6168)

Empty `main`, linked statically and dynamically against the merge base's products and this head's (default
`cargo xtask so` profile), lowest of 21 runs, `perf stat` instructions:u and minor-faults (Skylake-SP host):

| row | base ebb670553 | this head | change |
|---|---|---|---|
| static, instructions:u | 2202 | 2208 | +6 |
| static, minor faults | 9 | 9 | 0 |
| dynamic, instructions:u | 8969715 | 8972380 | +2665 (+0.03%) |
| dynamic, minor faults | 88 | 88 | 0 |
| libc.so.6 R_X86_64_RELATIVE | 2572 | 2575 | +3 (`__niji_thread_db`'s three pointers) |

No fault row moved and the layout contracts hold (startcheck). The static row is a non-PIE static link
(0 RELATIVE relocations in either).
Script: mei:~/work/SYS-THREAD-DB-3/startup_rows.sh. The inherited version (from run 2, never run before) passed
`LD_LIBRARY_PATH=<products>` to `perf` itself, so the system's loader loaded libniji's libc.so.6 into perf and stopped
(rc 127). Now perf runs under the system loader with `--library-path` of the system libdirs, and only the measured
program sees the products' path. libniji's loader takes `libc.so.6` only from LD_LIBRARY_PATH or its own libdir
(RUNPATH/RPATH do not apply to that name, R14-3), so an rpath link was not an option.

## Probe-settled behaviour

As in run 2: gdb 15.1 (now on mei), static libniji program, products' libthread_db: threads, per-thread backtraces,
TLS values 7/111/122/133, live and from a core file.

## RULING NEEDED / INFORMATIONAL

* INFORMATIONAL: dynamic programs pending on DL-23 (ruling #6166), unchanged.
* INFORMATIONAL: `tools/deb/parity.sh` not rerun (next step in the handoff).
* INFORMATIONAL: the dynamic empty-main row is about 9.0 million instructions on the base and on this head
  (unchanged by this branch); it may be worth a start-up look by whoever owns the loader rows.
* Process: one empty local heredoc (`python3 - <<'X'`, no content) slipped into a scratch command; harmless, noted.
