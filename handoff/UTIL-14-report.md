# UTIL-14 report: gencat in libc-dev-bin (GitHub issue #12)

Branch `task/util-14` from origin/next `b04f71229`. Code head `4809e2d1f` (the commits after it are handoff/ only).

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
