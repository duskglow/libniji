# HEADERS-O2-INLINES handoff (run 1, leg 3: complete)

**State.** Branch `task/headers-o2-inlines` from origin/next e0110e274; role keke:~/w/n3. Head ed3bae0f4 (code 3c3c57eda;
D-X.158 is the record records/decisions/D-X/D-X.158.toml); the report json update is on top. precheck green on ed3bae0f4. Decisions in force: **D-X.158** (header-inline design, errno veto, P3 C++
include dirs); differences row "atof, atoi, atol, atoll" (no compat option, reason in the row).

**Commits.** c11e52e34 inline bodies (`__NIJI_INLINES` in features.h.in; stdio/stdlib/wchar bodies; string/strings C++
overloads); 4c0ceca40 callsite_probe C++ include dirs; 197bff597 attr_probe `e` test + errno veto, diagattrs.tsv,
regenerated libc-include; 5ca8a1e70 + 5bd40e576 + 3c3c57eda tests `xtask/src/headers/slices/o2inlines.rs` (3 tests)
and `validation/headers_o2inlines.plants`; 1479dff59 D-X.158 and the differences row.

**Results (Zen 1 host; reference Ubuntu 24.04 glibc 2.39).**
* Per-name probe table (black box, objects only): reference inlines when `__OPTIMIZE__ && !__OPTIMIZE_SIZE__ &&
  !__NO_INLINE__`; getchar -> getc(stdin), putchar -> putc(stdout); getc/putc_unlocked family -> `__uflow` /
  `__overflow`; feof/ferror_unlocked -> nothing; getline -> `__getdelim` (GNU only); atoi/atol -> strtol or
  `__isoc23_strtol` (C2X/GNU); atof -> strtod; atoll -> strtoll; mbrlen -> mbrtowc (state given) / `__mbrlen`;
  bsearch -> nothing; string.h/strings.h search names: C++ const/non-const overloads. libniji now matches each.
* P3 on the 26 names: 55 rows -> **0**. o2probe (9 levels x 14 macro sets, C/C++): equal (except libstdc++'s own
  `-ffreestanding` wrapper rows on the reference side: probe artifact). nxprobe noexcept 64/64 equal.
* errnoprobe: callers that fold an errno test with libniji's headers and not the reference's: 269 -> **0**.
* attr table and headers byte-stable over two regenerations; attr_probe check 0 differ.
* Tests: `cargo test -p xtask o2inlines` 3/3 (jobs headers-o2-inlines-t2, and the suite below).
* Plants (headers-o2-inlines-plants): **4/4 CAUGHT**, no WARN: getchar body dropped, inline at -Os, `pure` back on
  atof, strchrnul's const overload dropped.
* P6 (headers-o2-inlines-p67): conformance `s08_strto_fenv_flags` with `--niji-headers` 1 passed (and default mode
  1 passed). P7 (same job; Devuan CPPFLAGS/CFLAGS both sides, `--niji-headers`): zlib 1/1 and xz-utils 19/19 on
  both sides; undef-compare **168 objects, 0 differ** (xz's `putchar` row gone; was 1 differ).
* `cargo xtask affected` escalates to FULL (header templates are generator inputs): FULL is the train's; gated
  here on `cargo test -p xtask` + precheck. Job headers-o2-inlines-xtask (before `so`) failed only
  `xtask/tests/transcripts.rs` (needs `target/niji-test/iconv`, built by `cargo xtask so`); after `so` the same file
  passed 7/7 (p67 job); headers-o2-inlines-xtask2 reruns the whole suite on the final head.

**INFORMATIONAL.** (1) attr_probe's `ct` test condition never excludes void functions (`ret` still ends with the
name), so void functions get a `ct` line that fails to compile and is dropped ("53 functions have a test that did
not compile"). Harmless; the `e` test uses a correct `has_result`. Not changed. (2) `cargo test -p xtask` needs a
prior `cargo xtask so` for transcripts.rs (it does not skip when the iconv product is absent).

**RULING NEEDED.** No compatibility option for the atof family's missing `pure` (recommend none: restoring it
restores a wrong errno reading; reason recorded in the differences row).

**Not to redo.** Probing, templates, callsite_probe, attr_probe veto, table, regeneration, tests, plants, P6, P7.

**Next step.** None for this story: the train's FULL gate. Probe scripts (not in the tree): local scratch and keke
`~/work/headers-o2-inlines/` (p3.sh, o2probe.py, cxxprobe.sh, nxprobe.sh, errnoprobe.py, attrjob.sh, p67.sh).
