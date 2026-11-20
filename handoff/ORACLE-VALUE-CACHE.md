# ORACLE-VALUE-CACHE: the math oracle's decided values kept across libniji changes (D-10.131)

Branch `task/oracle-value-cache` (from origin/next 08696f1a7, which has MATH-ORACLE-PROFILE and MP-ISQRT-FAST).
Host keke (Zen 1, 60 vCPU), role n4. Decision in force: D-10.131 (amends D-10.57). Plants:
`handoff/ORACLE-VALUE-CACHE-plants.txt`. Report: `handoff/reports/ORACLE-VALUE-CACHE.json`.

## Premise check (brief: stop if the oracle is no longer the dominant cost)

After MP-ISQRT-FAST the oracle was still ~86 % of a complex_f128 shard (its handoff) and the five sweeps 9549 keke
test-s. Measured here, the oracle's share of the five sweeps is 98 % (cold 9586 test-s against 199 test-s with every
value served), so the cache was built. It also rests on CI keeping `target/` between trains on a role, the same
dependency as the verdict cache (D-10.57, still unconfirmed by the coordinator).

## What changed (test code only; no product, no oracle code)

- `crates/math/tests/support/oracle_values.rs`: `Values` (open, get, save; `get_shared` for the special sweeps'
  8 checking threads), `oracle_key`, `tally_value`, `Query`, `enc_cr`/`dec_cr`. Wired into complex_ld_cr,
  complex_f128, complex, special_ld and special_f128 products: every oracle call (`cr`, and the random lines'
  tally value) goes through the shard's cache. libniji's and the reference's results are read from the programs'
  outputs every run; the line checks are unchanged.
- Key: the bytes of every workspace crate in niji-mathtools' Cargo.lock closure (today only crates/mathtools), name,
  version, source and checksum of any registry package in it (none today), the sweep's own source file, the cache's
  source, root Cargo.toml (profiles), rust-toolchain.toml, .cargo/config.toml, `rustc -vV`, compile-time and run-time
  RUSTFLAGS / CARGO_PROFILE_* / CARGO_BUILD_* / CARGO_TARGET_* (not _DIR/_TMPDIR), target and CPU flags, format
  version. Unreadable input: no cache. Query: kind, function, mode, part, operand encodings.
- Canary: 2 entries per kind per file recomputed on load; any difference discards the file (message
  `oracle values <dir>: canary ... the file is discarded`).
- Decided values only (finding #6890): `cr` results with flags and exact-zero bit; never an undecided one. The tally's
  reference value is now the true value rounded to nearest at prec + 40 bits, decided by Ziv from prec + 64 bits
  (error < 2^-40 ulp; the old one was a fixed-precision approximation at 2 prec + 40 whose last bits depended on call
  history). Undecided below 16 (prec + 40) bits: the approximation is used and not stored.
- Storage: `target/test-oracles/values/<crate>.s<k>/<key id>`, written to `.<id>.<pid>.<ns>.tmp` and renamed
  (concurrent writers each leave a complete file; last rename wins), `end <n>` trailer (short file not read). A save
  keeps only the values the shard asked for in that run and removes the shard's files of other keys. Size: 55 MB for
  all five sweeps (690 304 values), one key per tree. `NIJI_NO_VERDICT_CACHE=1` (nightly/release) or
  `NIJI_NO_ORACLE_VALUES=1`: everything computed, nothing written (CONTRIBUTING.md updated).

## Measurement (keke n4, same role and build, `cargo nextest ... --test-threads 24` under `~/bin/gate 16G`)

Job `oracle-value-cache-measure` (`~/work/oracle-value-cache/measure.sh`, logs in `~/work/oracle-value-cache/measure/`,
summary `sh ~/work/oracle-value-cache/ts2.sh <log>`). Each run first removes the sweeps' verdict records so every shard
runs (NIJI_NO_VERDICT_CACHE=1 would also turn the value cache off). Test-seconds, each test counted once:

| binary | cold | warm | warm after a libniji math change |
|---|---|---|---|
| complex_ld_cr_products (21) | 4531.3 | 114.3 | 112.2 |
| complex_f128_products (8) | 1376.7 | 25.0 | 26.0 |
| special_ld_products (9) | 1705.6 | 6.8 | 8.0 |
| special_f128_products (8) | 1593.1 | 6.0 | 6.1 |
| complex_products (8) | 379.4 | 46.6 | 45.5 |
| total | 9586.1 | 198.7 (-97.9 %) | 197.9 |
| nextest wall | 852.0 s | 14.4 s | 14.2 s |
| values | 690 304 computed | 690 304 served | 690 216 served, 1 870 computed |

The change: libniji's `csqrtl` real part scaled by 1 - 2^-63 (C adapter, scratch, restored with a fresh mtime and
`cargo xtask so`); every complex_ld_cr shard failed on the changed lines (20 failed), checked against oracle values
of which 1 870 were new. Cold equals MP-ISQRT-FAST's after column (9549) within noise: the new tally value did not
make a cold run slower. What remains warm is parsing, the programs' shared outputs and the checks (complex_ld_cr
5.4 s a shard).

## Results

Commits: c5dcc2480 (the cache, wiring, tests, D-10.131, CONTRIBUTING), 27a586f0e (FS-ORDER note for static),
47d1fdea5 (test: a file under another key's name is not read), then this handoff.

- Final head (47d1fdea5 code): `oracle-value-cache-lint-6` (clippy workspace all targets all features -D warnings,
  `cargo xtask static`, `records check`): rc 0. `oracle-value-cache-gate-2` (direct tier) and
  `oracle-value-cache-plants-2` (`cargo xtask plants handoff/ORACLE-VALUE-CACHE-plants.txt`, 12 plants) are the
  final-head jobs; see the hand-back for their state.
- `oracle-value-cache-gate` (head 27a586f0e, which differs from the final head only in verdict_cache.rs): rc 0,
  6 suites ok, 736 passed, 0 failed, 0 SKIPPED lines, peak 1.7G; test -p niji-math 1312 s (cold values: the key
  had changed since the measurement).
- `oracle-value-cache-plants` (27a586f0e): 12 plants, 11 CAUGHT with their messages, 1 SURVIVED: `other-key-read`
  (no check of the key named in a file's header). Files are named by key id, so no test reached the check; fixed in
  47d1fdea5 (the prune test renames a key-one file to the key-two name). `libniji-warm` (libniji's quick `csqrt`
  parts swapped) was caught with a warm cache (`complex_products.sN: <n> cached` in its log); `oracle-changed`
  (the oracle's `csqrt` parts swapped) missed the cache and was caught as `not correctly rounded`.
- Superseded: `-lint`, `-lint-2`, `-lint-3` (clippy fixes), `-lint-4` (fs-order), `-gate` and `-plants` (head
  27a586f0e).
- Mistakes of this run: one ssh start chained `test -s && setsid ... &` and held the ssh 10 minutes (the job ran
  detached and was unaffected); one wait call chained two jobwaits past the tool's 600 s limit.

## Open / next

- INFORMATIONAL: the gain depends on CI keeping `target/` (the cache lives in it) between trains; with a fresh
  target every run is cold (correct, no gain).
- INFORMATIONAL: whether Ziv resolves before its cap can depend on call history (#6890); a warm run serves a value
  decided in an earlier process where a fresh process could, in principle, report "oracle undecided". No such line
  exists in today's sweeps (no undecided errors cold or warm).
- Next: the coordinator lands this; nothing else is owed by this story.
