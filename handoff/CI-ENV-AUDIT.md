# CI-ENV-AUDIT: handoff

State: branch `task/ci-env-audit` (from origin/next 0495c4684). Audit only: one results directory,
no code, no CI change. Report: `results/ci-env-audit/README.md` (inventory, classification, the
reds, workspace measurements, traces, manifest specification, ranked proposals).

## What was done

* 300 CI gate logs (11-13 to 11-23) grouped by failure signature: 49 red; the last 20 non-story reds
  classified (H host 3, W workspace 4, B start-up bound 3-4, R recurring-unsettled 11).
* Host manifest prototype (`results/ci-env-audit/manifest.sh`) run on mei, keke, shiki, kanon and
  diffed: mei alone has `locales-all` (2.39-0ubuntu8.12 beside libc6 8.9; 511 locales vs 4), kernel
  149 vs 147, AVX-512; `cargo-nextest` and `~/.cargo/config.toml` differ across hosts.
* CI workspace on mei (`/mnt/zram/claude/w/ci-z/target`, 17 GB): 631 leftover per-process test
  directories (2.4 GB, since 11-10), pre-rename `liella*` products, `test-locales` never rebuilt,
  oracle stamps without host inputs.
* Cost on keke n1 (4-core slot, tmpfs target): cold 1179 s, no-op 303 s, products-only removal
  302 s (the 11-23 CI removal is free), one-file touch 926 s; target 1.44 GB.
* strace of `xtask so` and of the red product suites: job `ci-env-audit-trace` (keke); summary
  in the README's "Traces" section.

## Not to redo

The red classification (README tables) and the cost runs; logs `~/logs/ci-env-audit-measure2.log`
and `~/work/ci-env-audit/m2-*.out` on keke (the first run `ci-env-audit-measure` measured nothing
useful: `du` of the target symlink and an `--all-features` test build that cannot link).

## Open questions / rulings

* Proposal 6 (start-up bounds judged against the same gate's base build) changes how bounds gate:
  RULING NEEDED (recommend yes: the absolute bound stays; a red whose base is equally over is a
  base red).
* Proposals 2 and 8 need root on the CI host (package holds; a pinned root image): owner's.
* The R reds (heap_meta 5, dns_products 3, aio_engine 3) were not settled: next or host.

## Next step

Coordinator: build proposals 1 (workspace allow-list) and 3 (host drift manifest) in liella-ops;
file a story for proposal 4 (host-keyed oracle and test-locale caches, `crates/core/tests/support/
oracle.rs`) and one for the test scratch that `runpath_search`, `object_names` and `search_cache`
leave under `target/` on every run.
