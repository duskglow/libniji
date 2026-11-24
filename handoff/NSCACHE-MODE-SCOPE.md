# NSCACHE-MODE-SCOPE: the name-service cache's two test modes only where a program reaches it

State: branch `task/nscache-mode-scope`, code commit `ff685180c` (handoff and report commits on top).

## What was done

- **D-16.18** (`records/decisions/D-16/D-16.18.toml`, `amends = ["D-16.10"]`; owner ruling 2026-11-23 on
  TEST-REDUNDANCY-AUDIT H2, H3): D-16.10's test clause now covers only differential programs that reach the cache.
- `aio_products.rs`: `aio_equals_the_reference_uring_on` and `_eperm_on` dropped; `uring_off`, `eperm_off`,
  `enosys_off` and `allow_off` stay (the brief's letter).
- catalogs, syslog, fstree, posixipc, mntent, mallocdbg `_products.rs`: the `for mode in ["on", "off"]` loop is
  one run with `let mode = "on";` (the body is unchanged, dedented). "On" was chosen because it is the configuration
  users run, and a lookup reached through a path the static check does not see still goes through the cache.
- `crates/core/tests/nscache_reach.rs` (the ruling's condition), 5 tests:
  `the_cache_has_only_the_known_callers` (the files under `crates/*/src` that name `nscache::`/`NSCACHE` are a
  pinned list, each tied to a family or to why it reads no switch); `every_lookup_family_name_is_a_library_symbol`
  (each family name or prefix matches `spec/symbols.tsv`); `the_one_mode_tests_are_the_known_programs` (a
  `*_products.rs` that uses `nscache_mode.sh` without both `"on"` and `"off"` in code must be in ONE_MODE);
  `no_one_mode_program_makes_a_lookup` (each one-mode `oracle/<name>_diff.c` and its local `#include "..."` headers
  name no family function outside comments and literals); `the_scan_sees_calls_and_skips_comments_and_literals`.
- Plants: `handoff/NSCACHE-MODE-SCOPE-plants.txt` (10; the first is the brief's red-before-green: a getpwnam call
  added to aio_diff.c).

## Limits (stated in the test's doc and D-16.18)

- The families are hand-listed and wider than the direct callers (glob/wordexp tilde, grantpt, cuserid reach the
  cache through another module). A transitive route through a module that does not name the cache is not derived
  automatically; a new direct caller is caught by the pinned list.
- `cargo xtask affected` selects `nscache_reach` when the test changes, not when a program under `oracle/` changes
  (the test builds the path with `format!`). The CI train's FULL gate runs it on every landing.

## Proof still owed

- Saving: measure from the next CI gates' nextest lines (audit T1 method): aio_products pass 1 expected
  286 -> ~191 test-s, the six ~-45 test-s. Not measured in this run.

## Next step

Land via the train; afterwards read the train's per-test times for the seven tests and record the measured saving
(impact entry) against the audit's estimate.

Plan rows to read: D-16.10, D-16.18, plan/reviews/TEST-REDUNDANCY-AUDIT.md (H2, H3).
