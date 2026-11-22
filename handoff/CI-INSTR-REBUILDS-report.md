# CI-INSTR-REBUILDS: full report

Branch `task/ci-instr-rebuilds`, base origin/next 5a9b5d0a1. Code-final commit f579c3e7d (later commits: handoff/ only).

## Commits

* 4cd8a106b Hash only product inputs into the build tag (xtask/src/so.rs: `tag_covers`, `product_crates`,
  `check_tag_covers`, unit tests)
* 5fb6babe5 plants file; e7939561e, e234a0627 D-27.78 record
* 4e3957b8d Hash the iconv alias table into the build tag (`TAG_DATA_FILES`; found by the first plants run's
  baseline: `check_tag_covers` stopped `cargo xtask so` because niji-core `include_bytes!`
  `spec/encoding/iconv-aliases.tsv`, which neither the old nor the new tag covered)
* f579c3e7d D-27.78 key order (records check), plant perl fix; 1dd868422 plants results

## Cause (step 1 of the brief)

`NIJI_BUILD_TAG` (D-27.22) is read with `option_env!` by `niji-boundary` and `niji-ld`, so it is a cargo input of
every release library. It was a hash over every file under `crates/`, `csys/`, `include/` and `Cargo.lock`. Any
change to a test, bench, example or non-product crate changed it, and ld, libc and ldconfig rebuilt (libm and
libresolv too) for products that came out the same.

Most rebuilt runs were still required. The trains' gate logs (changed files vs base) show a product source in 46
of 52 rebuilt runs since 11-19 (e.g. 90390071c: crates/abi/src/layout.rs). 6 had none: 0add1dd07, 8e34f9c72,
08084029d, 33d98c85d, 5a9b5d0a1, 0299fd6e1. Consecutive-tip diffs were not taken (other people's history), so 6/52
is an estimate of the avoidable share.

Not the cause, checked: sync (rsync -c, no mtimes: no source newer than .synced after a build), role reset
(sync_role removes target/niji only), toolchain (1.99.1 pinned in both roles), copying the tree (r0 below), load.

## Replay on a copy of the ci-instr tree (keke, `unshare -r -m`, bind-mounted at the role path; live role untouched)

`NIJI_FULL_PROFILE=1 cargo xtask so`, the copy taken from ci-instr at 90390071c with its target/:

| run | change before it | xtask | ld | libc | ldconfig |
|---|---|---|---|---|---|
| r0-control | none (fresh copy) | old | 0.1 s | 0.0 s | 0.1 s |
| r1-next | sync 5a9b5d0a1 content (product sources changed) | old | 132.2 s | 173.7 s | 145.4 s |
| r2-testonly-old | comment appended to crates/core/tests/hot_text_products.rs | old | 130.9 s | 187.7 s | 147.8 s |
| r3-newtag-first | sync branch content (tag scheme changes once) | new | 122.1 s | 162.7 s | 138.7 s |
| r4-testonly-new | the same test-only edit | new | 0.0 s | 0.0 s | 0.0 s |
| r5-same-tip | none | new | 0.1 s | 0.0 s | 0.0 s |

(r5 overlapped the start of exp2 by 8 minutes: exp2's sync of the old content at 02:36:47 came 13 s into r5. My job
chaining was wrong: `jobwait` returns 3 at its own time limit and I did not loop on it. The sync wrote only files
whose content differs: xtask/, handoff/, records/ and the reverted test file. No product input was touched, and the
new tag over them is the same, so r5's 0 s holds.)

Interference: both replays bind-mounted their copy at the live role path ~/w/ci-instr inside `unshare -r -m`. The
live role was never written, but processes with that cwd made the role look busy to the CI (exp 01:38-02:44Z, exp2
02:36-02:59Z). The coordinator has ruled that CI role paths are never to be used, not even inside a namespace.

r2 against r4 is the fix: an edit no product compiles rebuilt all three on the old tag and reuses all three on the new
one. r4 and r5 are the two consecutive runs on one tip (brief step 2). Every run exits 1 after the cargo steps:
the trace-at-build step's `perf script` fails inside the user namespace ("Map missing from maps"). The live
ci-instr run for the same tip exits 0, and the cargo steps come first, so the evidence above is unaffected.

Cargo's own reasons: NOT AVAILABLE as proof. The brief's filter `CARGO_LOG=cargo::core::compiler::fingerprint=info`
prints nothing on cargo 1.99.1 (checked on a scratch crate with a dirty file), so the r0-r5 logs carry step times
only. `CARGO_LOG=cargo=info` does print the reasons. A second replay with it (keke ci-instr-rebuilds-exp2) was
ended by the coordinator at about 02:59Z and ruled out: like the first replay, its bind mount put its processes'
cwd at the CI role path, and train 899's instr check could not start. Its first run, f1 (the old xtask over
unchanged product sources), had finished at 02:55:30, before the kill. f1 printed `ChangedEnv { var:
"NIJI_BUILD_TAG", previous: Some("src-2539f602ee7a7296"), current: Some("src-b15eba4124f02ed3") }` and no stale
product file. Under that ruling it is quoted here for the record only, not as proof. The coordinator offered
fingerprint logs from the live ci-instr command for the next trains: that is the clean way to get cargo's own
word on real rebuilt runs.

What r0-r5 do show without those lines: r0 proves a fresh copy reuses everything. The only change before r2 is one
appended comment in a test file that no product compiles, and r2 rebuilt all three. On the new tag the same change
rebuilt nothing (r4). The tag is the only input the old and new xtask treat differently for that file.

## Reuse of products built elsewhere (step 3)

Byte-identical across hosts for 90390071c, the release archives from mei ci-z (Skylake-SP, the train's release
build at the end of its full gate) and keke ci-instr (Zen 1), md5 prefixes: libc.a 35094cf142ab, libm.a
05e5943362eb, libresolv.a 20629075c406, ld-pie libld_so.a b39e7376f3c7, libldconfig_bin.a ce2b7cc1ba45. The traced
layout key lists are identical too: hot-text.list eb1bcf091671, startup-text.keys ff76d490b8a4, probe-text.keys
6d72ddd0b284. The final libc.so.6/libm.so.6 were not compared, because mei's target/niji had been rebuilt for the
next tip before the check.

ci-instr cannot use them as the train runs today. That train started at 01:09. ci-instr's own release cargo was done
by about 01:16 (libc.a 01:14, ldconfig 01:16). mei's identical archives appeared only at 01:41-01:46, after its
41-min full gate. Waiting for them would end ci-instr about 25 min later, not sooner. No xtask change was made for
this. A request to the coordinator is below.

## Before / after (step 4)

ci-instr wall time, 78 runs since 11-19 (keke jobs.tsv joined with the step lines):

* median 28.3 min reused (n=13), 34.4 min rebuilt (n=52), 39.0 min with no step lines (n=13)
* rebuilt runs: 2 of 17 trains on 11-19, 23 of 30 on 11-20, 25 of 29 on 11-21
* ci-instr was the train's longest job in 34 of 78. In rebuilt runs: 21 of 52, by +0.1 to +7.8 min. In 20 of those
  21 the lead is under the ~6.1 min rebuild cost, so without the rebuild the gate would have been the longest.
* After: not measurable on trains until this lands. On the replay, an edit with no product input costs 0 s of
  cargo (r4) where it cost 131/188/148 s waited (r2). Expected train effect: only the trains with no product change
  (~6/52, ~12%) drop from about 34 to about 28 min. The required rebuilds stay, so ci-instr remains the longest job
  in about 40% of trains.

## Request to the coordinator (the job that starts ci-instr)

To remove the required rebuilds too, build the release products for the train's tip FIRST on the Skylake-SP host,
not at the end of the full gate (its three cargo builds took about 2 min there for 90390071c). ci-instr would then
take the products. That needs an xtask mode that accepts a product directory only when its build tag, head,
toolchain and profile match: a follow-up story (RULING NEEDED, a cache trust boundary). The larger prize is the
traced-layout step (`so/libc-archive`, 588-804 s per ci-instr run, every run): its key lists are identical across
the two hosts, so the whole release `so` output might be shareable. Prove the final libraries byte-identical first.

## Gate, plants, lint (mei:~/w/s1, f579c3e7d)

* ci-instr-rebuilds-lint3: records check, clippy --workspace --all-targets --all-features -D warnings, static,
  `cargo test -p xtask so::tests`, affected --list (direct tier, not FULL): rc 0
* ci-instr-rebuilds-gate (`cargo xtask affected --run --base origin/next`): 7 suites ok, 633 passed, 0 failed,
  peak 2.7G, rc 0
* ci-instr-rebuilds-plants-3: 11 plants, 9 CAUGHT, 0 SURVIVED, 2 WARN-NO-MSG (workspace-deps-dropped,
  data-files-dropped: both failed in the plant's own `cargo xtask so` with the tag check's "...which the build tag
  does not cover" before the unit tests ran; the msg: regexes name only the unit tests)
* SUPERSEDED: ci-instr-rebuilds-plants (baseline `so` failed on the uncovered iconv table; fixed in 4e3957b8d),
  ci-instr-rebuilds-plants-2 (a perl pattern held a slash; refused before any plant), ci-instr-rebuilds-lint
  (stale sync, killed), ci-instr-rebuilds-lint2 (records key order; fixed), ci-instr-rebuilds-unit1 (leg 1, rc 101;
  unit2 green on that code), keke:ci-instr-rebuilds-exp2 (ended by the coordinator: it held the CI role path; not used as proof)

## Informational

* `so/libc-archive` (traced layout) costs 588-804 s in every ci-instr run, rebuilt or not: more than the rebuild.
* `build_tag(root)` walks and reads the tree about 8 times per `cargo xtask so`: minor.
* The brief's CARGO_LOG target prints nothing on cargo 1.99.1. Use `CARGO_LOG=cargo=info`, or `-v` (a
  `Dirty <pkg>: <reason>` line, but xtask passes `-q`).
