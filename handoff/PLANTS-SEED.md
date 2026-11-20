# PLANTS-SEED: seed plants copies with unchanged build artefacts — measured, nothing shipped

State: branch `task/plants-seed`, stacked on `task/plants-copies` (its 4 commits are PLANTS-COPIES', not this story's).
This story adds only this note and `handoff/reports/PLANTS-SEED.json`. No code change. Verdict: DO NOT LAND (evidence only).

## Premise check (flag #6866; the coordinator agreed)

The brief assumed that most of a plants copy's cold build is third-party crates. That is false. `Cargo.lock` (main
workspace, 14 packages) and `crates/Cargo.lock` (product workspace, 13 packages) have no package with a registry or
git source, and there is no build-std. Only `fuzz/` has registry dependencies, and `cargo xtask so` does not build it.
So there are no third-party artefacts to seed.

## Measurement (Zen 1 host, 60 vCPU, jobrun without a slot cap, host load about 15)

Job `plants-seed-m1`: a fresh copy made the way the plants tool makes one (`rsync -a`, without `/target` and
`/fuzz/target`) under `/dev/shm`, with the plants environment (`NIJI_LAYOUT_TRACE=ungrouped`, a fixed
`NIJI_PLANTS_BUILD_TAG`). Then `cargo xtask so` under `/usr/bin/time -v`, with a `RUSTC_WRAPPER` that records wall and CPU
time per rustc call. The scripts are in `~/work/plants-seed/` on the host.

Cold `xtask so`: **360 s wall, 911 CPU-s (845 user + 66 sys), max RSS 1.18 GB, 44 rustc calls, 836 rustc CPU-s (92%)**.

| package [target dir] | rustc wall s | CPU s |
|---|---|---|
| xtask [debug] (the tool itself, `cargo run -p xtask`) | 90.9 | 241.7 |
| niji-boundary [product/ld-pie/fast] (`-Ccodegen-units=1`) | 197.1 | 183.7 |
| niji-boundary [product/fast] | 66.0 | 118.9 |
| niji-libc-so [product/fast] | 27.3 | 78.6 |
| niji-libm-so [product/fast] | 19.8 | 57.1 |
| niji [ld-pie / fast] | 39.6 / 31.6 | 33.1 / 32.0 |
| niji-math, niji-legacy, niji-ld, niji-ldconfig-bin, niji-ld-so (both dirs) | | 85 together |
| niji-rt, niji-kernel, niji-abi, niji-mem (all dirs) | | 13 together |

Everything here is a workspace crate. No third-party crates are built, so the third-party share is 0 s.

**Critical path:** niji-boundary for ld.so is a single rustc call of 198 s wall (codegen-units=1, `LD_RUSTFLAGS`), 55% of
the cold build's wall time. In a 4-core slot it runs on one core while the rest of the build waits for it.

## Seeding from the run's own baseline (the safe-looking alternative) does not get reused

Same job, second step: a second copy, rsync'd the same way, got a `cp -a` of copy 1's freshly built `target/` (458 MB,
0.6 s). Then `cargo xtask so` ran again:
**354 s wall, 884 CPU-s, 31 of 44 rustc calls re-ran (810 of 836 rustc CPU-s)**, so the seed saved about 3%.
- Product crates: their rustflags carry `--remap-path-prefix=<absolute copy root>=...` (`xtask/src/align.rs`
  `remap_flags`, via `encoded_rustflags`). The flags, and with them each unit's fingerprint, therefore depend on where the
  copy sits. Every product crate rebuilds in a copy at another path.
- xtask [debug] also rebuilt (234 CPU-s). Cause not determined: `CARGO_LOG=cargo::core::compiler::fingerprint=info`
  printed nothing through `cargo xtask so` (job `plants-seed-why`, which rebuilt fully again: 358 s, 951 CPU-s; that job
  ran without the wrapper, so it is not a clean A/B).
- After the seeded build, 10 files in copy 2's `target/` still name copy 1's paths: the reused debug units niji-ld,
  niji-rt and niji-kernel (`.rlib`/`.rmeta`/`.d`), and xtask's build-script `.d`. Separately, `xtask/build.rs` writes
  `#[path = "<absolute>"]` modules into its `OUT_DIR`. If a seeded copy reused that `OUT_DIR`, it would compile another
  copy's header-slice sources. This is a concrete stale-source hazard for any cross-copy seed.
- Products: `diff -rq target/niji` between the two copies differs only in `target/niji/.hot-lists` (not investigated).

## The most any seed can save

A seed saves one cold product build per copy beyond the first, and nothing for a serial run (`--copies 1`). Against
PLANTS-COPIES' 2-copy run (2789 s, 8574 CPU-s; cold build 564–574 s per copy in a shared 4-core slot, 377 s alone), the
saving is at most about 180–200 s wall (~7%) and ~900 CPU-s (~11%). Making it real would need copies at the same path
(for example each copy in its own `unshare -r -m` namespace, bind-mounted at one fixed path). It would also need the
coordinator's proof plants: a mutant in each crate class (leaf, depended-on, build.rs input, generated file) rebuilt and
CAUGHT from a seeded copy, including a mutated file older than the seed.

## Decisions / rulings

- RULING NEEDED: close PLANTS-SEED with no seed. Recommendation: do not build same-path copies for a ≤7% wall gain on
  multi-copy runs only.
- RULING NEEDED: the bigger lever for every cold product build (plants copies, gates, CI) is the 198 s single-unit
  niji-boundary ld-pie build. Recommendation: a measurement story on what ld.so needs from niji-boundary at
  codegen-units=1 (LAYOUT-LDSO-WINDOW is the constraint; read its D-rows first).
- INFORMATIONAL: the audit's 1364 s / 4812 CPU-s cold figure was not reproduced. Today's cold `xtask so` in a plants-style
  copy on the same host class is 360 s / 911 CPU-s. The audit figure probably includes test builds or slot contention.
- INFORMATIONAL: the coordinator note asked me to merge origin/next and the base before hand-back. Not done: this branch
  ships no code. The plants.rs conflict is PLANTS-COPIES-2's to resolve.

## Not to redo

Third-party seeding (there are none), and `cp -a` of another copy's `target/` (measured: about 3% reuse, and stale-path
hazards).

## Next step

None for this story. If the same-path seed is ever wanted: per-copy mount namespace at a fixed path, then the
coordinator's four proof plants, then before/after plants runs on one host.
