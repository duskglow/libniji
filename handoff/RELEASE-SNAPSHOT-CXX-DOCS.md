COMPLETE: snapshot-ready at 23b9dfdb5

# RELEASE-SNAPSHOT-CXX-DOCS: documentation currency for the snapshot at 23b9dfdb5, and the F1 fix

**State:** branch `task/release-snapshot-cxx-docs`, cut from origin/next `d5e9a8ddf`. Code commit `2d1bea4a0`
(tools/deb/check-system.sh: F1), plants file `979d741da`, then handoff/ only. Validated head unchanged:
`23b9dfdb5`; staged assets unchanged: keke `~/release-staging/snapshot-20261116/` (not restaged, not modified; copies
were used). Role mei `~/w/s2`; job scripts keke and mei `~/work/RELEASE-SNAPSHOT-CXX-DOCS/`.

## F1 fix (step 2)

`tools/deb/check-system.sh`: `./usr/bin/` added to the directory list (after `./usr/lib64/`, before `./usr/sbin/`) and
`libc-bin:./usr/bin/getent` to the file list (after `libc-bin:./usr/bin/getconf`), exactly those two entries. UTIL-LDD-3
may add the same two; if its lines are placed differently, a textual merge conflict results: keep one copy.

| check (final head's tree, `979d741da`) | job | result |
|---|---|---|
| check-system.sh on byte-identical copies of the 6 staged system .debs (`sha256sum -c SHA256SUMS` OK first) | mei `release-snapshot-cxx-docs-checksys-1` | `check: PASS (6 packages, version 2.39+liella20261116.1-1)` |
| the two set-aside plants (rscd-ldconfig-unstripped, rscd-postrm-not-packaged), on the validated head's system-flavour products (copy of keke's, libc.so.6 sha256 6885a175...) | mei `release-snapshot-cxx-docs-plants-1` | baseline ok (check-system passes unplanted); **2 CAUGHT**, 0 survived, 0 WARN |
| gate: cargo clippy --workspace --all-targets --all-features -D warnings, xtask clippy, static, affected --run --base origin/next | mei `release-snapshot-cxx-docs-gate-1` | green; affected selects 0 suites (shell script + plants file: docs-only tier), peak 1.6G |

## Documentation currency (step 1, read-only worktree at 23b9dfdb5)

**(a) Links, commands, scripts in README.md and docs/ (53 files).** A checker (relative markdown links and their
`#anchors`, backticked repo paths, `cargo xtask <sub>` against xtask/src/main.rs's dispatch) found: 238 links, **0
broken**; 197 `cargo xtask` mentions, **0 unknown**; 386 backticked paths, 26 not present, of which 8 are not
references (a proposed path, a glob, a generated dir, a quoted error message, "outside `tests/`"). Stale references
(non-blocking, one INFORMATIONAL):
- `crates/boundary/src/hot.rs` (now the directory `crates/boundary/src/hot/`): docs/design/loadtime-entry.md:23,
  docs/saturation/22-f64-invtrig-hypot-fast-tier.md:4, docs/saturation/24-f64-expm1-log1p-hyperbolic-cbrt-fast-tier.md:5.
- `tools/appendmerge.py` (not in the tree): docs/saturation/00-intro.md:3.
- `handoff/bench-refresh-1023-logs/` (removed): docs/saturation/10-fresh-measurement.md:3,5, 13-next-perf-stories-from-the-10.md:3.
- Generated views (fix in records/, not the view): docs/compat-flags.md `crates/boundary/tests/s29_rpc_reply_source.rs`;
  docs/saturation/07 `crates/libc-so/hot-text.list`; docs/saturation/01-05 name 11 removed handoff notes
  (posix-spawnp-row, realpath-depth-2, scandir-alphasort-deep, fts-path-copy, fts-shared-path, qsort-merge-index,
  str-chr-cmp-short-report, memcpy-policy, strstr-getline, strstr-short-setup, alloc-debug-fold, gettext-memo-hit-cost).

**(b) docs/install-packages.md against the staged assets: works (keke `release-snapshot-cxx-docs-install-2`, rc 0).**
A fresh Debian 13 root (mmdebstrap --mode=unshare, local trixie mirror, with gcc, g++, libc6-dev, binutils, sudo), the
staged directory copied in unchanged, the page followed literally: `sha256sum -c SHA256SUMS` OK (all 10 names);
the published names `liella6_0.0.snapshot20261116+23b9dfdb5880_amd64.deb` and `liella6-dev_...` are listed in
SHA256SUMS; `sudo apt install ./liella6_<version>_amd64.deb ./liella6-dev_<version>_amd64.deb` installs both (no other
package added), installed version `0.0~snapshot20261116+23b9dfdb5880` as the page states (commit's first 12 hex
digits). "What they are": everything under `/usr/lib/liella` except liella-gcc, liella-g++, their manual page and
`/usr/share/doc/liella6*`; the loader at `/usr/lib/liella/lib64/`; all listed libraries and start files present; no
maintainer scripts; `/etc/ld.so.conf` and the cache untouched; no Liella cache. "Building a program": the four example
commands work (dynamic hello runs, `readelf -l` names `/usr/lib/liella/lib64/ld-linux-x86-64.so.2`; static hello
runs; liella-g++ throw/catch + iostream runs; `-shared -fPIC` plugin builds and a program using it runs);
a missing `DT_NEEDED` library prints the documented message with status 127; `-static-pie` and `-pg` are refused.
`apt purge` restores the package set and every file under /etc and /usr. System flavour: the page gives no end-user
install steps for it, only the build/check tools; the 6 staged system .debs are byte-identical to the ones
RELEASE-SNAPSHOT-CXX validated (root-system 15/0, swap 52/0), and check-system now passes on them (above).
Superseded: `release-snapshot-cxx-docs-install-1` (rc 1: my stage dir was under `~/work`, which the namespace's
subordinate root cannot traverse, so apt could not read the keyring: NO_PUBKEY; proof.sh stages in /tmp for this
reason; install-2 does the same).

**(c) User-visible changes since snapshot-2026-11-15 (`git log --oneline bade54e07..23b9dfdb5`, 85 non-merge commits).**

| change | user-facing sentence | verdict |
|---|---|---|
| DL-EXIT-ORDER-PROBE: exit finalisers of start-time and dlopen objects run as one sequence; cycle order (D-27.54, COMPAT-31) | docs/differences-from-glibc.md row "Exit order inside a dependency cycle of loaded objects" (COMPAT-31 "registered, not implemented"); README.md:40 "runs constructors and destructors" | true |
| DL-LD-PRELOAD(-2,-3): LD_PRELOAD and /etc/ld.so.preload at start, secure mode, fakeroot | docs/install-packages.md:65-73; README.md:17, :40; docs/architecture.md:561 | true (separators space/colon and the "cannot be preloaded" message match crates/ld/src/start.rs) |
| DL-LIBSTDCXX(-2..-4): STB_GNU_UNIQUE bound by scope order at start; dlopen's unique table grows; out-of-memory named | none | **missing** (INFORMATIONAL) |
| UTIL-GETCONF (UTIL-5): getconf in libc-bin | README.md:68, docs/toolchain.md:72, docs/install-packages.md:123 | true |
| UTIL-GETENT(-2) (UTIL-6): getent built by `cargo xtask so`, shipped in libc-bin | README.md:68 "utilities shipped so far (`getconf`, `ldconfig`)"; docs/toolchain.md:72 "the utilities `getconf` and `ldconfig`"; docs/install-packages.md:123 libc-bin row lists ldconfig and getconf only | **stale, 3 sentences** (3 INFORMATIONAL) |
| RELEASE-ASSET-NAMES: published names use `.` for `~`, SHA256SUMS over them | docs/install-packages.md:32-34 | true (exercised in (b)) |
| tools/deb: root-system.sh shared-libstdc++ programs; swap-system.sh apt and fakeroot cases | docs/install-packages.md:134-138 | not false (apt and the shared libstdc++ cases are not named) |
| LAYOUT-WINDOW-BINS-7 (start-up fault bounds), SL-HEADERS slices 2-4 (abi-gen tables, Appendix C.12) | none needed: test bounds and spec tables; no headers ship (install-packages.md:36 "no C header set of its own yet", still true) | n/a |
| this snapshot as a whole | docs/RELEASE-NOTES.md's newest section is snapshot-2026-11-15; none for this snapshot | **missing** (INFORMATIONAL) |

The rest are handoffs, records, regenerated views and docs commits (DOCS-CURRENT, the 11-15 notes, testing-filesystems).

## Next step

Coordinator: tag `23b9dfdb5` and publish `snapshot-20261116` as staged; land this branch (check-system then passes
on the published packages). Route the INFORMATIONALs above to a docs story. Do not redo: (b) and the F1 reruns.

## Report

JOBS (rc 0, final head's tree): mei `release-snapshot-cxx-docs-gate-1`, `-plants-1`, `-checksys-1`; keke
`release-snapshot-cxx-docs-install-2`. Superseded: keke `-install-1` (above). Rule slip (mine): one Bash call
contained an empty local heredoc (no effect). Nothing left running.
