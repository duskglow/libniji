COMPLETE: snapshot-ready at 8d38ba7226af22a1078676531f6bcaa9cd88f40a (one non-blocking (a) finding, fixed on this branch: RULING NEEDED below)

# RELEASE-SNAPSHOT-1115: final validation of the snapshot head and its staged assets

**State:** branch `task/release-snapshot-1115` (from origin/next = the snapshot head `8d38ba722`), code head
`95a646fc8`; this note and `handoff/reports/RELEASE-SNAPSHOT-1115.json` on top. Role keke `~/w/n1`; scripts and
logs in keke `~/work/RELEASE-SNAPSHOT-1115/` (mei: same directory, smoke only). Pending record
`records/pending/RELEASE-SNAPSHOT-1115-1.toml` (D-NEW, series D-X, amends D-X.106). Plants
`handoff/RELEASE-SNAPSHOT-1115-plants.txt`.

## What this branch changes (tools only; no product code)

* `tools/deb/root-system.sh`: the empty root now builds hello, threads (C) and except, iostream (C++, libstdc++
  static) with plain gcc/g++ against the packaged libc6-dev and requires each to run through
  `/lib64/ld-linux-x86-64.so.2`; the DL-8 WAIT branch is gone (a failure is FAIL).
* `tools/deb/swap-system.sh` + `swap-cmds.sh` (new): Liella's libc6 + libc-bin swapped into a trixie root (dpkg
  `--force-downgrade --force-depends`, the pinned downgrade of plan/system-libc.md §3.7), proof that the root's
  libc.so.6 and loader are the package's (sha256), and 47 commands of the distribution's own programs compared
  (output and status) with a stock trixie root.
* Fix, finding F1: libc-bin's postinst runs ldconfig on `configure` too; new postrm removes `/etc/ld.so.cache` on
  purge; check-system expects the postrm. `check-symbols.sh` gets its executable bit (it exited 126 as a program).

## Step 3 results (snapshot products, keke)

Empty root, only Liella's six system packages (`release-snapshot-1115-roots-1` on the snapshot .debs; `-final-1`
on this branch's packaging of the same products):

| program | how | snapshot .debs | branch packaging |
|---|---|---|---|
| hello (dynamic) | PT_INTERP /lib64/ld-linux-x86-64.so.2, libc.so.6 by soname | PASS | PASS |
| threads (dynamic) | same, 40000 increments + TLS | PASS | PASS |
| except (dynamic C++) | same + libm.so.6 by soname | PASS | PASS |
| iostream (dynamic C++) | same | PASS | PASS |
| static hello, `ld.so --list`, dpkg states, conffiles, nsswitch, `dpkg --verify`, purge | | PASS | PASS |
| `ldconfig -p` lists libc.so.6 | cache from install | **FAIL (F1)** | PASS |

Trixie root, Liella swapped in vs stock trixie (libc6 2.41-12+deb13u4), `swap: RESULT same=47 differ=0` in both
jobs; the swapped root's libc.so.6 and ld-linux-x86-64.so.2 are byte-identical to the package's:

| program | commands | same | notes |
|---|---|---|---|
| ls | 5 | 5 | `-l` owner names (NSS files), `-aR`, `--sort=size`, missing path (status 2) |
| sort | 5 | 5 | keys, `-n`, `-f -u`, `-c` (status 1) |
| grep | 7 | 7 | `-n -c -i -E -o -w -v -F -r`, no match (status 1) |
| sed | 5 | 5 | BRE groups, `-n` with `=`, `-E`, `y`, hold space, syntax error |
| awk (mawk) | 5 | 5 | printf `%f %e %d`, sqrt/exp/sin/log/atan2, split, pipe to sort |
| bash | 4 | 4 | arrays, `printf %q`, `[[ ]]`, traps/signals, background job + wait, pipefail, exit 7 |
| perl | 5 | 5 | sort, POSIX strftime/floor/fmod, `%e %g` incl. inf, fork/waitpid, getpwuid, die (255) |
| python3 | 6 | 6 | math/json/struct, strftime, hashlib, subprocess, threads, pwd/locale, exceptions |
| find | 5 | 5 | `-printf`, `-newer`, `-empty`, `-exec`, missing path (status 1) |

validation/smoke loader rounds on this head (products built on keke, `LIELLA_FULL_PROFILE=1 cargo xtask so`;
mei ran the same products from a copy of the head): `lua_reload` and `cxx_dlclose` (thread_local plug-in) **ok,
1/1, identical output, on keke and mei** (`release-snapshot-1115-smoke-keke-1`, mei `-smoke-mei-1`). F13 is gone.

## Step 2 results (packages, snapshot head)

* `release-snapshot-1115-debs-1`: system build + `check-system: PASS (6 packages, 2.39+liella20261115.1-1)`;
  coexist build + `check: PASS (2 packages)`. Its rc 1 is only check-symbols.sh's missing exec bit (rc 126).
  `release-snapshot-1115-symbols-1` on the same .debs: `check-symbols: PASS` (dpkg-gensymbols -c4 clean, 4135
  symbol lines, 10 libraries, the four dpkg-shlibdeps samples as expected).
* `release-snapshot-1115-lint-1`: coexist `proof.sh` **trixie pass=21 fail=0 wait=0 known=0, excalibur pass=21
  fail=0 wait=0 known=0** (LAN mirrors, pinned keyrings; the dynamic smoke programs and static iostream now pass:
  no WAIT/KNOWN left). lintian `--pedantic` on the system .debs, both suites: **8 E / 45 W**: E `no-code-sections` x6
  and `shared-library-is-executable` (as SL-FLAVOUR), **new E `unstripped-binary-or-object [usr/sbin/ldconfig]`**;
  `no-shlibs` is gone (SL-SYMBOLS); new W on libc-bin: `no-manual-page`, `maintscript-calls-ldconfig`,
  `command-with-path-in-maintainer-script` (UTIL-LDCONFIG). Logs keke `~/work/RELEASE-SNAPSHOT-1115/logs-lint/`.

## Step 4: assets (staged, not published)

keke `~/release-staging/snapshot-20261115/` (`release-snapshot-1115-stage-1`): 8 .debs, `SHA256SUMS`
(`sha256sum -c` OK), `MANIFEST`. Built on keke from head 8d38ba722, SOURCE_DATE_EPOCH 1794742515:

| package | version | arch | bytes |
|---|---|---|---|
| libc6 | 2.39+liella20261115.1-1 | amd64 | 2631360 |
| libc-bin | 2.39+liella20261115.1-1 | amd64 | 75180 |
| libc6-dev | 2.39+liella20261115.1-1 | amd64 | 2949360 |
| libc-dev-bin | 2.39+liella20261115.1-1 | amd64 | 1956 |
| locales | 2.39+liella20261115.1-1 | all | 1924 |
| libc-l10n | 2.39+liella20261115.1-1 | all | 1916 |
| liella6 | 0.0~snapshot20261115+8d38ba7226af | amd64 | 2617740 |
| liella6-dev | 0.0~snapshot20261115+8d38ba7226af | amd64 | 2954156 |

These carry F1 (they are the snapshot head's packaging). Nothing went to GitHub; nothing tagged.

## Findings

* **F1 (a), fixed here, RULING NEEDED (blocking or not):** on the snapshot .debs a fresh install leaves no
  `/etc/ld.so.cache`. Reproducer: `tools/deb/root-system.sh <snapshot system debs>` -> `root: FAIL ldconfig -p ...
  Can't open cache file`; `dpkg --root -i` prints no trigger processing. Cause: deb-triggers(5) drops an activation
  while the interested package (libc-bin) is unconfigured, and the postinst ran ldconfig only when triggered.
  Effect: no cache until the next library install triggers ldconfig; the loader searches the default directories,
  so every program above runs. **Recommend: not blocking**; tag 8d38ba722 with F1 in the notes, or repack the
  same products with this branch's packaging after it lands (`build.sh --flavour system --products`, packaging only).
* (b) swap needs `--force-depends`: trixie's util-linux depends on `libc6 (>= 2.41)` (plan §4, SCHED-ATTR).
* INFORMATIONAL: lintian E `unstripped-binary-or-object` for libc-bin's ldconfig (strip it in system.sh or
  justify); Liella's ldconfig, run by the postinst with `-r` in a merged-/usr root, prints
  ``Path `/usr/lib' given more than once (from <builtin>:0 and <builtin>:0)`` (UTIL-LDCONFIG to compare with a
  reference transcript); the swapped root's cache lists 54 libraries vs stock 64 (Liella's libc6 ships fewer, plan
  §2); dpkg leaves glibc's `/var/cache/ldconfig` behind on the swap (not owned by Liella's libc-bin).

## Jobs (keke unless noted)

Final head: `release-snapshot-1115-gate-1` (cargo clippy --all-features -D warnings, xtask clippy, static, records
check, direct tier `--base origin/next`: **425 passed, 0 failed**, peak 1.7G), `-plants-1` (**5 CAUGHT**, 0 survived; WARN-DEV-SHM only), `-final-1`,
`-lint-1`, `-symbols-1`, `-stage-1`, `-smoke-keke-1`, mei `release-snapshot-1115-smoke-mei-1`.
Evidence on the snapshot .debs (findings, not final-head proofs): `-roots-1` (rc 1: F1), `-debs-1` (rc 1: exec
bit), `-ldprobe-1` (the F1 probe).

## Next

Owner: F1 ruling, then the tag. Follow-ups: strip ldconfig in libc-bin; UTIL-LDCONFIG's duplicate builtin path
message. Rule slip (mine): one empty Bash heredoc call (no effect).
