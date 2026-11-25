# HEADERS-NONNULL-MATCH handoff (end of leg 3; PENDING JOBS)

**State:** branch task/headers-nonnull-match, head: `git log -1` on the branch. Role keke:~/w/n2. keke scratch
`~/work/headers-nonnull-match/` (job scripts job6..job10.sh, zstd.sh; logs out7..out9). Decision D-X.174 (records/
decisions/D-X/D-X.174.toml); provenance fragment spec/abi/provenance.d/HEADERS-NONNULL-MATCH.txt.

## Result
* **Cause** (spec/tools/attr_probe.py, the `n.k` test): a literal 0 for function-pointer arguments on every line
  (pthread_create 2,4; on_exit 2); a size of 1 that woke `access` attributes (write 2, fwrite 1, strxfrm 1, regexec 4,
  regerror 3, the `*_r` buffers); the compiler's builtin knowledge read as the header's (printf/scanf formats, fputs,
  nan*, dcgettext 2; the nonnull unit now builds with -fno-builtin); no recursive header walk or pointer typedefs /
  `locale_t` (qsort 4, bsearch 5, statfs, rlimit, statx, strto*_l); calls through a function-like macro (fread_unlocked,
  fwrite_unlocked 4; the call is now `(F)(..)`).
* **Counts vs origin/next:** 56 functions lose marks (62 params), 48 gain them (53 params; 25 are functions new to
  the table).
* **Proof (chk7/chk8/chk9, keke):** `check --kinds n,rn` 0 differ in base, gnu, default, fortify2, c++, c++-fortify3;
  zstd 1.5.7 41 files with -Werror=nonnull: 0 failed (libniji and reference, fortify off and =2).
* **Fortify:** the reference's nonnull sets do not depend on `_FORTIFY_SOURCE` (chk7 split columns empty). The
  nonnull_fortify / nonnull_nofortify columns and the `__NIJI_NONNULL_F/NF` macros are removed; `table` now runs the
  nonnull tests with fortify on and off and stops, naming the functions, if they ever differ.
* **Full check (diagnostic_attributes_match_the_reference):** green on origin/next (2567 functions); the recursive
  walk probes 2826, and the table lacked the new rows (191 differences). Fixed: `check` compares an n/rn line only on
  its own signal (other attributes are judged on their own lines); rows spliced in from a full `table` run for
  deprecated (17), wur (11) and pure (4). chk9 with a full table regen left 10 differences (fmaximum* ct), caused by
  the full run dropping const on 30 committed double-function rows; the splice keeps the committed pure column
  instead. Verification of the spliced table: job chk11 (full base check) and the gate.

## Jobs on the final head (keke, started 17:20Z, queued for slots)
headers-nonnull-match-lint3 (clippy + static), -gate3 (affected --run), -plants3 (handoff/HEADERS-NONNULL-MATCH-plants.txt,
6 plants), -chk11 (attr_probe check, base, all kinds: expect 0 differ). Superseded: chk6..chk9 (older heads), chk10
(killed: origin/next table reproducibility probe, not needed).

## Exact next step
`~/bin/jobwait headers-nonnull-match-lint3 headers-nonnull-match-gate3 headers-nonnull-match-plants3 headers-nonnull-match-chk11`;
if all green: `cargo xtask precheck --jobs <those> --head <sha>` on the role, handback-check. A red chk11 / s19 test:
read `~/logs/headers-nonnull-match-chk11.log` (`^line`) and splice the missing column for the named rows
(script: the leg-3 scratch splice.py logic, documented in the provenance fragment).

## Open / for the report
* RULING NEEDED (follow-up story): the full `attr_probe table` no longer reproduces the committed pure column on 30
  double functions (asinh, cos, floor, fmaximum ...) and would add const to 152 complex.h functions; not taken here.
  Recommend a story to find why (the e/ct/bp tests with the three-unit split) before any full table regen.
* INFORMATIONAL: the reference marks warn_unused_result only under _FORTIFY_SOURCE for many functions (atoi, a64l,
  bsearch, chdir ...); libniji always: -Werror=unused-result without fortify fails on libniji only. Follow-up: wur
  per configuration.
* INFORMATIONAL: with -fno-builtin, libniji's explicit sentinel (execl), format (vprintf, strftime) and const (nan*)
  marks warn where the reference relies on the compiler's builtins.
* INFORMATIONAL: libniji has no fread_unlocked/fwrite_unlocked macro: a literal NULL stream at -O2 warns on libniji only.
* INFORMATIONAL: c++: 55 functions build their test lines on the reference only (chmod, chown, frexp ...).
* zstd 1.5.7 (`zstd-dl`) was fetched in leg 1, before the coordinator's local-mirror note (sha256 eb33e51f...).
