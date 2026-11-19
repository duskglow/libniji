# PLANTS-BUILD-SLOWDOWN: why plants' products rebuilds got slower, and the fix

State: branch `task/plants-build-slowdown` (from origin/next 255a6901d). Code commits: 2529671cb (headers-gen
index), c8a9c1301 (early interpreter/ldconfig builds), fca7b1e6e (unit test pinning the index); plants file
`handoff/PLANTS-BUILD-SLOWDOWN-plants.txt`. Report record: `handoff/reports/PLANTS-BUILD-SLOWDOWN.json`.

## Cause (found by timing phases and a perf profile, not by bisecting)

`cargo xtask so` runs `headers::stage`, so it runs the header generator in `--check` mode before every product build,
plants mutants included. The generator answered "this header's rows of this name" and "the configurations that do
not decide this name" (`Gen::rows`, `Gen::dont_care`, `is_covered`, `narrower_tag`, enum lookups) by scanning all
72 278 visibility.tsv rows (each a `BTreeMap<String, String>`) once per name per header: quadratic in the table.
`perf record` of `cargo xtask headers-gen --check` (161 s): 58% BTreeMap search plus 25% memcmp under those scans.
The cost grows with every header slice that adds rows and names, which matches the step on 11-17/11-18 on both hosts
at once. Second cost: `NIJI_BUILD_TAG` (a hash of crates/, csys/, include/, Cargo.lock) is read with `option_env!` by
`niji-boundary` and `niji-ld`, so any change to a hashed file rebuilds boundary (135k lines) in every product target
directory, even a one-line change to a 24-line root crate.

Not bisected: thread_common forbids checking out other people's old commits. The profile names the cause directly.
For per-commit numbers, time `cargo xtask headers-gen --check` on primed `~/wl/src/<sha>` trees.

## Numbers (Skylake-SP build host, 48 threads, RAM copy, `NIJI_LAYOUT_TRACE=ungrouped`, load as noted)

| build | origin/next | + index | + early builds |
|---|---|---|---|
| headers-gen --check alone | 161 s (perf run) | 0.66 s | - |
| cold `cargo xtask so` | 524 s (load 18-6) | 236 s (3-9) | 224 s (41-12) |
| no-op | 190 s (6) | 40 s (9) | 43 s (12-11) |
| mutant crates/core/src/lib.rs | 351 s (5) | - | - |
| mutant crates/boundary/src/hot/hyperbolic.rs | - | 174 s (9-5) | 117 s (11-24) |
| mutant crates/libc-so/src/lib.rs (root crate) | 344 s (5) | 173 s (9-5) | 118 s (39-45) |
| mutant crates/ld/src/start.rs | - | 177 s (6) | - |
| mutant xtask/src/headers.rs | 196 s (4) | - | - |

Phases of a boundary mutant after the index: libc 23 s, libm 21 s, ld 49 s, ldconfig 49 s (the two last are one
codegen unit per crate, so mostly one thread each). With the early builds: libc 28, libm 24, ld 33, ldconfig 0.1.
Constant tag (scratch copy only, not committed): root-crate mutant 173 s -> 48 s (load 38-40); boundary mutant
171 s (unchanged: boundary rebuilds anyway). The per-mutant restore build costs the same as the mutant build.
Keke (Zen 1) was not measured: this run's only role was on the Skylake-SP host. Expect the same 150 s fixed cost
gone (the profile is CPU-bound and does not depend on the host).

## What changed

1. `VisIndex` in xtask/src/headers.rs: built once at load. header -> name -> row indices (table order),
   header -> row indices, joined include-error configurations per header, joined header-less configurations per
   (name, kind). Every lookup returns the scan's rows in the scan's order. `headers-gen --check` passes (generated
   headers byte-identical) and `headers::tests::vis_index_answers_as_the_row_scan` pins each answer.
2. `EarlyBuilds` in xtask/src/so.rs: `run_so_as` starts the interpreter's and ldconfig's cargo commands in a thread
   right after taking the product lock, with their output dropped. `run_ld` and `run_ldconfig` still run the same
   commands (cargo answers from the early builds or redoes them, and reports failures). Dropping the guard joins the
   thread, so no build outlives the run.

## Plants

First run (plants-1, head c8a9c1301): 2 CAUGHT, 2 SURVIVED, 1 EQUIVALENT. Include-error rows dropped and kind
ignored both survived `headers-gen --check`, because today's tables give the same conditions. Fixed by the unit test
(fca7b1e6e). Final run: plants2 (see the report). `pbs-early-builds-off` is EQUIVALENT: it only changes build time.

## Open (RULING NEEDED, see the report)

- Build tag out of rustc's inputs (D-27.22, L14-10): measured 173 -> 48 s for a mutant outside boundary's dependency
  cone. Options: ld.so compares libc's tag with one the build links into both from an xtask-assembled object, or a
  plants copy builds with one tag per run.
- Concurrent mutants in `cargo xtask plants`: N copies, each with its own target dir (a cold build is ~225 s, and the
  target dir takes GBs of RAM). Worth it only after the two items above.

Next step: land; then ask for the tag ruling. Read: xtask/src/so.rs `build_tag`, `EarlyBuilds`;
xtask/src/headers.rs `VisIndex`.
