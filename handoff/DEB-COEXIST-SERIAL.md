# DEB-COEXIST-SERIAL: snapshot serial in the coexist package version

State: branch task/deb-coexist-serial, complete.

Done: `tools/deb/version.sh` makes both flavours' versions; coexist is now `0.0~snapshot<YYYYMMDD>.<n>+<12-hex commit>`,
`<n>` = NIJI_DEB_SERIAL (default 1, same input as the system flavour; a positive number without leading zeros, else
refused). build.sh calls it. `.` sorts above the `+` of the serial-less versions already published, so every new version
sorts above all of them (tested with `dpkg --compare-versions`). docs/install-packages.md shows the new pattern.

Tests: crates/core/tests/deb_version_order.rs (3 tests; no spec clause, release packaging). Plants:
handoff/DEB-COEXIST-SERIAL-plants.txt, 4 of 4 CAUGHT, no WARN lines.

Gate: `cargo xtask affected --base origin/next --run`: 6 suites ok, 562 passed, 0 failed. clippy and static clean.

Next step: the snapshot release script must set NIJI_DEB_SERIAL to the snapshot's serial (the .2 of snapshot-2026-11-20.2)
for the coexist build too; build.sh defaults to 1, so a second snapshot of a day built without it gets `.1` again.
Not done here (not in the brief): any release tooling that calls build.sh.
