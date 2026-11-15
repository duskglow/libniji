COMPLETE: snapshot-ready at bade54e077b7b80fb0c67146fadff89bf3c7d72d

# RELEASE-SNAPSHOT-FINAL: revalidation on the head carrying the F1 fix, assets restaged

**State:** branch `task/release-snapshot-final`, cut from origin/next `26a2a315f`. Code head (the commit to tag)
`bade54e07` "Strip ldconfig in libc-bin and check it"; only handoff/ commits on top (this note, the plants file,
`handoff/reports/RELEASE-SNAPSHOT-FINAL.json`). Role keke `~/w/n1`; job scripts and logs in keke
`~/work/RELEASE-SNAPSHOT-FINAL/` (debs.sh, validate.sh, gate.sh, chain-role.sh, stage.sh, smoke.sh = 1115's,
plants.txt = `handoff/RELEASE-SNAPSHOT-FINAL-plants.txt`); mei `~/work/RELEASE-SNAPSHOT-FINAL/tree` (git archive of
`bade54e07` + keke's release products). tools/deb's scripts were reused unchanged (root-system, swap-system,
swap-cmds, lint, proof, check*).

## Step 1: head

origin/next at start: `26a2a315f`. `git log --oneline 8d38ba722..26a2a315f`: 20 commits. **No product code
changed** (nothing under crates/ or csys/): RELEASE-SNAPSHOT-1115 (tools/deb: the F1 postinst/postrm fix,
root-system and swap-system/swap-cmds, check-symbols exec bit, docs/install-packages.md), SL-HEADERS-1 (header
corpus scan tool validation/headers_corpus.py, results/headers-*, plan/headers.md, one decision record), the
D-X.112 id assignment and the train's view regeneration.

## Step 2: the packaging change

`tools/deb/system.sh` strips libc-bin's `usr/sbin/ldconfig` with the shared libraries' strip options
(`--strip-unneeded --remove-section=.comment --remove-section=.note`); `tools/deb/check-system.sh` fails when the
packaged ldconfig has a `.symtab` or `.debug_*` section, or is missing. Packaged ldconfig: `ELF 64-bit LSB pie
executable, static-pie linked, stripped`; it still runs (postinst wrote the cache, below). Plant
`rsf-ldconfig-unstripped` CAUGHT.

## Step 3: reruns on bade54e07 (keke unless noted) vs RELEASE-SNAPSHOT-1115

| check | job | result | vs 1115 |
|---|---|---|---|
| system build + check-system | debs-1 | `check: PASS (6 packages, version 2.39+liella20261115.2-1)` | same (serial 2, below) |
| check-symbols | debs-1 | `PASS (4135 symbol lines, 10 libraries)`, the four dpkg-shlibdeps samples as in 1115 | same |
| coexist build + check | debs-1 | `check: PASS (2 packages)` | same |
| empty root (6 packages) | roots-1 | `root: RESULT pass=13 fail=0`: hello, threads, except, iostream dynamic via /lib64/ld-linux-x86-64.so.2, static hello, ld.so --list, dpkg states/conffiles/nsswitch/--verify, purge; **`ldconfig -p` lists libc.so.6 after a fresh install (F1 fixed)** | F1 gone |
| swapped trixie root | roots-1 | `swap: RESULT same=47 differ=0`; root's libc.so.6/loader are the package's; cache 54 libs (stock 64) | same |
| lintian --pedantic, trixie and excalibur | lint-1 | **7 E / 45 W** each: E no-code-sections x6, shared-library-is-executable x1 | unstripped E gone; W set identical (one W's postinst line 23 -> 13) |
| coexist proof.sh | lint-1 | trixie pass=21 fail=0 wait=0 known=0; excalibur pass=21 fail=0 wait=0 known=0 | same |
| lua_reload, cxx_dlclose (validation/smoke) | smoke-keke-1, mei smoke-mei-1 | ok 1/1 both sides, 0 Liella-only failures, on keke and mei (same libc.so.6 sha256 c1d01af5...) | same |
| gate (role) | gate-1 | cargo clippy --all-features -D warnings, xtask clippy, static, records check, direct tier `--base origin/next`: 1 passed 0 failed, peak 767M | - |
| plants | plants-1 | 6 CAUGHT (rsf-ldconfig-unstripped + 1115's five, rerun on these products), 0 survived; WARN-DEV-SHM only | - |

No result differs from 1115 except the two intended ones (F1 fixed, unstripped E gone): **no new finding**.

## Step 4: staged assets (not published, no tag)

keke `~/release-staging/snapshot-20261115-final/` (`stage-1`): 8 .debs, SHA256SUMS (`sha256sum -c` OK), MANIFEST
with head `bade54e077b7b80fb0c67146fadff89bf3c7d72d`. SOURCE_DATE_EPOCH 1794749764 (the head commit's time).
**LIELLA_DEB_SERIAL=2**: the 1115 build already staged 2.39+liella20261115.1-1 with different content (F1), so
this build takes the next serial of the same date (owner ruling R2's `<n>`); two different .debs never share
a version.

## Release-notes facts refreshed (only where the head changed them)

| package | version | arch | bytes |
|---|---|---|---|
| libc6 | 2.39+liella20261115.2-1 | amd64 | 2631820 |
| libc-bin | 2.39+liella20261115.2-1 | amd64 | 70088 |
| libc6-dev | 2.39+liella20261115.2-1 | amd64 | 2950980 |
| libc-dev-bin | 2.39+liella20261115.2-1 | amd64 | 1956 |
| locales | 2.39+liella20261115.2-1 | all | 1928 |
| libc-l10n | 2.39+liella20261115.2-1 | all | 1912 |
| liella6 | 0.0~snapshot20261115+bade54e077b7 | amd64 | 2618204 |
| liella6-dev | 0.0~snapshot20261115+bade54e077b7 | amd64 | 2955968 |

* F1 (no /etc/ld.so.cache after a fresh install) is fixed in these assets; drop it from the known issues.
* libc-bin's ldconfig is stripped; lintian is 7 E / 45 W per suite (E: no-code-sections on libc6-dev's
  objects, shared-library-is-executable on the loader).
* Unchanged from 1115 and still worth a line: the swap into a trixie root needs `--force-depends` (trixie's
  util-linux depends on libc6 >= 2.41); dpkg leaves `/var/cache/ldconfig` behind on that swap.

## Jobs

keke: `release-snapshot-final-debs-1`, `-roots-1`, `-lint-1`, `-gate-1`, `-plants-1`, `-smoke-keke-1`,
`-stage-1`; mei: `release-snapshot-final-smoke-mei-1`. All rc 0, all on `bade54e07`. Nothing superseded.

## Next

Coordinator: land, then fast-forward main to `bade54e07` and tag it; publish
`~/release-staging/snapshot-20261115-final/` (the older `snapshot-20261115/` carries F1: do not publish it).
Open follow-up from 1115 (not done here): UTIL-LDCONFIG's duplicate builtin-path message.
