# SL-HEADERS-S17: slice 17, GNU and BSD extension APIs (handoff after run 3, story SL-HEADERS-S17-3)

**State.** Branch `task/sl-headers-s17-3` (continues `task/sl-headers-s17-2`), merged with origin/next (merge commit 9135be81e). The 16 slice 17 headers
are generated and compile alone: `header_check.py` 0 failed of 5616 units (all slices on the merged tree), `headers-gen --check`, clippy, `cargo xtask static`,
`stubs` and `records check` clean (job `sl-headers-s17-3-lint2`). P10 identical (earlier run, templates unchanged since). Say "slice 17".

**Run 3 (done).**
* Moved onto the registry (handoff/SL-HEADERS-REGISTRY.md steps 1-4): the 16 headers are rows of `headers/slices/s17.tsv` (p1 1, uapi -); `header_check.py` and
  `xtask/src/headers.rs` are next's files plus the shared edits below. No slice 17 module is needed (no `@@marker` of its own).
* Shared-code edits (named for the coordinator): `header_check.py` (`PTHREAD_STACK_MIN` not value-asserted; the most negative 64/32-bit value is written as `(-N+1) - 1`;
  `_X_H_` guards skipped like `_X_H`); `probe_compare.py` (same `_H_` guard rule); `headers.rs` `members_nested` (bit-field members `T name : W`, `fn(...) *` members as
  `void (*name)(void)`, pointer members inside an unnamed union), `type_words` (tag declared before a parameter list that names it; `FILE *` through `tag_of_typedef`; `const struct T *`;
  typedef-named members), object declarations of function-pointer type (`extern void (*name)(void)`). next already had the `covered`-by-configuration rule (S09), so mine was dropped.
* `struct _obstack_chunk`: added to `abi_extract.py`'s list, rows added to `spec/abi/layouts.tsv` (4 rows, run on the reference system; the same run also changed two `prototypes.tsv`
  lines, not kept), `abi-gen` regenerated `crates/abi` (layout.rs, appendix_c_layouts.rs), `stubs` clean. `layouts-aarch64.tsv` has no `_obstack_chunk` rows (no arm64 sysroot here).
  `obstack.h.in` no longer `@@owns` the tag, so the header defines it.
* Ruling #6480 applied: D-X.131 + two rows in `docs/differences-from-glibc.md` (argp helpers and inline `argp_usage`/`argz_next`; complete `struct _IO_FILE` in printf.h). The exported
  `argp_usage`, `argp_state_help`, `argp_parse`, `argz_next` are declared (argp.h, argz.h) and exist in `symbols.tsv`. `validation/headers_s17.accept` lists the accepted printed lines.
* P2/P3 after the work (16 headers, job `sl-headers-s17-3-p23b`): P3 6 differences, all accepted (0 unaccepted); P2 23 differences, 1 accepted (`_IO_FILE`), 22 not:
  21 are slice 6 rows seen through an include (identical rows print for ctype.h and stdio.h alone: `_tolower _toupper isascii toascii` macro configurations, the 12 `isX_l`
  macros, `tolower_l/toupper_l` and `fwrite_unlocked` in the fortify configurations) and 1 `_LIBC_LIMITS_H_` (fixed in `probe_compare.py` after that run, rerun not done).
  **These are slice 6's residue (SL-HEADERS-S06), not fixed here**: they need ctype.h/stdio.h template work or a differences row decided with slice 6.
* Plants: `validation/headers_s17.plants` (9; the covered-names plant was replaced by two plants on the member code). Job `sl-headers-s17-3-plants` on dd452938a: 9 plants, 3 CAUGHT (member bit-field width, union pointer member, int64-min literal), 0 SURVIVED, 6 WARN-NO-MSG (queue-tailq-remove, obstack-finish-align, object-fnptr-name, tag-forward-decl, mcheck-enum-value, printf-va-list): each mutant of a template or of the generator fails the staleness check of `cargo xtask so` (`stale under libc-include/`) before the plant command runs, so its msg regex (what that command prints) never appears; rc is 1 for the right cause. Fixing that needs the plants tool to regenerate libc-include in the copy before `so`.
* Gate `sl-headers-s17-3-gate` (`affected --run --base origin/next`) escalated to FULL (layouts.tsv, crates/abi and xtask files changed; the registry run took about 61 minutes): still running at hand-back; the coordinator collects.

**Decisions in force.** D-X.131 (this run); pending `records/pending/SL-HEADERS-S17-2-1.toml` (D-X, generator additions; its text now also stands for the member forms above).
**Not to redo.** Header-alone fixes; P10; P3; the registry move; `_obstack_chunk`.
**Open.** (1) slice 6 ctype/stdio P2 residue (above) in the slice 6 continuation; (2) P6 (`cargo xtask so`, conformance with `--niji-headers`) not run; (3) `layouts-aarch64.tsv`
needs `_obstack_chunk` rows from an arm64 sysroot; (4) I once ran `git init` in the mei role dir by mistake (an empty `.git`, removed at once; nothing else touched).
**Run it.** `python3 spec/tools/header_check.py --include libc-include`; `cargo xtask headers-gen --check`; `sh validation/headers_p10_s17.sh`; plants
`cargo xtask plants validation/headers_s17.plants --dir <new RAM dir>`; P2/P3: header_probe.py / callsite_probe.py `--include libc-include --headers <16 headers> --out D`,
then `probe_compare.py visibility|callsites --got D --headers ... --accept validation/headers_s17.accept`.
**Plan rows to read.** plan/headers.md sections 2.4, 3, 4, 4.1 (slice 17).
