# XTASK-STALE-PRODUCTS: xtask so leaves no stale products; leftover test links skip with a reason

Branch `task/xtask-stale-products`, code head 86e299f64 (handoff commits on top). Base origin/next 0495c4684.

## The cleanup rule as built (brief item 1: the removal option)

`cargo xtask so` owns `target/niji` except the directories other commands write (`so::OTHER_COMMANDS`: `bench/`
from `cargo xtask bench`, `native/` from native-examples, `rust/` and `rust-test/` from rust-dylib). Its first step
(`so::begin_products`, before an import or a build) removes every other entry there (files, links, directories,
dot files) and remembers the names; at the end it prints
`so: removed N earlier product(s) no step of this tree makes: <names>` for those it did not make again.

Why removal rather than a record the lint reads: the lint, the products manifest (`.products-manifest`, which an
import trusts), the layout verdict key and every test read the directory listing; a record would have to be read by
all of them, and the listing would still lie. Removal makes the listing true for every reader. It covers the
import path too (an import only moves its listed files in, so a stale file there survived an import before).
No step skips a product it finds already there (checked: no existence test on a product in so.rs; the placeholders
record, the libm digest, the markers are all rewritten each build), so nothing is rebuilt: measured below.
A story that adds a product needs nothing new; one that drops a product gets it removed. A new command that writes
its own top-level directory into `target/niji` must add it to `OTHER_COMMANDS` (or its output is removed by the
next `xtask so` and rebuilt by that command).

## Test links (brief item 2)

`xtask/tests/test_link/mod.rs` (shared): `loads()` reads an ELF's `PT_INTERP` and its `DT_NEEDED` entries naming a
path; `usable()` returns the link only when it and all of those exist, else the reason
(`... is left from an earlier build: it loads <path>, which is gone`), printed as `SKIPPED:`.
Checked tests in xtask/tests: transcripts.rs (iconv link: now checked; ldconfig/localedef: static, no interpreter
or loaded product, existence is enough, and a stale one is now removed by `xtask so`), tz_tools.rs (zic/zdump links:
now checked), affected_mode.rs, probe_client.rs, header_p2_fixed_rows.rs (use no target/niji-test file).

## Proof (keke, Zen 1 host)

* Tests: `so::tests::a_product_build_leaves_no_earlier_product_for_the_manifest_lint` (fixture: stale gencat,
  link, header, charmap, markers, the other commands' dirs; the real products linked in as the build; manifest lint
  OK), `transcripts::a_leftover_iconv_link_whose_loader_is_gone_is_skipped` (copy of the iconv link with its
  interpreter renamed to a missing `no-linux-x86-64.so.2`: skipped with that path). Job xtask-stale-products-test3:
  clippy -D warnings clean, so unit test ok, transcripts 8/8, tz_tools 10/10.
* Real build (job xtask-stale-products-so-after): `cp getent target/niji/gencat; cargo xtask so` printed
  `so: removed 1 earlier product(s) no step of this tree makes: gencat`; gencat gone afterwards. (That job ended rc 101 afterwards on a wrong `cargo test --manifest-path` call I added; SUPERSEDED, the deb_manifest test ran in the gate.)
* Incremental build, nothing changed: before 338.9 s (job xtask-stale-products-so-base, second run), after 339.6 s
  (so-after): no rebuild added (difference within run-to-run noise; wall time on a shared host).
* Plants (handoff/XTASK-STALE-PRODUCTS-plants.txt): (a) cleanup skipped -> lint names gencat; (b) link-loads check
  disabled -> the leftover link runs and fails with ENOENT. Job xtask-stale-products-plants.
* Gate: xtask-stale-products-gate (`cargo xtask affected --run --base origin/next` + `cargo test -p niji --test
  deb_manifest`).

## Open / informational

* `xtask/src/transcript/mod.rs` `niji_default` still takes the getent/locale/iconv test links when they are files;
  `cargo xtask transcript check` run by hand on a leftover link fails with ENOENT. Not changed (outside xtask/tests;
  the tests now skip before calling it). A follow-up could move `test_link::usable` into xtask and use it there.

Next step: none beyond landing. Read: this file; `so::begin_products`, `so::clear_products`.
