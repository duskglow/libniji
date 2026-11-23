# HEADERS-O2-INLINES handoff (end of leg 2; WIP, code done, proofs pending)

**State.** Branch `task/headers-o2-inlines` from origin/next e0110e274; role keke:~/w/n3. HEAD = the commit that adds
this note (on top of 197bff597). Commits: c11e52e34 (WIP subject: templates), 4c0ceca40 (callsite_probe C++ include
dirs), 197bff597 (attr_probe errno veto + diagattrs.tsv + regenerated libc-include). All headers are regenerated and
committed. **Sync caveat:** sync_role puts the worktree's libc-include/ and spec/abi/ back over the role: after a
regeneration on keke, copy the changed files back (tar over ssh) before the next sync.

Probe scripts (not in the tree): local scratch `~/w/.scratch/HEADERS-O2-INLINES/`, keke `~/work/headers-o2-inlines/`:
`p3.sh <abs out dir>` (prints the compare rows for the 26 names only), `o2probe.py ref|niji ~/w/n3`, `cxxprobe.sh`,
`nxprobe.sh ref|niji` (C++ noexcept of 32 call forms), `errnoprobe.py ~/w/n3 OUT "-O2;-Os;-O0"`, `attrjob.sh` (table x2,
headers-gen, byte-stability, errnoprobe, attr check, p3). Jobrun outputs under ~/work are not visible (private mount?):
write to /tmp/hoi-* or print to the log.

**Results on the final code (Zen 1 host, reference Ubuntu 24.04 glibc 2.39), all leg 2:**
* o2probe niji vs ref: equal over -O0/-O1/-O2/-O3/-Os/-Og/-fno-inline/-ffreestanding/-fno-builtin x 14 macro sets,
  except the reference's C++ `-ffreestanding` `!error` rows for stdlib.h names (libstdc++ wrapper; the niji side of
  o2probe uses -nostdinc, no libstdc++): probe artifact, not a header shape.
* cxxprobe: equal except `&abs`/`&div` (libstdc++ wrapper, same artifact). nxprobe (noexcept, -O0/-O2): 64/64 equal.
* P3 (26 names, 5 headers, new callsite_probe): **0 rows** (55 before; 52 call/addr rows closed by templates, abs/div
  addr by the libstdc++ include fix).
* errnoprobe: libniji folds where the reference keeps: **0** (269 before). 359 fold on both sides (fabs, strlen...).
* attr_probe check: 0 differ; explained (printed, not counted): atof atoi atol atoll -Wunused-value (ERRNO_VETO:
  libniji's declaration is no longer pure; the reference's is, its -Os objects fold errno after atof), and
  argz_next, _option_is_end, is*_l (ERRNO_INLINE: the reference's own inline expansion folds; libniji's call keeps).
* attr table and headers: unchanged across two regenerations (BYTE-STABLE).
* Per-name probe table (before, from leg 1): inline when __OPTIMIZE__ && !__OPTIMIZE_SIZE__ && !__NO_INLINE__;
  getline -> __getdelim (GNU only); atoi/atol -> strtol or __isoc23_strtol (GNU/C2X); atof -> strtod; atoll -> strtoll;
  mbrlen -> mbrtowc / __mbrlen; getc_unlocked family -> __uflow / __overflow or nothing; bsearch -> nothing.

**Not to redo.** Probing, templates, callsite_probe, attr_probe veto, table, regeneration.

**INFORMATIONAL for the report.** attr_probe's `ct` test condition `ret.replace("extern", "").strip() != "void"`
never excludes void functions (`ret` still ends with the name), so void functions get a `ct` line that fails to
compile and is dropped (the "53 functions have a test that did not compile" line). Harmless; the `e` test uses a
correct `has_result`. Not changed (no table effect).

**Next steps (exact).**
1. Differences row (docs/differences-from-glibc.md, neutral): atof/atoi/atol/atoll are not declared pure, so a caller's
   errno check after the call stays at every -O level and `atof(s);` gives no -Wunused-value; no compat option
   (reason: the attribute lets the compiler drop an errno the call sets). Decide: also a COMPAT? (recommend none).
   Reserve a D-X id (`ops reserve --story HEADERS-O2-INLINES D-X`) for: header-inline design (__NIJI_INLINES condition,
   gnu_inline bodies from the spec), errno veto in attr_probe, C++ include dirs in P3.
2. Tests: new `xtask/src/headers/slices/o2inlines.rs` (see s18.rs `an_open_without_its_mode_calls_the_checked_entry`
   for compiling against libc-include and reading nm): per-name call shape at -O2 / -O0 / -Os (getchar -> no getchar
   ref at -O2, getchar at -O0/-Os; atoi -> strtol; getline -> __getdelim with _GNU_SOURCE; mbrlen -> mbrtowc), C++
   overload shape (const in -> const out compiles, const in -> char* fails, `&strchrnul` ambiguous), errno kept after
   atof and acos at -O2 and -Os (`errno = 0; r = atof(s); if (errno) mk();` -> mk referenced). Clause names in comments.
3. Plants file `validation/headers_o2inlines.plants` (docs/plants.md form): getchar's inline body reverted in
   headers/stdio.h.in (cmd must run headers-gen first, then the o2inlines tests); `pure` back on atof (diagattrs.tsv
   row `atof\t1\t1\t\t\t\t\t\t\t1`). Run `cargo xtask plants` under jobrun (RAM dir /dev/shm/claude/headers-o2-inlines/plants-<n>).
4. Conformance s08_strto_fenv_flags with libniji's headers (61 atof rows; see how SL-HEADERS-S07-TAIL ran it,
   `--niji-headers`), P7 zlib + xz (Devuan flags, handoff/SL-HEADERS-S18.md "Next step"): xz's putchar gone.
5. clippy, static, gate `cargo xtask affected --run` (jobrun gate), plants, precheck; report json; handback-check.
