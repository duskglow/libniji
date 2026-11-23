# CI-PRODUCTS-ONCE: build each train's release products once; make the traced layout cheaper (handoff, final)

State: branch `task/ci-products-once`, base origin/next fa79409c7. Role mei:~/w/s2 (Skylake-SP, 6-core slot); the
Zen 1 checks ran on keke in a copy at ~/work/ci-products-once/t1 (no CI role path used). Records: D-27.82 (manifest
and import), D-27.83 (the tracer's breakpoint pool). Plants: handoff/CI-PRODUCTS-ONCE-plants.txt.

## Commits

* 5c5d867d4 tools/hottext/trace_gdb.py: one-shot breakpoints made once per tracer (pool), with two self-checks.
* 9609ba1bc xtask: `target/niji/.products-manifest` written by every product build; `NIJI_IMPORT_PRODUCTS=<dir>`.
* 7edb8813b, 3aef96204, 1d912a6c1: the build-or-import step under test, records D-27.82/D-27.83, the plants file.

Proof on code head 1d912a6c1 (mei): ci-products-once-gate3 (records check, static, `cargo xtask affected --run --base
origin/next`: 7 suites ok, 642 passed, 0 failed; the xtask bounded set, FULL: the train's) and ci-products-once-plants2
(10 plants). SUPERSEDED: ci-products-once-gate (stale tree, records check), gate2 and plants (10/10 CAUGHT) on
3aef96204/7edb8813b (before the D-NEW comment fix), lint (record body newline).

## Step 1: the traced layout (Skylake-SP, release, `NIJI_FULL_PROFILE=1 cargo xtask so`)

| run | head | so/libc-archive | of it cargo-libc | regen (the 25 traces) | so/total |
|---|---|---|---|---|---|
| before (per-run breakpoints) | fa79409c7 | 387.5 s | 108.5 s | 238 s wall, 1232 s gdb CPU | 658.4 s |
| after (pool), cargo warm | fa79409c7 | 114.3 s | 0.0 s | 127 s wall, 531 s gdb CPU (standalone) | 145.6 s |

Like for like (without cargo): 279 s -> 114 s. One trace 58.5 -> 15.9 s. Where it went: making and deleting the 4421
one-shot breakpoints in every class run (6-14 s per run; gdb's cost per breakpoint grows with the count it holds).
On the Zen 1 host (copy, cold, 4 cores of a loaded host) the pooled build's libc-archive was 501.8 s with 205.5 s of
cargo (ci-instr's recent runs: 588-804 s); not a controlled comparison.
Identity of the output with the pool: head fa79409c7, all 1256 product files and the 5 lists byte-identical to the
per-run build (sha256 listing sums.f1 = sums.base1, digest of the listing 1e02ed63e8ec1d78; hot-text.list 4752d158931184ae,
startup-text.keys 54a7859b7540fdb7, probe-text.keys 8acded21f2d32e4b, hot-text.signatures 0954ec032fb1c30d,
startup-data.keys 2d9ca170f108b244). Head 90390071c: the pooled re-trace of its products gives all 5 lists identical
to the per-run build's (the archive is a function of sources and lists).

## Step 2: cross-host identity (Skylake-SP mei vs Zen 1 keke)

| head | files | result |
|---|---|---|
| 0299fd6e1 -release, -release-dyn (primed) | 4 + 13 | identical |
| 34cc0ded1, 854da9f38, b2485a7f1, d53c0d551 -release (primed) | 4 each | identical |
| 90390071c (mei cold build vs keke primed) | libc.a, libm.a, crt1/crti/crtn, libc.so.6, libm.so.6, ld.so, libc_nonshared.a | identical (cf6625752590b12a, 585ac2b4793a1e08, ... ) |
| 9609ba1bc (mei role vs keke copy, full target/niji) | 1259 entries | 1240 identical; 19 differ |

What differs and why: getconf, getent, iconv, ldd, locale, zdump, zic, libanl.so.1, libBrokenLocale.{a,so.1},
libc_malloc_debug.so.0, libdl.so.2, libmvec.{a,so.1}, libnsl.so.1, libpthread.so.0, libresolv.so.2, librt.so.1,
libutil.so.1, and the `.hot-lists` marker. Each carries the build tree's path (`~/w/s2` against the copy's path) in
DWARF only (`.debug_line_str` / `.debug_str`: the compile directory), never in a loaded section. `.hot-lists` names
the lists directory by path; its list hashes are equal. The primed sets agree because both hosts build them at the
same path. libc.a, libc.so.6, libm.a, libm.so.6, ld.so, the crt files and libc_nonshared.a match on all heads.

## Step 3: the import (D-27.82)

Manifest: tag, head, toolchain (all `rustc -vV` lines, `cargo -V`, the first line of cc, ld, as, gdb `--version`, 8
build variables), profile + layout (`release traced`), sha256 of every file and target of every link outside bench/.
Written last (0.4 s). Import refusal cases, each a unit test and a CAUGHT plant: tag, head, head unknown or dirty,
toolchain, profile, product checksum, unlisted/missing product, and the fallback (a refused import builds here).
A missing or unreadable manifest refuses too. Zen 1 run on 9609ba1bc with the Skylake-SP products: `so: imported 1259
products ... every field and checksum matched`, so/total 1.0 s (the local build: 840.2 s).

## Step 4: bench rows, imported vs local (keke, `cargo xtask bench --counters-only --only cbench_memref,cbench_hot,cbench_startup`)

180 rows each, same set. Per-call counts are not exactly reproducible between runs of the same binaries:
the glibc column (identical in both runs) differs in 91 of 180 rows, by up to 2.69% (putenv_2000).
Local run 1 vs import, libniji column: 65 of 180 differ, max 1.69% relative; local run 2 vs import: 75 differ, max
5.99% (startup_kinstr_c). Local 1 vs local 2 (same build, same tree): 72 differ, max 5.54% (startup_kinstr_c); glibc
column 85 differ. So the imported products move no row beyond the noise of two local runs. Row-for-row equality, as
the brief asked, cannot be shown with these counts: two runs of one build are not equal either. The code is equal by
checksum (step 2: every file with loaded code is byte-identical), which is the stronger proof.
Scripts: ~/work/ci-products-once/k.sh on keke; counters.local1/local/import.tsv beside it.

## Request to the coordinator (the job that starts ci-instr; RULING NEEDED, a cache trust boundary)

Build the train's release products first on the Skylake-SP host (`NIJI_FULL_PROFILE=1 cargo xtask so` in the gate
role, before the full gate; about 2.5 min warm with the pool), then copy that role's `target/niji/` WITHOUT `bench/`
(tar or rsync; the manifest `.products-manifest` included) to a fresh directory on the Zen 1 host and start ci-instr
with `NIJI_IMPORT_PRODUCTS=<that dir>`. Its log then says `so: imported N products` or one `so: import refused:` line
per reason and builds as today. Known effect: the 19 files above carry the gate role's path in their debug info.

## Open / next

* Fix-forward (recommend a story): path-independent debug info (a prefix map for the C and assembler steps of the
  utilities and small libraries), so every product is byte-identical whatever the tree path.
* The probes are compiled one after another between trace starts in regen.sh (up to 44 s each under load) and the
  first class run still makes its breakpoints (10.7 s): candidates, not tried.
* INFORMATIONAL: `layout: verdict cache: could not store .../target/layout-verdicts/<hash> (an ok line holds a
  newline)` on every release build in the role.
* INFORMATIONAL: a role's `.synced` has no `dirty=` line after a sync of uncommitted changes (tree=<sha>-<hash>),
  so `commit_of` reports the head as clean; the tag still covers the sources, so an import of such products would
  need the same sources anyway.
