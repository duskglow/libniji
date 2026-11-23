# DOCS-CATCHUP-1121: release notes and differences rows for the 11-20/11-21 snapshots

State: branch `task/docs-catchup-1121` from origin/next 0495c4684; HEAD in the hand-back. Role keke `~/w/r3`.

## Done
- docs/RELEASE-NOTES.md "Unreleased": 12 entries from the RELEASE-SNAPSHOT-1120C..1121B handoffs: #28, #6, #25
  (with THREAD-EXIT-NO-UNWINDER's loud failure), #23 kind 1, #24, #21, the loader's growing tables (+ dlerror record
  unmapped at thread exit), libresolv, start-up (Skylake-SP empty main 788,679 / 56 faults / 45 syscalls on 11-20 ->
  741,148 / 52 / 37 at 21.3), #20 (FCLOSE-COOKIE-SEEK), packages (libc6-dbg 19 of 19, coexist serial).
- docs/differences-from-glibc.md: "Character sets" intro at snapshot-2026-11-21.3 (294 names listed vs 1180; 79 of
  81 spelling checks; what does not open). Three rows for rulings #7286-#7288 with D-12.34 and registered
  COMPAT-39/40/41 (reserved ids; records/ files, views left to the train).
- Premise moved: the `-static-libgcc` thread-exit row already landed with THREAD-EXIT-NO-UNWINDER (D-22.23, COMPAT-38);
  not re-added. #23's kind-2 row ("Byte operations on a wide-oriented stream": ISO C) and kind-3 row ("UTF-8 above
  U+10FFFF": RFC 3629) state libniji's behaviour and reason: unchanged.
- Brief wording corrected: a cookie stream without a seek hook gives `fflush` 0 (not ESPIPE); ftell/fseek -1 ESPIPE.

## Probe (keke, job docs-catchup-1121-probe-1; glibc 2.39 dynamic vs libniji static, cookie over 100 bytes, read 10)
| case | glibc | libniji |
|---|---|---|
| no seek hook: fflush / ftell / fseek | -1 errno 0 / -1 EIO / -1 errno 0 | 0 / -1 ESPIPE / -1 ESPIPE |
| seek hook EINVAL: fflush | -1 EINVAL, ferror 0 | -1 EINVAL, ferror 1 |
| seek hook ESPIPE: fflush | 0, ferror 0 | 0, ferror 0 |
| memfd `r`, read 10 of 100, fclose: dup'ed fd offset | 100 | 10 |

## doccheck (RELEASE-SNAPSHOT-1120C's doccheck.py)
Before 20 problems, after 20 (the brief's 23 had already dropped to 20 on next); none in RELEASE-NOTES.md or
differences-from-glibc.md. The rest, one line each: arch-syscalls.md:47 csys/include/x86_64/; benchmarks.md:154
docs/saturation/05- (prefix); benchmarks.md:414 bench/stress/; install-packages.md:22 include/liella/ (old name);
perf-ledger.md:9 tests/; saturation/15-...md:3 bench/c/--only.c (a flag read as a path); compat-flags.md:33 and 13
lines in saturation/01..05, 07 (generated views: missing handoff/*.md and crates/libc-so/hot-text.list, fix in records).

## Checks
gate-1 (first head d3a76af93): records check OK; static red on trace: two new rows named no clause ID; fixed
(cookie row: s12_3_c_3, s12_8_f_0; fclose row: `no clause ID`, none exists). gate-2 on the final head: see hand-back.

## Next step
If gate-2 is green, land. Later snapshots (1121C..1122D: LOCALEDEF-*, LG6-CTYPE-OBJECTS, HEADERS-O2-INLINES, ...) are
not in these notes: a follow-up docs story.
