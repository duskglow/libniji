# CI-INSTR-REBUILDS: why ci-instr rebuilds the release libraries (handoff, final)

State: branch `task/ci-instr-rebuilds`, base origin/next 5a9b5d0a1, code-final f579c3e7d (later commits handoff/
only). Role mei:~/w/s1 (Skylake-SP). ci-instr runs on keke (Zen 1 host, role ci-instr). Full report:
handoff/CI-INSTR-REBUILDS-report.md. Plants with results: handoff/CI-INSTR-REBUILDS-plants.txt.

## Decisions in force

* D-27.78 (amends D-27.22). The build tag hashes product inputs only: Cargo.lock, csys/, include/,
  crates/Cargo.{toml,lock}, `TAG_DATA_FILES` (spec/encoding/iconv-aliases.tsv), and the crates the five product
  manifests reach by [dependencies]/[build-dependencies] (never dev-deps), without tests/ benches/ examples/.
  `check_tag_covers` fails any product build whose dep-info names a workspace file the tag leaves out.

## Done (do not redo)

* Cause found. A tag over every crates/ file made every train rebuild ld, libc and ldconfig. Most rebuilds are
  still required by product changes; about 6 of 52 were not.
* The fix, unit tests, plants (9 CAUGHT, 2 WARN-NO-MSG caught by the build-time check), lint3, direct gate (633 passed).
* Replay on a copy of ci-instr (keke): a test-only edit rebuilds on the old xtask (r2) and costs 0 s on the new one (r4, r5).
* Release archives and traced key lists are byte-identical between the hosts for 90390071c.
* Before numbers: median 28.3 min reused, 34.4 min rebuilt.

## Open questions / rulings

* RULING NEEDED: whether to run the train's release build first on the Skylake-SP host and import it into
  ci-instr, through an xtask import mode that checks tag, head, toolchain and profile (a cache trust boundary).
  Recommended as a follow-up story. It is not possible today: those products appear about 25 min after ci-instr
  needs them.
* Candidate story: share or avoid the traced-layout step (`so/libc-archive`, 588-804 s every run).

* NOT VERIFIED: cargo's own fingerprint reasons for real rebuilt runs. The brief's CARGO_LOG filter is silent on
  cargo 1.99.1, and the replay that used cargo=info (exp2) was ruled out by the coordinator. Take the coordinator's
  offer: CARGO_LOG=cargo=info in the live ci-instr command for a few trains. Never use a CI role path again, not
  even inside a namespace.

## Next step

None on this branch. After landing: measure the median ci-instr wall time on trains with no product change
(expected ~28 min instead of ~34 min).
