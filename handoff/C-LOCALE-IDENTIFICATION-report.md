# C-LOCALE-IDENTIFICATION report

Commits (all on task/c-locale-identification, base origin/next 0495c4684):
- dce808f7f Report the reference's C-locale LC_IDENTIFICATION, less contact fields (TSV + provenance tool + probe
  source, regenerated builtin_items.rs, oracle, s7_4_c_2, D-7.22, differences row, plants file)
- d1ca49dbd Check every C-locale identification value in s7_4_c_2 (plants catchable within the 600 s limit)
- the report commit on top.

## Results (keke:~/w/n4, Zen 1 host)

- Lint (c-locale-identification-lint, on dce808f7f): locale-gen --check, records check, static, clippy -D warnings: ok.
- Probe, issue reproducer: libniji (static products of the final head) prints `global-C territory [ISO]`. Every
  `_NL_IDENTIFICATION_*` index 0..16 of C, POSIX and C.UTF-8 (CATEGORY: all 12 strings) equals the reference
  except the five contact fields of C and POSIX (empty by design, D-7.10). C.UTF-8 equal on all 17.
- Gate (c-locale-identification-gate2, final head): rc 0; 15 suites ok, 6527 passed, 0 failed, 76 ignored; conformance 2/2; static ok; `gate: peak memory 2.2G (page cache included; reclaimed under the limit) oom_kill=0 rc=0`. affected said `FULL: the train's` in its bounded-set form (generated table and its input), not escalated.
- Plants (c-locale-identification-plants2, final head): 4 plants: 4 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN-RC-ONLY, 0 WARN-NO-MSG, 0 BUILD-TIMEOUT.
- Start-up checks (§9): c-locale-identification-startup (`cargo xtask test`, then NIJI_FULL_PROFILE so + hot_text_products): rc 0 (layout check STARTUP-DATA-WINDOWS passed, every test green, hot_text_products green). The change is data only (the built-in string pool), no code on a start-up path, so no layout-bins diff was taken.

## Superseded

- c-locale-identification-lint: ran on dce808f7f; the final gate (gate2) ran clippy and static on the final head.

- c-locale-identification-gate: older head dce808f7f; stopped by me when the test changed (its products and xtask
  steps had passed).
- c-locale-identification-plants (plants-1): BASELINE FAILED: the two host-diff plants' command (`locale-gen && xtask
  so && cargo test ... locale_products`) ran past the 600 s per-plant limit while the gate built beside it. Plants
  now use the in-process s7_4_c_2, which checks every value.
- c-locale-identification-quick: ran the old test version (a sync was BUSY); quick2 is the valid one.

## Rulings and notes

- No RULING NEEDED: coordinator #7965 confirmed the five contact fields stay empty (D-7.10, owner 2026-10-01).
- INFORMATIONAL: the PROBE program `spec/tools/langinfo_probe.py` needed was not in the tree; it is now
  `spec/tools/probe_langinfo.c`, and a full re-record with it reproduced every other row byte for byte.
- INFORMATIONAL: `kill-label <gate label> --kill` ended the jobrun wrapper but left the gate's `xtask affected --run`
  children running (sync then BUSY); `kill-mine <role> --kill` ended them.
