# UTIL-14 report: gencat in libc-dev-bin (GitHub issue #12)

Branch `task/util-14` from origin/next `b04f71229`. Code head `4809e2d1f` (the commits after it are handoff/ only).

## UTIL-14-2 (continuation, 2026-11-23): branch `task/util-14-2`

CI red on `2e25f9305` (train 946: seven locale/products tests) was settled by the coordinator as a host change on the
CI host (#7846; LOCALE-PER-DIR-LOAD fixes it). This run: merged origin/next `0495c4684` (`bd99cec16`; one conflict in
docs/install-packages.md, next's rows kept plus this branch's libc-dev-bin row), and `573030317` names libc-dev-bin's
gencat in the libc6-dbg row. No gencat code change. Proof on keke (Zen 1 host), role n3:
* `util-14-2-lint0` (inherited head 2e25f9305): full clippy -D warnings + `cargo xtask static`: rc 0.
* `util-14-2-gate` (bd99cec16): `cargo xtask affected --run --base origin/next` (direct tier, 6 test targets incl.
  gencat_products, hot_text_products, deb_manifest): 15 suites ok, 1848 passed, 0 failed; peak 4.1G; 2 SKIPPED lines.
* `util-14-2-tests2` (final head 033a496fa): the seven train reds all pass on this host (locale_data x2, s07
  s7_2_f_0, locale_products, catalogs_products, printf_corpus_products, hot_text_products
  init_data_clusters_hold_at_every_text_phase), plus gencat_products and `the_committed_gencat_transcripts_hold`.
* `util-14-2-pkg2` (033a496fa): both flavours build; check.sh PASS (2), check-system.sh PASS (7 packages, the
  libc6-dbg DWARF checks next added included); libc-dev-bin ships ./usr/bin/gencat.
* `util-14-2-final` (033a496fa): full clippy -D warnings + `cargo xtask static`: rc 0.
* Plants `util-14-2-plants` (033a496fa, handoff/UTIL-14-plants.txt): 7 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG (one WARN-DEV-SHM: the RAM dir is under
  /dev/shm, as the brief says for a Zen 1 host; no gencat test mounts its own /dev/shm). Results file:
  handoff/UTIL-14-2-plants.txt. (`util-14-plants3` on 4809e2d1f: 7 CAUGHT.)
* Test fold (coordinator note 10:27Z: tests cost like code): `611826752` moves gencat_products.rs into
  catalogs_products.rs as `mod gencat` (body and skip conditions unchanged; D-31.28's test path, the plants file and
  gencat_lookup.c's comment follow). Proved on that head: gate `util-14-2-gate2`, plants `util-14-2-plants2`,
  clippy+static `util-14-2-final2` (gate2: 15 suites ok, 1852 passed, 0 failed, peak 3.4G; plants2 in the hand-back; the gate, plants and final jobs above are superseded
  by these, tests2/pkg2 stay valid for everything but the moved test file).
* SUPERSEDED: `util-14-2-tests` and `util-14-2-pkg` (bd99cec16, both green) left no tested-content record (their
  command lines did not name the role path, so jobrun could not tell what they tested); re-run as tests2/pkg2.

## Commits

* `88fce63ef` WIP: Add gencat to libc-dev-bin: `csys/util/gencat.c` (C front end, dynamic over libc.so.6, the zic
  precedent), built by `cargo xtask so` (`xtask/src/so.rs`, `target/niji/gencat` and `target/niji-test/gencat`),
  transcript default (`xtask/src/transcript/mod.rs`), manifest row `system libc-dev-bin usr/bin/gencat`.
* `d43bf38a0` Make gencat's catalogs byte-identical to the reference's: rules from the transcripts (below), 38
  transcripts under `validation/transcripts/gencat/`, `xtask/tests/transcripts.rs::the_committed_gencat_transcripts_hold`,
  `crates/core/tests/gencat_products.rs` + `oracle/gencat_lookup.c` (catgets through libniji and the host libc over
  the catalogs of both programs, and byte equality of the two catalogs), D-31.28, docs (tools/deb/system.sh,
  docs/install-packages.md, RELEASE-NOTES).
* `cd937d5ad` Fix gencat test lint and D-31.28 test references (clippy `as` cast; records check wants test files).
* `4809e2d1f` Add a gencat transcript for octal escapes past 0377: the octal-limit plant survived the first full
  plants run (no transcript had an escape value between 0400 and 0777); `octal-range` recorded from the reference,
  libniji equal.

## Proof (keke, Zen 1 host, role n2, head 4809e2d1f)

* `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo xtask static`: clean (records
  included).
* `cargo xtask affected --list`: FULL (escalated: csys/util/gencat.c and xtask/src/so.rs). FULL is the train's, so the
  proof is the changed crates' tests: job `util-14-tests2` (`cargo xtask so && cargo test -p xtask && cargo test -p
  niji --test gencat_products`, under ~/bin/gate 16G): rc 0, peak 1.6G, 7 test binaries all ok,
  `the_committed_gencat_transcripts_hold` ok (39 transcripts), `catalogs_from_both_gencats_are_interchangeable` ok.
  (`util-14-tests` on cd937d5ad: also rc 0, peak 1.7G.)
* Transcripts: 39, every one equal to the reference (catalog bytes compared, not only the exit status and streams).
  No recorded difference.
* catgets equality: 5 sets x 60 messages (escapes, joined lines, padding for collisions), each key looked up through
  libniji's and the host's catgets on both catalogs; the two catalogs are byte-identical.
* Plants (`util-14-plants3` on 4809e2d1f, `cargo xtask plants handoff/UTIL-14-plants.txt`): result in the
  hand-back and JSON. `util-14-plants2` (cd937d5ad): 6 CAUGHT, octal-limit SURVIVED (fixed by 4809e2d1f), stopped
  before symbolic-move ended (7 plants: plane-start, pool-order, octal-limit, merge-skip, header-blank, signed-order,
  symbolic-move). WARN-DEV-SHM printed (the RAM dir is /dev/shm, as the brief prescribes for a Zen 1 host).
* Packages (`util-14-pkg2` on 4809e2d1f; `util-14-pkg` on cd937d5ad: rc 0): both flavours built
  (tools/deb/build.sh, NIJI_DEB_COMMIT + SOURCE_DATE_EPOCH, role dir), check.sh passed, check-system.sh `PASS (7
  packages, version 2.39+niji20261123.1-1)`, `libc-dev-bin: ./usr/bin/gencat` 0755 root/root.

## Probe-settled behaviour

In D-31.28 and handoff/UTIL-14.md (plane search, pool order, symbolic numbering, merge, diagnostics, -H header).

## RULING NEEDED

* Spec App. D.2.2, "`gencat` output" bullet says the pool holds the sets "in descending order". That is true only for
  sources naming their sets in ascending order: the pool holds the sets in the reverse of the order they were first
  named (the default set 1 counting as named first), each set's messages in ascending signed 32-bit order
  (transcripts `set-order`, `default-set`, `default-set-implicit`). Recommend: correct the bullet to that text
  (pending UTIL-SPEC-3 touches this part; the spec was not edited here, under CONTRIBUTING's three-way rule).

## INFORMATIONAL

* Leg 2 issued one Bash call with a here-string (`git commit -F /dev/stdin <<< ""`), against the no-heredoc rule;
  it committed nothing and was not repeated.
* Leg 3's first plants run (`util-14-plants`, started on d43bf38a0) was stopped with kill-label before it ended, to
  sync the lint fix; superseded by `util-14-plants2`, itself superseded by `util-14-plants3` (new transcript).
* Not probed, not needed for the catalogs: the name used for stdin in diagnostics of a parse error.
