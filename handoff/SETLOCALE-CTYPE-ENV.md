# SETLOCALE-CTYPE-ENV: handoff (after SETLOCALE-CTYPE-ENV-6)

State: branch `task/setlocale-ctype-env-6` = SETLOCALE-CTYPE-ENV-5's branch plus a merge of origin/next 7288d847a
(merge commit 010575ec4, no conflicts), plus this handoff and report. Build role mei:~/w/s2.
Decision: records/decisions/D-X/D-X.93.toml. Percall spec: validation/percall/setlocale.c.
Ruling #5298 (coordinator): no EnvMemo for setlocale(cat, ""); the environment is read on every call.

## SETLOCALE-CTYPE-ENV-6 (this run): re-measured on the window-bins layout; no code change needed

- The structural layout fix the brief waited for (LAYOUT-CLUSTER-WINDOW-ALIGN) was closed and replaced by
  LAYOUT-WINDOW-BINS (owner R1, #5743), which is on next (924c1eb59 merge). Merged origin/next 7288d847a.
- Step 0, inherited head merged: `cargo xtask clippy` + `cargo xtask static` green (job setlocale-ctype-env-6-lint0).
- Start-up bounds re-measured before any change (job setlocale-ctype-env-6-startup0, mei, Skylake-SP):
  `cargo xtask so && cargo test -p liella --test hot_text_products` and the same with `LIELLA_FULL_PROFILE=1`:
  26/26 passed in both profiles, including run 5's two reds (init_fork_child_stays_within_its_page_fault_bound,
  init_data_clusters_hold_at_every_text_phase). Nothing for this branch to remove; no bound moved.
- Gate: `cargo xtask affected --run` (direct tier) job setlocale-ctype-env-6-gate rc 0; `cargo xtask test` job
  setlocale-ctype-env-6-xtest rc 0 (6613 + 5230 passed; STARTUP-DATA-WINDOWS ok).
- Plants (handoff/SETLOCALE-CTYPE-ENV-5-plants.txt re-run on the merged head, job setlocale-ctype-env-6-plants):
  2 CAUGHT (setlocale_env_is_read_on_every_call), 1 EQUIVALENT (formatter-inlined, layout only), no WARN.
- Percall (job setlocale-ctype-env-6-percall, release, instr/call, base next 7288d847a -> head 010575ec4; glibc):
  ctype "" 8347 -> 568 (2551); ctype "" LANG 7147 -> 880 (6233); all "" 8891 -> 1083 (11401);
  all "" LC_ALL empty 9590 -> 4386 (21531); all "" LANG 8539 -> 3824 (21075); query 78 -> 71 (44);
  ctype C 149 -> 126; all C 144 -> 122; ctype POSIX 162 -> 139; all POSIX 157 -> 135. No row slower than next.

## Earlier runs (summary)

Run 5 found the coreutils-shaped start-up fault (the traced hot group grew past a 32 KiB boundary) and removed
it: values measured on demand (696e56fcf), lookup `#[inline(never)] fn env_lookup` and strftime's general
formatter `time::format::format` `#[inline(never)]` (e91db4e85). Runs 1-4: the on-demand environment read of
setlocale(cat, "") itself; reports in handoff/reports/SETLOCALE-CTYPE-ENV-*.json.

## Open / next step

- If the gate and xtest are green the branch lands; else run only the failing test.
- Optional: a cost bound in setlocale_env_repeat (counts are printed only); a cheaper LANG-only walk
  (ruling #5298 rules out a memo). setlocale_query is still 71 vs glibc 44 instr/call (not this story's row).

## Not to redo

The kept path is right for LC_ALL empty; #5188 is settled. Run 5's 51st fault is understood and fixed; the
start-up bounds pass on the window-bins layout, so don't re-profile them.

## Notes

- INFORMATIONAL: `cargo xtask so` warns `hyp_quick is never used` (crates/boundary/src/hot/hyperbolic.rs:94), from next.
- Superseded jobs: none this run (lint0 and startup0 ran on the merged head 010575ec4, which is the code head).
