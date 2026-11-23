# AFFECTED-TESTS-ONE-LOAD (TEST-REDUNDANCY-AUDIT M4): a cheap `Repo::load`

State: branch `task/affected-tests-one-load`, base origin/next 0495c4684; HEAD named in the report json. Decision in
force: D-X.160 (records/decisions/D-X/D-X.160.toml). Plants: handoff/AFFECTED-TESTS-ONE-LOAD-plants.txt.

## What was chosen, and why

Option 1 of the brief (make the load cheap), after profiling: it is a pure gain for every `cargo xtask affected`
too, keeps every test as it is (names, assertions, messages, one process each under nextest), changes no gate
mechanics and keeps nothing on disk (no cache, so no staleness question).

Profile (perf, Zen 1 build host, `xtask affected --list --files README.md` = one load plus a trivial selection,
base 0495c4684): 34.97 G instructions, 4.7 s. Of that, `Repo::load` was 77 %, and `refs_of` (tokenising Rust text,
one `String` per identifier) 62 % of the total. Cause: each test target's text has every support directory it names
appended (`text.contains(<dir name>)`); core's `tests/oracle/` (1.2 MB of C) goes to about a hundred targets, so the
targets' texts total 137 MB from 7 MB of their own, and all of it was tokenised. Next: the csys symbol scan
(`sym_refs`, a BTreeSet<String> lookup per word, ~24 % after the first fix) and the all-pairs hub counts (two
`String` clones per node pair).

Changes (xtask/src/affected.rs only):
1. `Tok` borrows from the text; `refs_closed` returns the references and whether the text is CLOSED (no
   `use`/`mod`/`as`/path cut off by its end, no `::` at either end). Proof sketch in its doc comment: the tokens of
   texts joined with `\n` are theirs one after the other (tokenising is per line), and a closed text's only reads
   outside itself are "is the next token `::`" and "did the previous token be `::`", both false between closed
   texts; so the references of joined closed texts are the union of each one's.
2. `Repo::load` reads each support directory's files once per crate (text, references, closed) and gives a target
   its own references plus those of its support files when every part is closed; otherwise it reads the whole
   joined text as before. Target texts, `support_texts` and the support-dir rule (tested on the growing text) are
   unchanged.
3. `sym_refs` looks words up in a `HashSet<&str>` (membership only, so order cannot matter).
4. `hubs` and `entry_hubs` go from each text's references to the nodes they name through a (crate, module) index
   instead of every node against every node; same counts (each named node once per text).

## Results (all measured on the Zen 1 build host, mei:~/w/s3)

| measure | before (0495c4684) | after | source |
|---|---|---|---|
| `xtask affected --list --files README.md`, instructions | 34.97 G | 10.65 G | perf stat, 3 runs each, identical to 0.01 % |
| same, task-clock | 4.70 s | 1.48 s | same runs |
| `affected::tests` under nextest (16 threads), instructions of the whole run | 3,894.6 G | 1,945.3 G | jobs affected-tests-one-load-suite-base / -suite-new |
| same, CPU-s | 688 | 262 | same |
| same, sum of per-test seconds | 1,924.8 (104 tests) | 530.3 (106 tests: +2 new) | same (host under other load: test-s is noisy) |
| same, nextest wall | 146 s | 53 s | same |

The audit's 1,403 test-s (T1/T4) is the CI-gate figure; the brief asks the coordinator to read the next CI gates'
`affected::tests` PASS lines (T1 method) for the landed number. Expected: about -70 % of that pass-1 share.

Not done (next, if wanted): the remaining selection cost per test is mostly `is_word`/`contains` scans over the same
137 MB of target text (`Target::text` holds own + support); splitting it into own + shared support texts would cut
the selection side the same way, but every reader of `text` would need checking. SipHash of every scanned word in
`sym_refs` is ~16 % of the remaining load (a first-byte/length prefilter before the lookup is exact and cheap).

## Tests and plants

New: `closed_texts_join_by_the_union_of_their_references` (a table of closed and cut-off fragments: the union
equals the joined text for every closed pair; each cut-off kind is not closed, and five pairs where the union would
be wrong) and `a_target_s_references_are_those_of_its_whole_text` (on the tree, one target per distinct support set).
Plants: 7 (see the plants file); results in the report json.

## Gate

Direct tier (xtask unit tests + static). First run red only in `xtask --test transcripts` (two iconv tests): the
role held a `target/niji-test/iconv` linked against an interpreter under the role's former path
(`/mnt/zram/.../target/niji/ld-linux-x86-64.so.2`, absent; `target/niji` absent), so exec gave ENOENT although
the test's skip check (`is_file`) passed. Not this change; rerun after `cargo xtask so` (job
affected-tests-one-load-transcripts).

## Exact next step

Coordinator: land; read the next CI gates' `affected::tests` test-s against 1,403.
