# RELEASE-ASSET-NAMES: stage release assets under the names GitHub publishes (issue #3)

**State:** branch `task/release-asset-names`, cut from origin/next; code commit `69ffb8470`, handoff commits on top.

* `tools/deb/stage-release.sh --out DIR [--head SHA] [--note TEXT] FILE.deb...` (was only a job script on keke):
  copies each file under its published name (`~` -> `.`), refuses a name with a character outside
  `A-Za-z0-9._+-` (GitHub rewrites others; `+` is kept as the existing libc6 names show) or two files that collapse to
  one name, writes SHA256SUMS over the staged names and checks it, and writes MANIFEST (staged name, package, Debian
  version; names the build host; not an asset, not in SHA256SUMS). Does not overwrite an existing DIR.
* Test `crates/core/tests/deb_stage_release.rs` (no spec clause): builds fake `~` and plain .debs with dpkg-deb, stages,
  checks names, `sha256sum -c`, the version inside unchanged, MANIFEST, and refusal of a name with a space.
* Plants (`handoff/RELEASE-ASSET-NAMES-plants.txt`): tilde kept, SHA256SUMS listing the `~` name, odd name accepted:
  3 CAUGHT, 0 survived.
* docs/install-packages.md: verify sentence. docs/RELEASE-NOTES.md: the brief's sentence, verbatim, in the 2026-11-15
  section (wrapped across lines).

Jobs on mei `~/w/s1`: `ran-gate` (affected --run, 1 test passed, 0 skipped), `ran-plants`, plus foreground clippy and static.

**Next:** land. Use `tools/deb/stage-release.sh` for the next staging. The published `snapshot-2026-11-15` was fixed by hand already.
