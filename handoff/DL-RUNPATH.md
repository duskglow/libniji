# DL-RUNPATH: dlopen searches the program's and the caller's DT_RUNPATH/DT_RPATH `$ORIGIN`; RTLD_NOLOAD by loaded names

State: branch `task/dl-runpath-2` (DL-RUNPATH continued, origin/next e4d7011e6 merged). Head and job results:
handoff/reports/DL-RUNPATH-2.json and the hand-back.

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

- RULING NEEDED (coordinator note, DL-PROCESS-OBJECTS-2 #7265; not the `$ORIGIN` defect, no test added). A program
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

## Next step

Land. Then the owner's set-user-ID probe above, and the `$PLATFORM` probe on a v3 Intel host and an AMD host.
