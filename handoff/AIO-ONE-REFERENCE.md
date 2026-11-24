# AIO-ONE-REFERENCE (TEST-REDUNDANCY-AUDIT M2): aio_products runs the reference once

State: branch `task/aio-one-reference`, code commit c88bb26e7 on base 9139a21d9 (origin/next). Test-only change: no
release-note fragment, no decision row (a local test-structure detail; no comparison removed, so no ruling).

## Premise check

The brief said six variants and 286 test-s per pass. NSCACHE-MODE-SCOPE (H2) had already dropped the two "on"
variants: four tests on the base, 50.3 + 51.2 + 51.4 + 51.1 = **204.0 test-s** in pass 1 of CI gate 9139a21d9
(`ci-gate-9139a21d9-*.affected.log`, the audit's T1 method, one gate). aio_products runs in pass 1 only
(xtask/src/nextest.rs, products_once).

## The split (measured before any change)

One `cargo test -p niji --test aio_products -- --test-threads 4` on the Skylake-SP build host (load ~13), job
`aio-one-reference-split`, with temporary timing prints (not committed). Every variant gave the same numbers:

| phase | seconds | share |
|---|---|---|
| builds (aio_diff.c both sides, aio_noring.c; cached by build_pair) | 0.72 | 2 % |
| reference run | 25.3 | 55 % |
| libniji run | 19.7 | 43 % |

The reference is the larger share, so the change went ahead.

## The change

- `crates/core/tests/aio_products.rs`: one test, `aio_equals_the_reference`, builds once, runs the reference once,
  then compares each entry of `CONFIGS` (uring-off; eperm-off, enosys-off, allow-off under aio_noring) in turn.
  The old per-test body is `compare()`, unchanged except that it takes the reference output. Each configuration
  runs inside `catch_unwind`: a failure (a difference, a section alarm, a recorded line no longer printed by the
  `without` helper, a failed run) prints `configuration <tag> (<engine>; name-service cache <mode>) FAILED: ...`
  and the rest are still compared; the test then fails with `N of 4 configurations failed: <names>`.
- `.config/nextest.toml`: the `privileged-aio` group (max 3 of the old tests at once) is removed with its
  override; the test keeps its start-first priority (the privileged-chain priority override is unchanged).

## Results

- After: one test, **105.6 s** (job `aio-one-reference-lint`, same host, `--nocapture`). That is ~0.7 + 25.3 + 4 x 19.7,
  as the split predicts. Saving ~98 test-s in pass 1 (204.0 -> ~106), measured on a role, not yet in a CI gate.
  The wall time of the aio chain stays about the same: before, 4 tests at most 3 at once = two rounds of ~51 s.
- Gate `aio-one-reference-gate` (direct tier, `--base origin/next`): 2 suites, 10 passed, 0 failed, peak 1.7G.
  The selection is bounded (aio_products and special_ld_products, which reads nextest.toml); `affected` prints
  "FULL: the train's" for nextest.toml (not an escalation).
- Plants `aio-one-reference-plants` (`handoff/AIO-ONE-REFERENCE-plants.txt`): 3 CAUGHT, 0 SURVIVED, 0 WARN, each
  by its `msg:`.
  enosys-only and allow-only put an extra stdout line in one configuration's run only; each must fail with
  `1 of 4 configurations failed: <that configuration>`. known-not-marked breaks the per-configuration
  recorded-difference check: all four must fail, all named.

## Next step

Confirm the saving from the first CI gates after landing (the T1 method: the `niji::aio_products
aio_equals_the_reference` PASS line against 204.0 test-s). Possible follow-up, not done here and not measured: run the
four libniji configurations at the same time (each already has its own private /dev/shm and scratch). That would cut
the test to ~26 s wall, but nextest would not see the extra processes. That is a load-profile choice for the coordinator.

## Do not redo

The split (above). Do not reintroduce per-configuration tests to get parallelism; use threads inside the one test if
the follow-up is wanted.
