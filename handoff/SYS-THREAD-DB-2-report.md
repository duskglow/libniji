# SYS-THREAD-DB-2 report

Branch `task/sys-thread-db-2`, base origin/next `ebb670553`. Role keke:~/w/r1.

## Commits (this run)

1. Remove the niji-thread-db stub crate; fix the descriptor's const widening (inherited head did not compile:
   E0658, `u64::from` in a const fn).
2. Export `__niji_thread_db` from data.rs (unsafe audit: `UNSAFE(export)` only in the export files); add the gdb
   proof `thread_db_products.rs` + `tests/oracle/thread_db_target.c`.
3. Ship libthread_db.so.1 in the system flavour; pending decision record; plants file.
4. Handoff and report.

## Jobs (keke)

| label | what | result |
|---|---|---|
| sys-thread-db-2-lint0 | clippy+static+so, inherited head | SUPERSEDED: rc 101, E0658 (fixed in commit 1) |
| sys-thread-db-2-lint1 | same, commit 1 | SUPERSEDED: rc 1, unsafe-audit (export outside export files; fixed in commit 2) |
| sys-thread-db-2-lint2 | static, so, thread_db_products, clippy (commit 2) | rc 0; the gdb proof passed |
| sys-thread-db-2-gate | direct tier `cargo xtask affected --run`, final head | queued at hand-back |
| sys-thread-db-2-plants | `cargo xtask plants handoff/SYS-THREAD-DB-2-plants.txt` (6 plants) | queued at hand-back |
| sys-thread-db-2-startrows | start-up rows before/after (#6168): empty main, static and dynamic, instructions:u and minor faults (min of 21), base products built from ebb670553 in RAM | queued (runs after the gate) |
| sys-thread-db-2-startcheck | `cargo xtask test` + FULL-profile so + hot_text_products | queued (runs after startrows) |

Script for the start-up rows: ~/work/SYS-THREAD-DB-2/startup_rows.sh on keke (prints `ROW static-base/after`,
`ROW dynamic-base/after` and the RELATIVE relocation count of each libc.so.6); read `~/logs/sys-thread-db-2-startrows.log`.

## Probe-settled behaviour

gdb 15.1 on keke, static libniji program, products' libthread_db: "Thread debugging using libthread_db enabled",
four `Thread 0x... (LWP n)` rows, per-thread backtraces reaching `niji_tdb_worker` / `niji_tdb_stop` / `main`,
`tv` = 7, 111, 122, 133 (TLS through `td_thr_tlsbase(th, 1)`); the same from a gdb-written core file.

## RULING NEEDED / INFORMATIONAL

* INFORMATIONAL: dynamic programs pending on DL-23 (ruling #6166); recorded in the decision record and handoff.
* INFORMATIONAL: `tools/deb/parity.sh` not rerun this run (budget); next step in the handoff.
* Process: one local python heredoc was used for a plan-text edit (harmless, not refused); noted for honesty.
