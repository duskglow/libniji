# TEST-REDUNDANCY-AUDIT: handoff

State: branch `task/test-redundancy-audit` from origin/next e3b0caf60; HEAD: see
handoff/reports/TEST-REDUNDANCY-AUDIT.json. Planning story: a report, no code, no test changed.
Report: plan/reviews/TEST-REDUNDANCY-AUDIT.md. Follow-up briefs: handoff/TEST-REDUNDANCY-AUDIT/*.md (8).

## Result
- High-confidence items (H1-H3): ~-100 s per full CI gate run (= per train) of a 1,505 s run: pass 2 -965
  test-s (~-91 s), pass 1 -140 test-s (~-9 s); 0 test binaries, ~20 lines. All items: ~-215 s, -70 binaries
  (of 321), ~-1,100 lines (estimates marked in the report).
- Largest single saving is a harness effect, not a redundancy: xtask `affected::tests` reload the tree index per
  nextest process (1,403 test-s vs 60.5 s wall / ~243 CPU-s in one process).

## Method and data (all on mei)
- T1: nextest PASS lines of five CI full gates (ci-gate-{ef44c6896,1c70414bd,b04f71229,3db4c4009,aacf0bce8}
  .affected.log), median per test. Did not run my own full `cargo xtask test` (brief step 1): five train runs
  give the same data with spread. Scratch scripts (parse.py, agg.py, items.py, counts.py, products.py,
  timings.py, failhist.py) were in the local scratch dir, not committed.
- T3: job `test-redundancy-audit-timings` (mei, role s3): cold `cargo test --workspace --no-run --timings`,
  440 s wall, 1,495 CPU-s; report at mei:~/work/test-redundancy-audit/cargo-timing.html. Its target dir
  ~/w/s3/target/audit-timings (~0.6 GB of test binaries plus deps) can be deleted by the role's next user.
- T4: job `test-redundancy-audit-affected-once` (mei): affected::tests in one process.
- T5: FAIL lines of 608 CI gates (since 10-21) and 90 developer gate logs on mei.

## Rulings needed (in the report)
- H1 PASS2-PRODUCTS-ONCE (gate coverage): recommend yes.
- H2/H3 NSCACHE-MODE-SCOPE: amend D-16.10's test clause to programs that reach the cache: recommend yes.
- M1 PASS2-KERNEL-SHARD0, M6 LD-OOM-SWEEP-STOP (coverage / repeat count): recommend yes after measuring.

## Not done / next step
- The coordinator submits the follow-up briefs (order in the report). Not costed: merging the generated
  per-section `sNN_*` binaries (a stubs generator change); casecmp_avx2 sweep size (needs plants first).
- Do not redo: the timing ranking (T1 is reproducible from the logs); the nscache reach check (done per program).
