# LDSO-PROGRAM-OPTIONS-REST report

Result: program mode's options cost nothing on an ordinary exec'd start again (empty main -137 / -138, three
libraries -197 on Skylake-SP / Zen 1 against origin/next); `--list` of a missing, unreadable, directory, short
or non-ELF file and running a non-ELF file end with the reference's form and status 127; dlopen after
`--library-path` and `--inhibit-cache` has an in-tree test. Brief step 2 (`--inhibit-rpath`, the two hwcaps
options, `--list-tunables`, `--list-diagnostics`) is NOT done: next story; its probe notes are in the handoff note.

## Commits

- 25a27d6ff Take program mode's options off the exec'd start; --list refusals as running
- 260006817 Expect the reference's text for running a non-ELF file in ldso_run
- 8c1a9ba1a Join --preload into LD_PRELOAD without panic sites
- this report, the handoff note and the JSON record

## Start-up counts

See the table in handoff/LDSO-PROGRAM-OPTIONS-REST.md (perf user instructions, setarch -R, env -i, median of 51,
min/max within 2 on Skylake-SP and within 50 on Zen 1). Jobs: mei `ldso-program-options-rest-count2`, keke
`ldso-program-options-rest-zen-count2`. The single-step profile (same jobs) attributes every step: before the
last commit `relocate_self` +234 (three panic locations from `split_at_mut`), `link` -128 / -188, `run` -7,
`niji_ld_start` -2; after it `relocate_self` is equal to base.

## Tests and jobs on the final code head (2b416b5a7; mei unless named)

- `ldso-program-options-rest-check3`: `cargo clippy --workspace --all-targets --all-features -D warnings`,
  `cargo xtask clippy`, `cargo xtask static`: clean.
- `-gate3`: direct tier (`affected --run --base origin/next`), rc 0, no SKIPPED lines, peak 842M.
- `-ldtests3`: preload, start_objects, ldso_search, search_cache, ldso_link (the loader tests the direct tier
  does not select but this branch reaches): 30 passed.
- `-startup3`: `cargo xtask test` (nextest runs 7,128 / 4,875 / 128 / 46 passed, 0 failed), full-profile
  `cargo xtask so` and hot_text_products: rc 0.
- `-count2` (Skylake-SP) and keke `-zen-count2` (Zen 1): products of 8c1a9ba1a; 2b416b5a7 changes a test only.
- Superseded: `-check2`, `-gate2` (rc 0), `-ldtests2` (ok), `-plants2` (7 caught, inhibit-ignored SURVIVED: the
  reason for 2b416b5a7), `-startup2` (stopped when the head moved), `-prof1` (stepper took the environment).

## Plants (handoff/LDSO-PROGRAM-OPTIONS-REST-plants.txt)

`-plants3`: 8 plants, 8 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, no WARN lines. Each names its test in `msg:`.

## Probe-settled behaviour

In the handoff note (Skylake-SP host, the installed 2.39 loader).

## INFORMATIONAL

- ld.so's self-relocation costs about 50 steps per relocated offset (`relocate_self`: 20,676 steps for 415 RELR
  offsets, both classes), and at least 113 of the 415 offsets are panic locations (link.rs 42, dlopen.rs 25,
  ld/src/init.rs 21, dlsym.rs 12, image.rs 9). Every panic site in ld.so costs every start about 50 user
  instructions; the queued LD-RELR-LOOP and a panic-site count for ld.so (a static bound) would both pay.
- `--inhibit-cache` has no in-tree observable on the test hosts (no libniji loader cache there), so the
  `inhibit-ignored` plant can only be caught where a cache exists.

## RULING NEEDED

None.

## Refusals

None.
