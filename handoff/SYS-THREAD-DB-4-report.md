# SYS-THREAD-DB-4 report

Branch `task/sys-thread-db-4` (continues `task/sys-thread-db-3`), origin/next `b04f71229` merged in (the merge
base). Role keke:~/w/r1 (Zen 1 host). Code head 842d7dd3f; later commits are handoff/ only.

## What this run did

1. **Merged origin/next** (eebbf63bb). The CI conflict of run 3 (tools/deb/build.sh, system.sh, check-system.sh)
   is resolved by taking next's scripts as they are and shipping the library through PKG-MANIFEST's data: two rows
   in `tools/deb/manifest.tsv` (`libthread_db.so.1` in libc6/niji6, the `libthread_db.so` link in
   libc6-dev/niji6-dev). Other conflicts: crates/Cargo.toml (next's lists, without the removed `thread-db`),
   xtask/src/so.rs `LIBC_PRIVATE` (union of both sides).
2. **Docs** (74a13ba19): README phase-14 row, architecture crate table and limitations, unreleased release notes.
3. **Own red found by the first lint job and fixed** (842d7dd3f): next added the `libresolv.so.2` product, which
   links niji-boundary without the `runtime` feature, and `cargo xtask so` refuses writable data of libresolv's own.
   `#[used]` on `__niji_thread_db` kept the descriptor, and through it the thread list's statics (HEAD, LIVE,
   TLS_IMAGE), in every product. The attribute is now `#[cfg_attr(feature = "runtime", used)]`: libc.so.6 and
   libc.a keep (and export) it, the other products drop it by section gc. Two plants prove both directions.
   Comments naming the pending record now name D-22.17.

## Jobs (keke, head 842d7dd3f)

| label | what | result |
|---|---|---|
| sys-thread-db-4-lint, -plants | first start on eebbf63bb | SUPERSEDED: stopped while queued (docs commit followed) |
| sys-thread-db-4-lint2 | lint + products on 74a13ba19 | SUPERSEDED: rc 1, `cargo xtask so` refused libresolv.so.2's writable data (the red fixed in 842d7dd3f) |
| sys-thread-db-4-plants2 | plants on 74a13ba19 | SUPERSEDED: rc 2, baseline build failed for the same reason |
| sys-thread-db-4-lint3 | `cargo xtask clippy`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`, `cargo xtask so`, `manifest.py lint --products target/niji`, `deb_manifest` test, gdb proof `thread_db_products` | rc 0; manifest OK (141 rows); deb_manifest 5 passed; gdb proof 1 passed |
| sys-thread-db-4-deb | both flavours built from the role (`NIJI_DEB_COMMIT`), `tools/deb/check.sh`, `tools/deb/check-system.sh` | rc 0: coexist `check: PASS (2 packages)`; system `check: PASS (7 packages, version 2.39+niji20251123.1-1)`; libthread_db.so.1 (13840 bytes) in libc6 and niji6, the .so link in libc6-dev and niji6-dev |
| sys-thread-db-4-plants3 | `cargo xtask plants handoff/SYS-THREAD-DB-4-plants.txt` | rc 0: 8 CAUGHT, 0 SURVIVED, no WARN-RC-ONLY / WARN-NO-MSG; one WARN-DEV-SHM (copy under /dev/shm, as the brief directs for this host class; no plant here mounts /dev/shm) |
| sys-thread-db-4-gate | direct tier `cargo xtask affected --run` | PENDING at hand-back (06:05Z): clippy, products, xtask tests, niji (3 modes), niji-boundary (3 modes), niji-ld, niji-legacy, niji-math all `ok`; still running; the dispatcher verifies its rc |
| sys-thread-db-4-startrows | start-up rows (below) | PENDING (queued after the gate in the chain; `cargo xtask so` then the rows script) |
| sys-thread-db-4-startcheck | `cargo xtask test`, FULL-profile so, hot_text_products | PENDING (queued after the start-up rows) |

## Start-up rows (ruling #6168)

Pending: `~/logs/sys-thread-db-4-startrows.log` on keke (`ROW` lines: static and dynamic empty main, base
b04f71229 vs this head, lowest instructions:u and minor faults of 21 runs, plus libc.so.6's RELATIVE count). The
script is keke:~/work/SYS-THREAD-DB-4/startup_rows.sh (run 3's, unchanged but for the header). Run 3 measured
+3 RELATIVE relocations in libc.so.6 and +6 instructions static / +0.03% dynamic, no fault row moved; this run's
code change only removes the descriptor from the non-libc products, so libc.so.6's rows are expected unchanged.
Chain: keke:~/work/SYS-THREAD-DB-4/chain.sh (lint3 -> deb -> gate -> startrows -> startcheck, one build at a time).

## RULING NEEDED / INFORMATIONAL

* INFORMATIONAL: dynamic programs pending on DL-23 (ruling #6166), unchanged.
* INFORMATIONAL: `docs/codemap/README.md` (generated) still lists the removed `thread-db` crate; left to the
  train's view regeneration.
* INFORMATIONAL: `tools/deb/parity.sh` not rerun.
