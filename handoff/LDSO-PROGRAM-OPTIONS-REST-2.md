# LDSO-PROGRAM-OPTIONS-REST-2: the five small ld.so program-mode options (brief step 2, then step 5)

State: branch `task/ldso-program-options-rest-2` (on top of run 1's `task/ldso-program-options-rest`, base origin/next
1d4d07280), role mei `~/w/s4` (Skylake-SP class). Run 1's note (handoff/LDSO-PROGRAM-OPTIONS-REST.md) has the
start-up method. Decision **D-27.88** (records/decisions/D-27/D-27.88.toml, `amends = ["D-27.80"]`, records check ok).
Full report: handoff/LDSO-PROGRAM-OPTIONS-REST-2-report.md. Plants: handoff/LDSO-PROGRAM-OPTIONS-REST-2-plants.txt.

## Commits (this run)

- e60a3be48 (leg 1): the five options, `ld/report.rs`, `image::search_list`, test
  `ldso_program_inhibit_rpath_hwcaps_and_lists`.
- edcbc1ab8 (leg 2): hwcaps names carried as `&[u8]`; `X86_64::platform` = `hwcaps.len() >= 2` (no GOT slot, no memcmp
  on the start).
- 96f7b8082 (leg 2): D-27.88, differences row 337 shrunk to `--audit`, new row 338 (list contents, no compat option).
- d0dccd207 (leg 3): tests: dlopen after `--glibc-hwcaps-prepend` (reference probed: `dlopen:5`), `dl_platform` vs the
  listed subdirectories; the plants file.
- e7f8ea499 (leg 3): row 338 names every key the reference has and libniji lacks (probe: 75 key names vs 11).
- 90b8bc61b (leg 3): fuzz target `ld_cache` retyped to `&[&[u8]]` (static's fuzz-check was red after edcbc1ab8);
  D-27.88 key order.

Results on the final code (mei s4): direct gate `-gate1` 1063 passed / 0 failed; plants `-plants1` 13/13 CAUGHT;
`cargo xtask test` `-xtest1` rc 0; full-profile so + hot_text_products `-fullprof1` rc 0; layout bins = merge base
(suffixes aside); counts above (`-count4`, keke `-zen-count4`). COMPLETE.

## Decisions in force

D-27.88 (amends D-27.80), D-27.80 (refusals; now only `--audit`), differences rows 337 and 338.

## Not to redo

Probes (reference ld.so 2.39): --inhibit-rpath matching, hwcaps prepend/mask, tunables list, diagnostics quoting and
key set, dlopen after --glibc-hwcaps-prepend. Start-up counts (both classes) of the final ld.so code (edcbc1ab8; later
commits change tests, docs and a fuzz target only).

## Next step

Land. Then rtld-audit (`--audit`, LD_AUDIT; ruling #7514) as its own phase-sized story; optional small follow-up: the
system-describing `--list-diagnostics` keys (row 338). Plan rows: D-27.80, D-27.88.

## Start-up counts (perf user instructions, setarch -R, env -i, median of 51)

Before = run 1's products (cba05985c code), after = edcbc1ab8 (the final ld.so code). Skylake-SP empty main 548,900 ->
548,897, three libraries 797,794 -> 797,635 (mei `ldso-program-options-rest-2-count4`); Zen 1 545,083 -> 545,080 and
793,869 -> 793,710 (keke `ldso-program-options-rest-2-zen-count4`). Products: mei
/mnt/zram/claude/ldso-program-options-rest-2/c1/; scripts: mei ~/work/ldso-program-options-rest-2/.
