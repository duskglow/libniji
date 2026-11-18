# SL-HEADERS-S10: slice 10, locale, text, matching (handoff after run 3)

**State.** Branch `task/sl-headers-s10-3` (continues `task/sl-headers-s10-2`); head in `handoff/reports/SL-HEADERS-S10-3.json`. Say
"slice 10" in commits. 20 headers generated and committed; `header_check.py` 0 failed of 6048; clippy and static clean.

**Done in run 3.**
* Moved the branch onto the header registry (`handoff/SL-HEADERS-REGISTRY.md` steps 1-4): took next's `header_check.py` and `headers.rs`,
  re-expressed the slice as `headers/slices/s10.tsv`, `spec/tools/header_checks/s10.py` (`must_fail = [regexp.h]`, `p1_skip` for
  `ftw.h` `statx`/`statx_timestamp`). Two shared edits, each its own commit: `headers.rs` (bit-field members of `layouts.tsv`, and the
  `FILE` tag-only rule) and `header_check.py` (the optional module names `must_fail` and `p1_skip`, the "must fail" runner rule, and
  P1 leaving out `FILE` where the header shows no `struct _IO_FILE`). Same count rule as the registry: M 6048 after, nothing lost.
* The `FILE` rule is data driven, so `pwd.h`, `grp.h`, `shadow.h`, `gshadow.h` (slice 14) were regenerated: they declare the tag only,
  as `visibility.tsv` has them.
* P2 (10 rows) and P3 (60 rows) classified in `records/pending/SL-HEADERS-S10-3-1.toml` (D-NEW, series D-10): every row waits for
  `<sys/stat.h>` (slice 7) or the fortify layer (slice 18); the endian/`sys/types.h` rows went away with the merge (SL-HEADERS-VIS-FIX
  owns that family, ruling #6462). P7 cannot run until slices 7 and 8 land (no `fcntl.h`, `sys/stat.h`, `dirent.h`, `signal.h`).
* P6 (`conformance/run.sh --niji-headers`) and plants: see the report json for the labels and results.

**Decisions in force.** #6415 (regexp.h fails like the reference, probe table in `SL-HEADERS-S10-2-1.toml`), #6416 (`fnmatch.h` under
`_TIME_BITS=64` alone via `__NIJI_TIME_BITS_UNCHECKED`), #6462 (VIS-FIX owns the endian family).

**Not to redo.** regexp.h probes; the P2/P3 classification above; the registry move.

**Next step.** After slice 7 lands: add the `sys/stat.h` include to `ftw.h`/`fts.h` templates, empty `p1_skip` in `s10.py`, rerun P2
(the 5 `ftw.h` rows should go) and run P7 (grep, sed, diffutils, findutils, gawk). After slice 18: P2/P3 fortify rows.

**Tools.** `~/work/sl-headers-s10-3/probes.sh <n>` on the build host (P2, P3 only; scratch copy in the run's scratch directory).
