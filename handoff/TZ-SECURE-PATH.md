# TZ-SECURE-PATH: the secure-mode TZ rule applied

**State:** branch `task/tz-secure-path` (from `origin/next` fa79409c7); code commit 3cc69ca4b, then this note and
the report. Role mei:~/w/s4 (Skylake-SP host).

## What was done

* `crates/core/src/time.rs`: `load(sys, tz, tzdir, default, secure)`; `load_in(..., trusted: Option<&[u8]>)`
  is the seam (secure = `Some(ZONEINFO)`). In secure mode `TZDIR` is ignored; a value with a `..` component
  anywhere, or an absolute path not strictly inside the trusted directory (compared component by component after
  empty and `.` components are dropped), reads no file and goes to the rule fallback, as a value naming no zone
  file does today (invalid rule: UTC named by the leading valid name, empty if none). Outside secure mode nothing
  changed. The unset-`TZ` default file (`/etc/localtime`) is read in both modes.
* `crates/legacy/src/time_compat.rs`: `Env` gains `secure: &dyn Fn() -> bool`, asked only in `add` (a new load,
  under the lock): the lock-free path of an implicit `tzset` is unchanged. `crates/boundary/src/zone.rs` passes
  `process::secure`.
* Tests: `crates/core/tests/time_load.rs` `s23_6_c_2_secure_names_files_only_inside_the_zone_directory`
  (host-independent: a scratch directory as the trusted one; inside loads, outside / sibling-prefix / `..` refused,
  the same values load in plain mode, `TZDIR` ignored, `load(.., secure)` wiring). `conformance/c/s23_tz_secure.c`
  (`RUN-SECURE`, end to end through the boundary; needs `/usr/share/zoneinfo/Asia/Tokyo`, skipped with the reason
  otherwise).
* Spec text: the threat-model `TZ` row made precise (a valid rule before `..` stays a rule; "inside" after
  normalisation); `spec/behaviour/dynlink-secure.md` gains the new probe cases.

## Probe (reference = host glibc 2.39, set-group-ID copy; libniji = the conformance run)

| TZ | reference secure | libniji secure | plain (both) |
|---|---|---|---|
| `Asia/Tokyo`, `:Asia/Tokyo`, `JST-9`, `/usr/share/zoneinfo/Asia/Tokyo`, `:/usr/share/...` | +9 JST | +9 JST | +9 JST |
| `/usr/share/zoneinfo//Asia/./Tokyo` | +9 JST | +9 JST | +9 JST |
| copy of Tokyo in `/tmp/...`, `:` form, `../../../..` + it | 0 `''` | 0 `''` | +9 JST |
| `Asia/../Asia/Tokyo` | 0 `Asia` | 0 `Asia` | +9 JST |
| `/usr/share/zoneinfo/../zoneinfo/Asia/Tokyo` | 0 `''` | 0 `''` | +9 JST |
| `JST-9/../x` | +9 JST | +9 JST | +9 JST |

The fallback matches the reference: no differences row, no compat option. Probe script: run from the story's
scratch (`probe.sh`: `cc` build of the conformance source, `chgrp` to a supplementary group, `chmod 2755`).

## Decisions in force

The threat model's `TZ` row (owner approval #2661, DL-0d section A). No new D-row: every detail chosen matches the
reference's probed behaviour.

## Open / not to redo

* A kept zone is keyed by `TZ` and `TZDIR`, not by the secure flag. `AT_SECURE` never changes in a started process;
  only the hosted fallback (no readable auxv: real vs effective IDs) can change, and only in a process that already
  had the privilege. Not addressed; recorded here.
* Plants: `handoff/TZ-SECURE-PATH-plants.txt` (6; the two wiring plants are caught by the conformance program).

## Next step

None for this story once its gate and plants are green; the CI train runs the full gate.
