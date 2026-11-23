# PROBE-NIJI-TREE: the probe client compares against the tree it is told

State: COMPLETE. Branch task/probe-niji-tree, code commit 4e4ca7db2 (this note and the report json follow it).

## What changed
- `tools/probe/probe`: `--niji` has no default. A probe or `--selftest` without it exits 2 with
  `probe: --niji <tree> is required: the libniji tree to compare with, usually your own role tree (--niji ~/w/<role>)`,
  checked in `main()` before the source is read or anything is built. `--find`, `--show` and `--record` need no tree.
  How products are found or rebuilt is unchanged.
- The result names what it measured: text line `niji: <tree> @ <commit>   products <sha256>` (the cache line moved
  to its own line); `--json` gains `tree` and `products_hash`; `meta.json` stores `products_hash`, so `--show` prints it.
- `tools/probe/remote.sh` usage/comments and `tools/probe/README.md`: the option is required, and why (one sentence);
  the `~/w/nightly` notes are gone.
- New test `xtask/tests/probe_client.rs` (no spec clause, probe tooling): without `--niji` -> exit 2 + message (a
  file probe, `--json`, `--selftest`); with `--niji <absent tree>` the given tree is used. HOME is a scratch dir, so
  the test reaches no tree, cache or `~/bin/shared` on the host.

## Callers (grep for `probe ` invocations, `bin/probe`, `remote.sh`, `w/nightly`)
No script in the repository invokes the client with a fixed argument list: `remote.sh` passes the user's options
through (its usage now shows `--niji`). The other hits are prose (spec/behaviour/ctype-words.md, key-nomem.md,
conformance/c/s07_langinfo_items.c, validation/README.md) or handoff history. No xtask code calls the client.
Outside the repo (read-only for me): `~/src/liella-ops` has no invocation of `~/bin/probe`; its backlog files mention
`w/nightly` (coordinator's).

## Proof (keke, role n3)
- `probe-niji-tree-proof`: without `--niji` rc 2 with the message; `--niji ~/w/n3` on `tools/probe/langinfo_items.c`
  rebuilt the stale products, ran, and printed `niji: ~/w/n3 (printed expanded) @ products-610cfe34   products 610cfe34d3b7...93aa6f`;
  `--json` gave the same tree and hash; an independent sha256 over the four products gave the same hash.
  (That probe's verdict is DIFFERENT: it is the langinfo item-dump probe, not part of this story.)
- `probe-niji-tree-lint`: cargo xtask clippy, cargo xtask static, cargo test -p xtask --test probe_client: rc 0.
- `probe-niji-tree-plants` (cargo xtask plants, handoff/PROBE-NIJI-TREE-plants.txt): 2 plants, 2 CAUGHT
  (default restored; check dropped), no WARN lines.
- `probe-niji-tree-gate` (affected, direct tier): 4 suites, 467 passed, 0 failed, peak 1.3G. It ran on the code commit;
  the later commit only adds this note and the report json under handoff/ (docs-only to the selection).

## Open / next
- The installed `~/bin/probe` on keke is still the old client until someone runs `tools/probe/remote.sh --install`
  after landing (not done here: it changes a shared tool for every agent before the branch lands).
- `cargo xtask affected` maps `tools/probe/probe` to docs-only, so a change to the client alone selects no test;
  mapping it to `xtask/tests/probe_client.rs` would gate client changes (the test `docs_only_runs_no_tests` lists
  that path and would change with it).
- `~/w/nightly` left untouched on every host (coordinator's).
