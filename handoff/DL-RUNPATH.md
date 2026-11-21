# DL-RUNPATH: dlopen searches the program's and the caller's DT_RUNPATH/DT_RPATH `$ORIGIN`; RTLD_NOLOAD by loaded names

State: branch `task/dl-runpath-4` (DL-RUNPATH-3's branch continued; origin/next 97bf3542f merged in DL-RUNPATH-4).
Head and job results: handoff/reports/DL-RUNPATH-4.json and the hand-back. DL-RUNPATH-4's work is in
"Run state (DL-RUNPATH-4)" at the end.

## Premise and what the probes found (reference = the build host's glibc 2.39, black box; Zen 1 host)

The brief's item 1 ("the loader searches neither DT_RUNPATH nor DT_RPATH of the program") no longer held as stated:
the start's search (link.rs) and dlopen's search (`search::search`, chain = caller then program) already
implement the ld.so(8) order, and an absolute DT_RUNPATH served dlopen. What failed was every **`$ORIGIN`**
element seen by dlopen, for two reasons:

1. The program has no path in dlopen's loader, so its `$ORIGIN` was `None` (DL-NAME-PATH-2's finding).
2. Start-time objects got their path at the first dlopen from `keep_start_paths`, which searched each one again:
   it skipped objects without DT_SONAME, and a library found through the program's `$ORIGIN` could not be found
   again (cause 1), so such a library had no `$ORIGIN` either (its own DT_RUNPATH `$ORIGIN/cdep` failed).

Probe corpus (scratch, not in the tree; the same layout is in `crates/ld/tests/runpath_search.rs`): programs with
DT_RUNPATH / DT_RPATH `$ORIGIN/lib:$ORIGIN/sub`, libcall.so (DT_RUNPATH `$ORIGIN/cdep`), libplain.so (no lists).

| case | reference | libniji before | after |
|---|---|---|---|
| program dlopen("libB.so"), RUNPATH or RPATH `$ORIGIN/sub` | P/bin/sub/libB.so | not found | same as reference |
| program dlopen("libB.so"), absolute RUNPATH | found | found | found |
| program dlopen("$ORIGIN/sub/libB.so") | P/bin/sub/libB.so | not found | same |
| libcall dlopen("libD.so") / ("$ORIGIN/cdep/libD.so") via its RUNPATH | P/bin/lib/cdep/libD.so | not found | same |
| libcall (has RUNPATH) dlopen("libB.so"), program RUNPATH or RPATH | not found | not found | same |
| libplain (no lists) dlopen("libB.so"): program RPATH / RUNPATH | found / not found | not found / not found | same as reference |
| program dlopen("libE.so") needing libF.so (both in sub): RUNPATH / RPATH | "libF.so: cannot open" / found | not found | same as reference |
| RTLD_NOLOAD "libA.so" (start-time, no DT_SONAME), "libC.so" (only libcall's RUNPATH reaches it) | handles | NULL | same |
| RTLD_NOLOAD ld.so by PT_INTERP path / soname / another path to its file | handle / handle / NULL, no error | NULL / handle / NULL | same as reference |
| dlsym(RTLD_DEFAULT, "_r_debug"), program not naming ld.so | ld.so | ld.so (D-27.65 already) | same |
| RTLD_DI_SERINFO of a `$ORIGIN` program | P/bin/lib, P/bin/sub, ... | `$ORIGIN` entries missing | listed |
| LD_LIBRARY_PATH before RUNPATH; RPATH before LD_LIBRARY_PATH (SERINFO order) | as ld.so(8) | same | same |

ld.so's global-scope position: with a level-3 dependency defining `_r_debug`, the reference binds ld.so's (ld.so
sits where libc.so.6's dependency would be, before deeper objects); libniji's ld.so joins the global scope last
(D-27.65). Probe output matched for the cases run; the level-3 case is in the scratch probe only.

## What changed

- `link.rs`: `program_origin` (cold) holds the start's `$ORIGIN` rule; `Origin::dir` uses it and publishes the
  kept directory in `state::START_ORIGIN`. `entry.rs` records the `ld.so PROGRAM` path in `START_RUN_PATH` (one
  store, only under `ld.so PROGRAM`). Nothing new runs for a program without tokens.
- `dlopen.rs`: `Loader.origin` (the program's `$ORIGIN`, published once). `look_up_origin` (cold) takes the start's
  copy, else applies `program_origin`. It runs when the loader is made if the program's DT_RPATH/DT_RUNPATH hold a
  token, or in `load_set` for a program's `$ORIGIN` name or an LD_LIBRARY_PATH token. `scope_of(l, 0)` and dlinfo's
  `origin(v, 0)` read it. `find`: a bare name also matches an object without DT_SONAME by the name it was loaded
  under (`by_given`); a path equal to ld.so's link-map name (PT_INTERP) is ld.so.
- `dladdr.rs::keep_start_paths`: a start-time link map's name (the found path, DL-NAME-PATH-2) is the object's
  path; the re-search stays as the fallback. `given_name`: a start-time object without DT_SONAME takes the bare
  DT_NEEDED name its path ends in.
- Tests: `crates/ld/tests/runpath_search.rs` (4). Plants: handoff/DL-RUNPATH-plants.txt.

## Secure mode (ruling #7190, accepted: one rule everywhere, #7094)

dlopen expands the program's `$ORIGIN` in a secure process only when it is a trusted directory, through the same
`search::expand` rule the start uses (dlopen's `Env::ctx` passes `AT_SECURE`). DL-RUNPATH-2 adds the test the ruling
asked for, `secure_dlopen_origin_only_trusted` (crates/ld/tests/runpath_search.rs), with no root:

- forced flag: the test program sets `AT_SECURE` to 1 in its own auxiliary vector, which is where dlopen reads it. Then
  `libB.so` (via DT_RUNPATH `$ORIGIN/sub`) and `$ORIGIN/sub/libB.so` give NULL when the program sits in the corpus
  (untrusted), and both are found when the corpus is bound over a default directory (`/usr/lib/x86_64-linux-gnu` on
  the Skylake-SP host) in `unshare -r -m`. The same program with the flag left alone finds both in the corpus.
- the kernel's flag: a set-group-ID copy (a supplementary group) must also refuse the untrusted `$ORIGIN`.
- All three parts ran on the Skylake-SP host (no SKIP lines). The plant `no-secure-rule` drops the flag from dlopen's
  search context.

Set-user-ID probe for the owner (needs root; nothing here was run as root). Build as the user, in a scratch dir D that
is not on a nosuid file system: the SECURE program of runpath_search.rs, linked against the system C library
(`cc -o probe secure.c -Wl,--enable-new-dtags,-rpath,'$ORIGIN/niji-origin-sub'`), a libniji copy linked as in the
test with the same RUNPATH, and `cc -shared -fPIC -o libB.so b.c`, where b.c holds `int libb(void){return 2;}`.
Then, as root:
1. `install -d D/u/niji-origin-sub && install -m 644 D/libB.so D/u/niji-origin-sub/ && install -o root -m 4755 D/probe D/u/probe`
   (untrusted, set-user-ID). As the user: `D/u/probe keep libB.so '$ORIGIN/niji-origin-sub/libB.so'`. Expected:
   `secure 1`, both NULL.
2. `install -d /usr/lib/x86_64-linux-gnu/niji-origin-sub && install -m 644 D/libB.so /usr/lib/x86_64-linux-gnu/niji-origin-sub/ && install -o root -m 4755 D/probe /usr/lib/x86_64-linux-gnu/niji-origin-probe`
   (trusted). As the user: `/usr/lib/x86_64-linux-gnu/niji-origin-probe keep libB.so '$ORIGIN/niji-origin-sub/libB.so'`.
   Expected: `secure 1`, both `/usr/lib/x86_64-linux-gnu/niji-origin-sub/libB.so`.
3. Repeat 1 and 2 with the libniji copy, then compare the outputs. Clean up:
   `rm -rf D/u /usr/lib/x86_64-linux-gnu/niji-origin-sub /usr/lib/x86_64-linux-gnu/niji-origin-probe`.

## Open (route)

- RESOLVED by ruling #7280 (done in DL-RUNPATH-3: the differences row and COMPAT-37, registration only; no
  development-build fallback). The finding (coordinator note, DL-PROCESS-OBJECTS-2 #7265): a program
  with DT_RPATH or DT_RUNPATH `<dir>` whose DT_NEEDED names `libc.so.6` by soname. Probed on the Skylake-SP host
  (scratch ~/work/dl-runpath-2/pin.sh, job dl-runpath-2-pin): the reference maps `<dir>/libc.so.6` for both tags.
  libniji's loader (this head's products, `NIJI_PREFIX` unset) opens only `/lib/libc.so.6` and stops with
  `libc.so.6: cannot open shared object file`. This is the pin (`search::PINNED`, R14-3 (a), threat-model rule 9:
  libniji's own sonames resolve to `<prefix>/<libdir>` before any search), not the search. Naming libc by path
  bypasses the pin, which is why that works. The pin is in spec/security/threat-model.md but has no row in
  docs/differences-from-glibc.md. Recommend: keep the pin and add the row. Decide separately whether a development
  build (products outside the prefix) should get a fallback.

- INFORMATIONAL: `$PLATFORM` is `haswell` on an x86-64-v3 CPU (D-27.16, probed on Haswell); on the Zen 1 host the
  reference expands it to `x86_64`. A loader story should probe both CPU kinds.
- INFORMATIONAL: start-time objects without DT_SONAME are still left out of a handle's local scope
  (`adopt_deps` matches by soname or path only). `given_name` now has the name, so a follow-up can match it.
- Not probed on the reference: dlopen through an LD_LIBRARY_PATH `$ORIGIN` list (the start's case was probed by
  DL-NAME-PATH-2). The test expects the start's rule.

## Run state (DL-RUNPATH-2, branch `task/dl-runpath-2`)

- origin/next e4d7011e6 merged (a0abf5f3b). The one conflict was dlopen.rs's imports: next's `Arena` kept along with
  `START_ORIGIN` / `START_RUN_PATH`. The DL-RUNPATH code uses the loader's arena accessors (`path`, `keep`,
  `objs_upto`) and indexes no old array. The COMPAT-19 placement is as next has it (entry.rs and link.rs auto-merged).
- Code head e6047c2f1 (the test and handoff/DL-RUNPATH-2-plants.txt: DL-RUNPATH's 7, `no-secure-rule`, and
  DL-PROCESS-OBJECTS' 5). Step 0 (clippy + static on the inherited head): green. `dl-runpath-2-check1` (clippy, so,
  runpath_search 5/5, dlopen_process_objects 2/2): green.
- Jobs on the Skylake-SP role: `dl-runpath-2-gate` (direct tier, refuses FULL), `dl-runpath-2-plants`,
  `dl-runpath-2-y10` (system-flavour packages of this head, then RELEASE-SNAPSHOT-1121C's y10.sh; the bare-name
  `$ORIGIN` dlopen line was its one FAIL), `dl-runpath-2-startup` (after the gate: `cargo xtask test`, full-profile
  so + hot_text_products). Results: handoff/reports/DL-RUNPATH-2.json and the hand-back.

## Run state (DL-RUNPATH-3, branch `task/dl-runpath-3`, Zen 1 host)

- Step 0 on the inherited head 6c015da40 (clippy + static, `dl-runpath-3-step0`): green.
- DL-RUNPATH-2's `dl-runpath-2-startup` (rc 124) was its timeout at test 140 of 6993 on a saturated Skylake-SP host,
  not a failure. Its rerun `dl-runpath-2-startup2` (same head) ran all 6993 tests of the first pass green; the native
  pass's one failure was `niji-boundary::s03_memory_symbols kernel_pages_map_aligned_past_a_gap_without_an_aligned_range`,
  the known next-wide defect HEAP-GAP-TEST-KEKE fixes. Both superseded by `dl-runpath-3-startup` on this head.
- Ruling #7280 done (5a71e6d0b): docs/differences-from-glibc.md row in "Deliberate user-facing incompatibilities"
  and records/compat/COMPAT-37.toml (reserved; status `registered`, decisions D-27.16, D-X.95). No code change.
- Harness check for the ruling's "use LD_LIBRARY_PATH or the path, not an rpath": every `-Wl,-rpath` in the tests,
  conformance helpers, validation and tools names the test's own scratch directory, `$ORIGIN` or a fixed corpus path;
  none names the products, which tests pass by path (`libc_s`, `ld_s`) or LD_LIBRARY_PATH. Nothing to change.
- Plants: handoff/DL-RUNPATH-2-plants.txt rerun on this head (`dl-runpath-3-plants`): 13 plants, 12 CAUGHT,
  1 EQUIVALENT (`no-start-publish`, reason in the file), no WARN lines. No new code, so no new plants.
- Gate `dl-runpath-3-gate` (direct tier, not FULL): 9 suites, 654 passed, 0 failed, 0 SKIPPED lines, peak 3.1G.
- Start-up `dl-runpath-3-startup` (`cargo xtask test`): first pass 6993/6993; native pass 4854 passed, 1 failed:
  the same `kernel_pages_map_aligned_past_a_gap_without_an_aligned_range` (HEAP-GAP-TEST-KEKE), nothing else. Its
  `&&` chain therefore stopped before the full-profile check, which ran on its own as `dl-runpath-3-hottext`.
- Jobs and results: handoff/reports/DL-RUNPATH-3.json and the hand-back.

## Run state (DL-RUNPATH-4, branch `task/dl-runpath-4`, Skylake-SP host)

- Why it was red: CI could not merge next (conflict in dlopen.rs with DL-PROCESS-OBJECTS-2's growing loader
  tables), not a failure of this work. Step 0 (clippy + static on the inherited head 0ac30daa3,
  `dl-runpath-4-step0`): green.
- origin/next 97bf3542f merged (ebd22a1af). Conflict resolution: next's table imports and its `Spill` `retired`,
  with START_ORIGIN / START_RUN_PATH and the loader's `origin_tried` / `origin_len` / `origin` fields after it.
  One more site no longer compiled (`dl-runpath-4-check1`, E0608): `given_name` indexed `l.objs` directly; it now
  writes through `Recs::set` (9d3b0f530; both copies). Everything else of the chain already used the arena accessors.
- Plants: handoff/DL-RUNPATH-4-plants.txt (DL-RUNPATH-2's 13, `no-given-name`'s literal following the new line).
- Results on 9d3b0f530: `dl-runpath-4-check2` (clippy -D warnings, static, so, runpath_search 5/5,
  dlopen_process_objects 3/3 incl. the 300-library corpus), `dl-runpath-4-gate` (direct tier: 10 suites, 665 passed,
  0 failed, peak 2.3G), `dl-runpath-4-plants` (12 CAUGHT, 1 EQUIVALENT, 0 WARN), `dl-runpath-4-y10` (#10/#18:
  20 PASS / 0 FAIL), `dl-runpath-4-hottext` (full profile so + hot_text_products 33/33).
- `dl-runpath-4-startup` (`cargo xtask test`): first pass 7019 of 7020 passed; the one is
  `niji::dns_products resolv_equals_the_reference`, TIMEOUT at nextest's 1800 s, so the native pass did not run.
  Cause (outside this story): after a scenario's timeout the oracle's forked children (run under `sudo -n unshare`
  by `resolv_run.sh`) are left orphaned and keep the test's stdout pipe open, so `Command::output()` never returns
  and the test's own 900 s guard cannot end it. The stuck ones in this run were the host-linked oracle's (which never
  load libniji's ld.so); the role still holds such orphans from runs 4 days ago (s3, libniji-linked) and the s4 role
  from 11 days ago. They are root-owned: `kill <pid>` as claude gives EPERM (reported as refused, not retried).
  DL-RUNPATH-2's rc-124 start-up job on this host stopped at the same test. Recommend: a harness story (kill the
  scenario's process group inside the namespace, or read with a deadline), and a root clean-up of the orphans.

## Next step

Land. Then the owner's set-user-ID probe above, and the `$PLATFORM` probe on a v3 Intel host and an AMD host.
