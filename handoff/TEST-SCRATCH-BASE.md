# TEST-SCRATCH-BASE

State: branch task/test-scratch-base. Test scratch paths create their base.

## Cause (evidence)
The CI train removes the tests' scratch before the run: liella-ops `ops/ci.py` ~line 1882 runs
`rm -rf target/test-scratch target/tmp` in the CI role's gate command, and `tools/sync_role.sh` ~line 111 does the same on
every sync. Cargo creates `target/tmp` only when it builds a test target, so with test binaries already built (a cached
role) nothing recreates it, and `Scratch::path` (which never created its base) wrote into a missing directory
(`NotFound` at aio_engine.rs:175). Tests using `Scratch::dir` passed because it does `create_dir_all`.
Neither file is in this repo (liella-ops is read-only for this run); the removal is deliberate cleanup, so the tests are
the right place to fix it.

## Changes
- `Scratch::path_in` (so `path`, `dir`, `sock_dir`) calls `create_dir_all(base)`, panics with the base in the message.
- `support/sync_trap.rs` and `skip_note_visible.rs` create `CARGO_TARGET_TMPDIR` before writing a file there.
- New test `crates/core/tests/scratch_base.rs` (`path_in_makes_a_missing_base`); plant `drop-mkdir` CAUGHT
  (handoff/TEST-SCRATCH-BASE-plants.txt).

## Not done / open
Other test files build `CARGO_TARGET_TMPDIR`-relative paths inline (about 40 sites, git grep CARGO_TARGET_TMPDIR);
those that write a plain file or directory without `create_dir_all` (e.g. s12_stdio_clauses.rs:107, native_v1.rs:533,
appendix_a_primitives.rs:3435) still assume the base. A follow-up could move them to `Scratch` or add one
`target_tmp()` helper. Alternative: make ci.py/sync_role.sh `mkdir -p target/tmp` after the `rm -rf`.

Next step: land; optionally the follow-up above.
