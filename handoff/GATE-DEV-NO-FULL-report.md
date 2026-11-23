# GATE-DEV-NO-FULL report: item 5, the escalated developer gates of 11-16..11-19 against the bounded selection

Method. The build hosts' gate logs (`~/logs/*gate*.log`, CI's `ci-gate-*` left out) that printed `FULL (escalated`
since 11-16: 40+ on keke, 30+ on mei. Twelve were taken across the classes. Each log lists its changed files
(`changed files: N`), so no other branch's history was read. Each list was replayed on this branch's tree with
`cargo xtask affected --list --tier direct --files <list>` (job `gate-dev-no-full-sel`, keke). Every one now prints
`FULL: the train's (...)`, never `FULL (escalated`. FULL wall time is the job's own `jobs.tsv` row on the host that
ran it. keke = Zen 1, 60 vCPU; mei = Skylake-SP. Both hosts were shared with other roles' jobs at the time, as the
original runs were.

Measured. Two bounded sets were run with `--run --files <list>` on this tree (job `gate-dev-no-full-meas`, keke):
  - Xtask class (`layout-ldso-order-3-gate`'s list): **3,333 s**, rc 0. Split: products build 1,577 s (cold),
    xtask unit tests 1,118 s, `test -p niji` (three start-up layout targets) 205 s, `cargo xtask static` 29 s,
    `layout-check` 16 s, prechecks and clippy in seconds.
  - Image class (`startup-got-page-4-gate`'s list, `_start` in entry.S): **stopped unfinished at about 2,510 s**,
    to keep the run's wall-time deadline. By then the xtask unit tests had taken 1,111 s (that branch's records/
    files select them under the ordinary rules) and the products build was under way. Expected total: about 3,000 to
    3,500 s (est., from the Xtask split: products build, k5 and the layout tests).
These are times of the selection on this branch's code, not on the other branches' code. All other rows are
estimates ("est.") built from those two splits and from the selection size.

| gate (host) | escalating class: first file | FULL ran (s, rc) | bounded selection | bounded time (s) |
|---|---|---|---|---|
| layout-ldso-order-3-gate (mei) | Xtask: xtask/src/hottext.rs, so.rs | 3367, rc 1 | 3 targets + xtask unit, layout-check, layout tests | **3333 measured (keke)** |
| sys-dbg-package-2-gate (mei) | Xtask: xtask/src/so.rs | 1818, rc 124 (timeout) | 5 targets | est. ~3300 (same set) |
| ldso-trace-prefix-gate1 (mei) | Xtask: xtask/src/main.rs (+so.rs) | 2592, rc 1 | 4 targets, 2 unit libs | est. ~3300 |
| rename-niji-docs-gate (mei) | Generated: spec/clauses.tsv (+xtask main.rs) | 4253, rc 0 | 2 targets | est. ~1300 (xtask unit + checks) |
| spec-abi-additions-gate (keke) | Generated: spec/abi/versions*.tsv (+xtask) | 6514, rc 0 | 38 targets | est. ~2000 |
| sl-headers-s05-4-gate (keke) | Generated: headers/*.h.in, libc-include/ (+xtask so.rs) | 4490, rc 1 | 3 targets, 140 conformance | est. ~3400 |
| sl-headers-s17-3-gate (mei) | Image: crates/abi/src/layout.rs (+headers) | 4662, rc 0 | 220 targets, 91 conformance, 4 unit libs (abi one hop) | est. ~4500 (little saved) |
| startup-got-page-4-gate (keke) | Image: csys/src/machine/entry.S (`_start`) | 7432, rc 0 | 3 targets, 4 unit libs, products, k5 | **>2510, stopped; est. 3000-3500** |
| util-8-2-gate1 (keke) | Image: csys/util/locale.c (+xtask so.rs) | 6672, rc 0 | 5 targets, 4 unit libs | est. ~3400 |
| pkg-manifest-2-gate (keke) | Image: tools/deb/*/libc-so.ld | 5849, rc 0 | 5 targets | est. ~2000 (products, no xtask unit) |
| sys-compat-versions-gate (keke) | Image: crates/libm-so/compat_aliases.S | 4744, rc 1 | 6 targets | est. ~2000 |
| sys-thread-db-3-gate (mei) | Config: Cargo.lock, Cargo.toml (+libc-so, xtask) | 2562, rc 0 | 226 targets, 140 conformance (its own Rust changes) | est. ~2500 (little saved) |

The twelve FULL runs took 54,955 s (15.3 h) of host time. The bounded estimates add up to about 34,000 s, roughly
38% less (est.). The keke rows that finished (rc 0, 5,849 to 7,432 s) come down to about half. Saving more takes
something other than selection:
  - The **xtask unit tests take about 1,100 s on keke under load**, which is the largest fixed cost of any change
    touching xtask/ or records/.
  - The **products build** (1,577 s cold) runs serially with the xtask unit tests, although neither needs the other.
    Running them side by side would save about a third of an Xtask or Image bounded run (not tried).
  - The two rows that save little (s17-3, thread-db-3) select 200+ targets under the ordinary Direct rules (the
    crates/abi one-hop dependents; crates/boundary/src/data.rs names `fork`), not under the train route.

For reference, the step split of a FULL run on keke (startup-got-page-4-gate): workspace nextest 3,365 s, products
1,394 s, interposed nextest 1,253 s, remez-check 541 s, interposed scan 333 s, native examples 196 s, rust-dylib
134 s, headers-gen-check 104 s, conformance 44 s; 7,418 s in all. A bounded set drops the interposed pass, remez and
the unrelated test targets.
