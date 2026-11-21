# SL-HEADERS-S10: slice 10, locale, text, matching (handoff after run 5)

**State.** Branch `task/sl-headers-s10-5` (continues `task/sl-headers-s10-4`); head in `handoff/reports/SL-HEADERS-S10-5.json`.
Say "slice 10" in commits. The slice is data-only (ruling #6748): no diff in `spec/tools/header_check.py` or `xtask/`.

**Run 5.** Merged origin/next (HEADERS-GEN-S10-HOOKS landed the p1_skip/must_fail hooks and the FILE tag-only rule). Dropped this
branch's versions of both edits (xtask/src/headers/typedefs.rs and header_check.py are next's). The generator now refuses a literal
define of a macro macros.tsv owns: `FNM_NOSYS` (fnmatch.h.in) and `LC_GLOBAL_LOCALE` (locale.h.in) use `@@value(...)`. Regenerated
libc-include/ (fnmatch, fts, ftw, langinfo, locale, re_comp, regex, search, wordexp). Plants: the four that planted the dropped
generator edits are removed (the hooks story owns them); the two literal plants follow the new template text. `headers-gen --check`
ok (165 headers).

**Decisions in force.** #6415 (regexp.h fails like the reference), #6416 (`fnmatch.h` under `_TIME_BITS=64` alone via
`__NIJI_TIME_BITS_UNCHECKED`), #6462 (VIS-FIX owns the endian family), #6748 (data-only slices).

**Open.** `header_check.py` reports 8 P8 failures, all `sys/mount.h` with `linux/mount.h` (`struct winsize` redefined). This branch has no
diff in `sys/mount.h` or its template against origin/next, so it is next's (slice 15 area), not this slice's.

**Not to redo.** regexp.h probes; the P2/P3 classification (`records/pending/SL-HEADERS-S10-3-1.toml`); the registry move.

**Next step.** After slice 7 lands: add the `sys/stat.h` include to `ftw.h`/`fts.h` templates, empty `p1_skip` in `s10.py`, rerun P2
(the 5 `ftw.h` rows should go) and run P7. After slice 18: P2/P3 fortify rows.

**Tools.** `~/work/sl-headers-s10-3/probes.sh <n>` on the build host (P2, P3 only).
