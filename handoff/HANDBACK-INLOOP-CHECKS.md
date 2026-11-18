# HANDBACK-INLOOP-CHECKS: the hand-back's checks in the agent's own loop

Branch `task/handback-inloop-checks` (from origin/next 923df67e9). Role mei:~/w/s1. Run 1 stopped at the context
checkpoint (~150k) with the command written and pushed as WIP. Code head 91fb25946: job
`handback-inloop-checks-wip-unit3` on mei (`cargo test -p xtask precheck` 9 passed, `cargo xtask clippy`, `cargo xtask static`)
rc 0. Superseded: `-wip-unit` (edition 2024: `gen` is reserved), `-wip-unit2` (lab-names: a host name in a test).

## State (run 1)

- `xtask/src/precheck.rs` (new): `cargo xtask precheck [--jobs a,b,host:c] [--head <sha>] [--only <checks>] [--base
  <ref>] [--logs <dir>] [--base-src <dir>] [--host <name>] [--story <id>]`. One verdict line per check, each red
  with its fix; exit 0 green, 1 red, 3 WAIT (no red, a named job not ended), 2 usage. The module doc lists every
  check with the ops rule it mirrors (ops/handback.py steps a, a2, b; ci.py placeholders and train-owned;
  dispatcher.tested_diff and _BASELINE_LABEL).
- `xtask/src/main.rs`: `precheck` dispatch; `static_list()` (the `static` checks, shared); `test_steps` runs
  `precheck::CHEAP_STEPS` (minus stubs) through `precheck::cheap_step`, same order and step names as before.
- `xtask/src/spec.rs`: `stubs` writes `spec/clauses.tsv` and a stub file only when the text changed (a role's
  mtimes are how precheck tells what a later sync changed; the gate runs stubs in the role).
- `xtask/tests/fixtures/precheck/`: a hand-back laid out as a role plus its host's logs (green on every check);
  `precheck::tests::precheck_fixture_hand_back_is_green` runs it. Placeholders are written `{{NEW}}` there and in
  the tests are built from parts, because CI's placeholder check reads every added line outside handoff/.

## Design decisions (local, testable; no ruling needed)

- Role trees have no git. Changed files come from the sync's `.changed`; added lines are lines the merge base's
  copy (`~/wl/src/<merge_base>`, primed by sync_role) lacks; without it every line of a changed file counts (stricter).
- Tested content (step a2) on the role: equal `content_tree` in `~/logs/.tested/<label>` and the role's `.synced` is
  the exact case. Otherwise `stale_files`: role files and directories newer than the `.tested` copy (sync refuses
  BUSY while a job runs, and rsync -c without -t touches only changed files), outside prose (handoff/, docs/,
  records/, results/, *.md), `.gitignore`d paths and the sync excludes. A newer directory is a red (an entry was
  added or removed). Blind spot: removing a top-level file after a job (the root is relinked every sync); stated in
  the doc, and `ops handback-check` still catches it. In a git checkout the exact `git diff` rule runs instead.
- A `gate`-kind job with rc 0 and no `affected: ok` line is red (an `--list` run, or one that stopped early).

## Post-hand-back checks found and where they are now

| check (who runs it after hand-back) | in precheck? | time |
|---|---|---|
| FULL `stubs` (gate's first step) | yes, plus "stubs rewrote X" | 0.3 s |
| purity, unsafe/csys/env/opaque/crash/reexec audits, trace, fuzz-check | yes | < 1 s each |
| csys/abi/unicode/encodings/locale/native/compat/math-gen --check | yes | 0-1.4 s each |
| headers-gen --check | yes | 47 s |
| remez --check, tables --check | yes | 0.1 s, 4.1 s |
| validation self-tests (FULL only) | yes | ~10 s |
| `static` (CI precheck, hbc step c) | yes, deduplicated | seconds |
| clippy (hbc step c) | yes | incremental |
| hbc a: label resolves, rc, plants through the tool | yes (this host's labels) | ms |
| hbc a2: tested content = head | yes (see above) | < 1 s |
| hbc b: placeholders, train-owned views | yes | ms |
| hbc b: decision id reserved for the story | no: needs the ops store (`ops handback-check`) | - |
| products, nextest passes, conformance, native-examples, rust-dylib, symgap | no: too slow (need the products) | 424 s, 1385 s + 875 s, 27 s, 54 s, 66 s |

Times: the FULL gate `headers-abi-regen-gate` on mei (Zen 1 host), 11-18.

## Next steps (run 2)

1. Not yet run for real: `cargo xtask precheck` on the role end to end (watch for false stale reds from in-tree writers).
2. `xtask/src/affected.rs` (brief item 2): widen `GEN_PRECHECKS` to prefix entries for every generator input and
   output (spec/abi/ -> abi, headers, compat, math, locale, native gen checks and stubs; headers/ and libc-include/
   -> headers; spec/unicode/ -> unicode; spec/encoding/ -> encodings; csys/syscalls.tsv -> csys;
   crates/boundary/src/posix/s10_math.tsv -> math; glibc-functional-spec.md -> stubs), and when the selection
   escalates to FULL run every `precheck::CHEAP_STEPS` step first, stopping on a red (now `stubs` red runs the whole
   FULL gate). Call `precheck::cheap_step` from `gen_precheck`. Tests in affected's test module.
3. Plants file `handoff/HANDBACK-INLOOP-CHECKS-plants.txt`, one per red class against the fixture, cmd
   `cargo test -p xtask precheck_fixture_hand_back_is_green`: (1) queued: rename the gate's jobs.tsv label;
   (2) failing job: plants row rc 0 -> 1 (perl: pattern, tabs); (3) stale: `newer` line handoff/... -> src/a.rs;
   (4) stubs: real tree, mutate the `spec/clauses.tsv` header, cmd `cargo xtask precheck --only stubs`;
   (5) undiagnosed gate: `affected: ok` line in the gate log changed; (6) placeholder: docs row `COMPAT-12` ->
   `COMPAT-{{NEW}}`. Plus code plants on precheck.rs (has_placeholder, is_baseline_label, stale_files dir rule,
   from_role base lines) and affected (prefix mapping, FULL prechecks).
4. Docs: CLAUDE.md workflow and CONTRIBUTING.md hand-back section (run `cargo xtask precheck --jobs ... --head ...`
   before every hand-back that is not INCOMPLETE; red = NOT READY). The developer flow lives in the ops repo
   (rules/developer-flow.md, read-only here): the coordinator adds, after step 4's clippy/static paragraph:
   "Before you hand back (not INCOMPLETE), sync your final head and run on your role
   `cargo xtask precheck --jobs <your JOBS labels> --head <sha>` (a few minutes). Exit 1 (any RED) = NOT READY: fix
   what it names. Exit 3 (WAIT) = a named job has not ended: jobwait it, or hand back PENDING JOBS."
5. Wall time on a header-slice-sized change (target < 5 min), then the direct-tier gate on the final head; wait.
