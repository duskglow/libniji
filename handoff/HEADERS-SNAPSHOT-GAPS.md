# HEADERS-SNAPSHOT-GAPS: header gaps that broke tree, xxhash and static busybox (snapshot 24.2), stale "no headers" texts

**State (leg 1 ended at a refresh):** branch `task/headers-snapshot-gaps` (from origin/next 460d23b8f), role keke:~/w/n1.
Commits: 251c06db3 docs (done), eb3d41f4f probe candidates + visibility rows, 0ec35ae45 typedefs/macros rows,
73c68a7a9 S_IS* template + s07 unit. `libc-include/` is NOT regenerated yet (static's headers-gen --check is stale).
Decisions in force: none new yet (every change follows the reference's probed visibility/type/value; no differences row).

## Done
* **Cause of tree/busybox gaps:** the names were never probe candidates. `spec/tools/header_probe.py`: `package_names()`
  (build-log names + families from busybox's own source, utmp(5)/utmpx(5), installed docs) and the rule "each reserved
  `__X_t` candidate also probes `X_t`" (finds blksize_t, suseconds_t, fsblkcnt_t, ... which sys/types.h also lacked).
  `--names FILE` probes a subset (a name's rows depend on its own units only). Run on keke (reference: Ubuntu GLIBC
  2.39-0ubuntu8.9, GCC 13.3), job headers-snapshot-gaps-hp2: 94 names, 1968 rows spliced into visibility.tsv (committed
  rows unchanged, in place; the 216 `-` rows identical). type_probe.py --headers (the 90 headers showing them), job
  headers-snapshot-gaps-tp1: 29 typedefs.tsv + 21 macros.tsv rows spliced. Probe-settled: u_long etc. in sys/types.h in
  gnu17,gnu2x,default,bsd,svid,gnu,lfs64,fob64,time64,dynstack,reentrant,fortify1-3, C++; RE_TRANSLATE_TYPE in regex.h
  and re_comp.h under gnu + C++ only; _PATH_UTMP/_PATH_WTMP in paths.h `*`, utmp.h `!time64only`, utmpx.h gnu+C++;
  UTMP_FILE/WTMP_FILE "/var/run/utmp" "/var/log/wtmp" in utmp.h; UTMPX_FILE/WTMPX_FILE in utmpx.h gnu+C++ only.
  Scratch scripts (not in tree): ~/w/.scratch/HEADERS-SNAPSHOT-GAPS/{newnames.py,splice_vis.py,splice_tm.py}.
* **xxhash:** reference hides S_IFMT/S_IFDIR in posix200809 and below while S_ISDIR works; libniji's S_IS* expanded
  to S_IFMT. headers/sys/stat.h.in now uses `@@value(S_IFMT)`/`@@value(S_IF*)`; s07.py `units` compiles every S_IS*
  in every configuration. NOT YET CONFIRMED that xxhash's xsum_os_specific.c defines `_POSIX_C_SOURCE` (its compile
  line: default mode, `-O2 -D_FORTIFY_SOURCE=2`): fetch xxhash 0.8.3 orig from the mirror (as busybox was, into
  mei:/mnt/zram/claude/HEADERS-SNAPSHOT-GAPS/) and grep its feature macros.
* **Docs (step 5):** libc6-dev and niji6-dev descriptions, README.md:27, release-notes.d/SL-HEADERS-S20.md (niji-gcc,
  niji-g++ only), install-packages.md (no a.out.h). Done.

## Exact next steps
1. `RE_TRANSLATE_TYPE` is type-valued (no macros.tsv row): own it in headers/regex.h.in and headers/re_comp.h.in
   (`@@owns`, `@@if RE_TRANSLATE_TYPE`), type from a black-box probe on keke (`_Generic((RE_TRANSLATE_TYPE)0, ...)`,
   gnu config) and layouts.tsv's re_pattern_buffer `translate` member; busybox grep.c's later errors (lines 381,
   802-806) follow from the unknown type.
2. On keke role: `cargo xtask headers-gen`, fix what it rejects (new typedefs in ~70 headers, sighandler_t, sigval_t);
   copy back libc-include/, commit. Then `python3 spec/tools/header_check.py` (jobrun) and form.py rows untouched.
3. Plants in handoff/HEADERS-SNAPSHOT-GAPS-plants.txt via `cargo xtask plants` (stat.h.in back to S_IFMT -> s07 unit;
   drop the sys/types.h u_long visibility row -> P1; utmp WTMP_FILE row; regex RE_TRANSLATE_TYPE).
4. Gate: `cargo xtask affected` will say FULL (generated-table inputs) -> precheck + header_check (kind test).
5. Step 4 of the brief: debs from this head + `ypk.sh` on mei (~/work/RELEASE-SNAPSHOT-1124D/, debs.sh, then ypk.sh;
   tree, xxhash, busybox niji must PASS). hp.sh there re-probes the named identifiers.
6. clippy, static, precheck, report json, handback-check.
