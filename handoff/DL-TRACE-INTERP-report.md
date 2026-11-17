# DL-TRACE-INTERP-2 report (continuation of DL-TRACE-INTERP)

Branch `task/dl-trace-interp-2`, code head `a389fd4e2`. Role keke `~/w/n1`.

## Commits (this run)

* `ff292f6af` Merge origin/next: the libniji rename reaches the branch (CI's old-name red was one `Liella` in
  spec/behaviour/dynlink.md); differences-table conflict resolved (next's rows plus the `ldd -v` row, which gains the
  reference column it lacked); spec note and D-31.15 cite D-27.56 (the id the pending record was given) instead of
  the pending file; `tools/deb/build.sh` system build line split back into `NIJI_LIBDIR=$libdir NIJI_FULL_PROFILE=1`
  (it read `$libdirNIJI_FULL_PROFILE=1`: empty libdir, full profile off).
* `30ff851ec` `list_interpreter_line_matches_reference` also runs `--list` with each side's loader through a symlink
  (`NAME => PATH`, compared whole; the handoff's listed gap).
* `a389fd4e2` `handoff/DL-TRACE-INTERP-2-plants.txt`: the inherited 6 plants plus 2 on the `NAME => PATH` line.

## Jobs (keke, head a389fd4e2 synced as tree a839f042)

* `dl-trace-interp-2-gate-2` (`~/work/DL-TRACE-INTERP-2/gate.sh` under `~/bin/gate 16G`, `set -e`): xtask clippy,
  workspace clippy --all-features -D warnings, xtask static, `cargo xtask affected --run --base origin/next`
  (rc 0: set -e reached the next step; target/affected.log summary at 06:17 read 32 suites ok, 12059 passed, 0 failed, conformance 137/0, but cargo xtask test writes the same file, so those figures may be its own), `cargo xtask test`,
  `NIJI_FULL_PROFILE=1 cargo xtask so` + `cargo test -p niji --test hot_text_products` (the start-up checks: the
  loader is touched). Verdict: running at hand-back; clippy, static and the direct tier passed (set -e reached cargo xtask test).
* `dl-trace-interp-2-plants-1` rc 0: 8 plants, 8 CAUGHT, 0 SURVIVED, 0 WARN.
* `dl-trace-interp-2-debs-2` rc 0: `tools/deb/build.sh --flavour system` of the synced tree, `check-system.sh`:
  `check: PASS (6 packages, version 2.39+niji20261117.1-1)`.
* `dl-trace-interp-2-swap-2` rc 0: `swap: RESULT same=61 differ=2`, ldd-6 and ldd-7 KNOWN (below), commands=63, bad=0.

## Swap root (trixie, unprivileged mmdebstrap; script keke `~/work/DL-TRACE-INTERP-2/swap.sh`)

Verdict rule (stricter than UTIL-LDD-3's): every command, ldd-1..5 included, identical with no normalisation at
all (no path rewriting, no interpreter-line rewriting); ldd-6 and ldd-7 (`ldd -v`) identical once only the version-need
blocks of libc.so.6 and libm.so.6 are dropped on both sides (the documented `ldd -v` row). UTIL-LDD-3 had same=54
differ=7 with (a)-(c) normalised; this branch: swap-1 `swap: RESULT same=61 differ=2` (ldd-6, ldd-7: libc.so.6's block
naming the loader's versions, and libm.so.6's). swap-1's own verdict step stopped on a stale result directory name
(`liella`, renamed `niji`); swap-2 reran with the name fixed.

## Probe-settled behaviour

Unchanged from DL-TRACE-INTERP (spec/behaviour/dynlink.md "Trace lists"). New this run: running the reference loader
through a symlink prints `PT_INTERP => <link path as run>`, and libniji's loader prints the same (the test's new case
passes on both sides; plants `trace-arrow-spelling`, `trace-arrow-path-as-run` CAUGHT).

## Superseded

`dl-trace-interp-2-lint-1` (rc 0, merge head ff292f6, before the test commit), `-gate-1` and `-debs-1` (stopped by
me: the head changed), `-check-1` (rc 0, diagnostic: the list tests on 30ff851), `-swap-1` (rc 1: verdict script's
directory name; the comparison itself was same=61 differ=2).

## Rule slips

None known. Refusals: none.
