# DL-RUNPATH: dlopen searches the program's and the caller's DT_RUNPATH/DT_RPATH `$ORIGIN`; RTLD_NOLOAD by loaded names

State: branch `task/dl-runpath`, cut from origin/next df83365cd (DL-NAME-PATH-2 already landed in it). Head and job
results: handoff/reports/DL-RUNPATH.json and the hand-back.

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

## Open (route)

- RULING NEEDED, secure mode: no AT_SECURE probe was possible without root (a set-uid program needs another owner).
  dlopen now applies to the program's `$ORIGIN` the same rule the start already does (threat-model rule 2: only
  a trusted directory; the `/proc/self/exe` source is DL-NAME-PATH-2's pending ruling). Recommend accept.
- INFORMATIONAL: `$PLATFORM` is `haswell` on an x86-64-v3 CPU (D-27.16, probed on Haswell); on this Zen 1 host the
  reference expands it to `x86_64`. A loader story should probe both CPU kinds.
- INFORMATIONAL: start-time objects without DT_SONAME are still left out of a handle's local scope
  (`adopt_deps` matches by soname or path only). `given_name` now has the name, so a follow-up can match it.
- Not probed on the reference: dlopen through an LD_LIBRARY_PATH `$ORIGIN` list (the start's case was probed by
  DL-NAME-PATH-2). The test expects the start's rule.

## Next step

Land. Then the secure-mode ruling above; the `$PLATFORM` probe on a v3 Intel and an AMD host.
