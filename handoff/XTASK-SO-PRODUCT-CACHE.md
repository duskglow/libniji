# XTASK-SO-PRODUCT-CACHE: `cargo xtask so` restores unchanged product sets and traces

Branch `task/xtask-so-product-cache`; head: see `handoff/reports/XTASK-SO-PRODUCT-CACHE.json`. Decision D-X.181.

## What landed

- `xtask/src/so_cache.rs`, two content-keyed caches (sha256 only, never a modification time):
  - **set** (`target/so-cache/sets/<key>`, 3 kept): all of `target/niji` (except the other commands' `bench`,
    `native`, `rust`, `rust-test`) plus the `target/product` files later commands read (the lists the products'
    marker names, `layout-bins.tsv`, trace logs, `placeholders/`). Key: every tree file except `target/`,
    `fuzz/target/` and `.git`; the headers already staged (so the host's kernel UAPI copies count); the trace probes
    and `page_faults.c` run through `cc -E` (system headers); the xtask binary and tools by content
    (`layout_verdict::host_inputs` plus ar, objcopy, strip, gdb, gzip, python3, bash, rustc, cargo, ...) and by
    version (`import::toolchain`); the environment (`NIJI_*`, `CARGO*`, `RUST*`, PATH, CC, AR, LANG/LC_*, ...);
    profile and layout; the canonical root. On a hit, every copy is checked against the stored sha256, the
    `.profile` marker goes in last and the manifest is written fresh. If anything fails to verify, the cache is
    treated as a miss and the products are cleared and rebuilt in full.
  - **trace** (`target/so-cache/traces/<key>`, 4 kept): the lists and `trace.log` of a trace-at-build. Key: the
    `hot-text.pre-group.a` archive by content plus the same host lines, and the tree without `crates/**/*.rs` (the
    trace reads no Rust source; it reaches the trace only through the archive).
  - Both are off under `NIJI_TRAIN` (like the layout verdict cache) and with `NIJI_SO_CACHE=0`.
- `so.rs`: headers are now staged before the early threads start (so the key can take them); the set is tried right
  after; it is stored after the manifest; `trace_layout` asks the trace cache first.
- `bench.rs`: `step: bench/build`, `step: bench/counters` and `step: bench/rounds` lines.

## Measured (keke, role n1, test profile, traced layout; plants running at the same time for part of it)

`step: so/total`, seconds. Before = origin/next 6d0c3e0b6 (job `xtask-so-product-cache-base`) and this branch with
`NIJI_SO_CACHE=0` (the old behaviour). After = this branch (jobs `xtask-so-product-cache-measure`,
`xtask-so-product-cache-mutants`).

| build | before | after |
|---|---|---|
| no-op | 328.6, 332.0, 336.0 (median 332.0) | 2.4, 2.4, 2.3 (median 2.4; set hit, 1691 products restored and checked) |
| cold (no stored set) | 336.0 (cache off) | 343.2 (key 1.8 s + store 0.1 s + trace key; plants were running) |
| test-only one-file mutant (`crates/core/tests/s29_legacy_clauses.rs`) | 331.3, 335.3, 337.0 (median 335.3) | 91.2, 93.3, 91.5 (median 91.5; set miss, trace hit) |
| library one-file mutant (comment line at the top of `crates/core/src/lib.rs`: every line moves) | not measured (stopped to leave time for the final gate; the cache cannot shorten it) | 604.8 (set miss, trace miss: the archive changed) |

Byte-identical: the sha256 of every file under `target/niji` (1687 files, the manifest aside) is the same for the
cache-off build, the cold build that stored the set, three warm hits and the hit after two mutants (`sums-*.txt`
in the measure job's directory). The test-only mutant changes exactly one product, `libm.so.6.source-digest` (a
digest of the sources), which is expected.

Gate builds run with a reset environment (`~/bin/gate`: `--reset-env`, its own PATH), so they do not share entries
with builds outside the gate (PATH is a key input). The gate's own product build was a miss and stored its set; a
second gate on the same tree hits.

## Decisions in force / open

- The train (`NIJI_TRAIN`) does not use the cache: RULING NEEDED (recommendation: keep it off; the train is the trust
  anchor, as for the layout verdict cache).
- A hit does not re-run the layout, CET or placement checks: the stored set passed them when it was built, under
  the same key.
- A set miss costs about 1.6-1.8 s for the key and about 0.1 s to store; the trace key costs a similar share inside
  libc-archive.

## Do not redo

- Per-product keys for library mutants: challenge #8851 (accepted). Almost every product takes libc.a, libc.so.6 or
  the niji-boundary cargo build as input, so a library mutant has to rebuild them anyway.

## Next step

- Owner/coordinator: the train ruling. Possible follow-up: excluding `docs/`, `handoff/`, `plan/` from the set key
  would let doc-only edits hit. It is not done here because nothing yet proves the build never reads them.
