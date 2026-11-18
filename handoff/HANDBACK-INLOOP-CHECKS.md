# HANDBACK-INLOOP-CHECKS: the hand-back's checks in the agent's own loop

Branch `task/handback-inloop-checks-2` (from origin/next 923df67e9; carries run 1's `task/handback-inloop-checks`).
Role mei:~/w/s1 (Skylake-SP, 48 threads). Code head 4f3530282 (later commits: handoff/ only).
Run 2 (HANDBACK-INLOOP-CHECKS-2) finished the story: affected prechecks, plants, docs, wall time, gate. Its first
session died on a login expiry while gate2 and plants2 ran; the second session fixed gate2's red (below) and reran.

## What the branch does

- `cargo xtask precheck [--jobs a,b,host:c] [--head <sha>] [--only <checks>] ...` (`xtask/src/precheck.rs`): every
  cheap check that judges a hand-back, one verdict line each, red lines name the fix; exit 0 green, 1 red, 3 WAIT
  (a named job not ended), 2 usage. Module doc lists each check and the ops/CI rule it mirrors.
- `xtask/src/affected.rs` (rule 10): `GEN_PRECHECKS` maps every generator's inputs AND outputs (directory prefixes;
  stub files by pattern, `gen_prechecks_for`) to its `--check`; a selection that escalates to FULL for any reason runs
  every `precheck::CHEAP_STEPS` step first, `stubs` first (`prechecks`); a red ends the run in seconds with each fix
  (`run_prechecks_or_stop`). The FULL run then repeats them inside `cargo xtask test` (~45 s, kept so that command stays
  complete on its own).
- `main.rs`: `cheap_step` runs one `CHEAP_STEPS` step (test_steps, precheck and affected all call it: one list,
  cannot drift); `static_list()` shared with precheck. `cheap_step` stays in main.rs because only main.rs may name
  the audit/tool modules (`policy`, `fuzz`) for a gate step (affected's leaf tests).
- `spec.rs`: `stubs` writes only changed files, every table and stub (`write_if_changed`): the gate runs stubs in
  the role, and precheck reads the role's times to tell what changed after a job started.
- Docs: CLAUDE.md workflow and CONTRIBUTING.md step 5 (run precheck before a hand-back; exit 1 = NOT READY).

## Results (run 2)

- Step 0 on the inherited head: clippy, static, precheck tests green (`handback-inloop-checks-2-lint0`).
- End-to-end `cargo xtask precheck` on the role (head 998f2b05a, warm build): GREEN, 31 checks, wall 73 s
  (headers-gen-check 39.6 s, static 13.7 s, wl_mtmem self-test 10.0 s, tables 3.9 s; everything else < 2 s)
  (`handback-inloop-checks-2-pc1`). A header slice touches headers/ and spec/abi/ only (no Rust), so clippy is a
  no-op and the wall time is this ~75 s; target was < 5 min. Measured on mei, not keke (my role is on mei).
- Unit tests (15, `cargo test -p xtask -- precheck generator escalated layouts_table stubs_rewrite`), clippy, static:
  green on e9f5232 (`handback-inloop-checks-2-unit2`).
- Gate on f4ebaaf (`-gate2`, escalated to FULL: xtask changed) RED: 6809 passed, 2 failed,
  `affected::tests::audit_modules_are_leaves` and `tool_modules_stay_off_the_product_path` (precheck.rs named
  `policy::`/`fuzz::`; the second test also read `static_checks`/`test_steps` bodies this story split). unit2's
  filter had left both out: run the whole `cargo test -p xtask` on an xtask change. Fixed in 59d90efc9 (cheap_step
  back in main.rs; the test reads cheap_step and static_list too). Full xtask tests 486 passed, clippy, static
  green (`-lint4`). Superseded: `-gate`, `-plants` (rc 124 after 10-17 s, restarted a minute later as -gate2/-plants2 by the first session; its
  reason was not recorded), `-gate2` (red),
  `-plants2` (22/22 CAUGHT, but on f4ebaaf), `-lint3` (the static_list half still red).
- On 59d90efc9: plants3 22/22 CAUGHT; gate3 GREEN (12147 passed, 0 failed). Both superseded by the fixes below.
- The first real hand-back precheck (`-pc2`, after a handoff-only sync) was a FALSE RED: it called gate3 and plants3
  stale, for two reasons. (a) The sync had removed the ignored `conformance/build`, and precheck's own fuzz check
  had written `fuzz/target`, which only `fuzz/.gitignore` ignores; a newer directory counted as a change.
  (b) `stubs` rewrote identical `spec/symbols.tsv` and `spec/primitives.tsv` inside the gate. Fixed in e1ebc6786
  (nested .gitignores; a newer directory counts only when it lacks an entry of the merge base's copy
  `~/wl/src/<merge base>`; strict without the copy) and 4f3530282 (`write_if_changed` for all stubs tables; the stubs
  snapshot covers them). `-pc3` (after e1ebc67) named only the real change plus the two tables, which 4f35302 fixed.
  Full xtask tests 488 passed, clippy, static green (`-lint6`). Superseded: `-lint5`, `-pc2`, `-pc3`, `-gate3`, `-plants3`.
- Gate on 4f3530282 (`-gate4`, escalated to FULL: xtask changed): GREEN, 32 suites, 12151 passed, 0 failed,
  conformance 139/0, `affected: ok`, peak 11G, oom_kill=0, 35.5 min.
- Plants (26: 7 red classes on the fixture or the real tree, 19 code plants) on 4f3530282: `-plants4` ended rc 3 at
  once (`REFUSED: line 26: bad plant id`: my 5b59921 edit lost the space before a `|`), fixed in 49272ae; `-plants5`
  then REFUSED `code-stale-dir-rule` (pattern matched no site since e1ebc67 put that push in a `None =>` arm), fixed in
  513d2eb (every lit: plant checked to match one site). `-plants6` (head 513d2eb, code = 4f3530282): 26/26 CAUGHT,
  0 WARN, 70 min. Results: handoff/HANDBACK-INLOOP-CHECKS-plants.txt. Superseded: `-plants4`, `-plants5`.
- Final `cargo xtask precheck --jobs gate4,plants6 --head <sha>` (`-pc4`): RED, static
  private-paths: two home paths in leg 2's handoff/report text (the refused command); written as `~/...`, then `-pc5`
  on the final head (verdict in the hand-back). Superseded: `-pc4`.

## Post-hand-back checks and where they are now

| check (who runs it after hand-back) | in precheck? | time (Skylake-SP) |
|---|---|---|
| FULL `stubs` (gate's first step) | yes, plus "stubs rewrote X" | 0.3 s |
| purity, unsafe/csys/env/opaque/crash/reexec audits, trace, fuzz-check | yes | < 1.1 s each |
| csys/abi/unicode/encodings/locale/native/compat/math-gen --check | yes | 0-1.6 s each |
| headers-gen --check | yes | 40 s |
| remez --check, tables --check | yes | 0.1 s, 3.9 s |
| validation self-tests (FULL only) | yes | 10.4 s |
| `static` (CI precheck, hbc step c) | yes, deduplicated | 13.7 s |
| clippy (hbc step c) | yes | incremental |
| hbc a: label resolves, rc, plants through the tool | yes (this host's labels) | ms |
| hbc a2: tested content = head | yes | < 1 s |
| hbc b: placeholders, train-owned views | yes | ms |
| hbc b: decision id reserved for the story | no: needs the ops store (`ops handback-check`) | - |
| products, nextest passes, conformance, native-examples, rust-dylib, symgap | no: too slow (need the products) | 424 s, 1385 s + 875 s, 27 s, 54 s, 66 s |

## For the coordinator (ops-side text; rules/developer-flow.md is read-only here)

After step 4's clippy/static paragraph: "Before you hand back (not INCOMPLETE), sync your final head and run on your
role `cargo xtask precheck --jobs <your JOBS labels> --head <sha>` (about 75 s warm). Exit 1 (any RED) = NOT READY:
fix what it names. Exit 3 (WAIT) = a named job has not ended: jobwait it, or hand back PENDING JOBS."
`ops handback-check` could call it on the role too (same rules, plus the id reservation it alone can check).

## Decisions (local, testable; no ruling needed)

- Role trees have no git: changed files from the sync's `.changed`, added lines against `~/wl/src/<merge_base>`
  (without it every line counts: stricter). Tested content: equal `content_tree`, else role files newer than the
  `.tested` record outside prose and every `.gitignore` (nested ones for their own subtree); a newer directory is red
  only when it lacks an entry the merge base's copy has (without the copy: any newer directory, strict). Blind spots,
  both caught by `ops handback-check` (it compares the content trees): a branch-added file removed after the job,
  and a removed top-level file.
- A `gate` job with rc 0 and no `affected: ok` line is red.
- The escalated FULL run repeats the cheap steps (~45 s of a ~45 min gate) rather than adding a skip switch to
  `cargo xtask test` (a switch could make that command incomplete; slow and working wins).

## Not to redo / next step

State (leg 3): code done at 4f3530282; gate4 and plants6 green; the final precheck `-pc4` and
`ops handback-check` ran on the last head (results in the hand-back and the report json). Nothing left to do.
REFUSED in leg 2 (reported): `python3 ~/src/liella-ops/bin/ops extend-budget HANDBACK-INLOOP-CHECKS-2
--why "..."` (auto-mode classifier).

The story is done. A plants-file check (parse every line, each pattern matches one site) belongs in precheck: two
plants runs here died at once on file errors that a 1 s check finds. Possible follow-ups: (1) wire `ops handback-check` to run precheck on the role;
(2) when the FULL escalation comes from xtask/src only, run the xtask unit tests (~3 min warm) among the prechecks,
before the products: gate2's red (two xtask unit tests) took 44 min to show and would have shown in about 3.
