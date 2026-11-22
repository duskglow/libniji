# LDSO-PROGRAM-OPTIONS-REST-2 report

Brief step 2 (the five small options) and step 5 (differences row 337 down to `--audit`), on top of run 1
(task/ldso-program-options-rest). Decision D-27.88 (amends D-27.80).

## What changed

- `ld.so --inhibit-rpath LIST`: DT_RPATH/DT_RUNPATH ignored for an object whose load path equals an entry exactly; an
  empty first entry names the program (`''`, `':'`); `'x:'`, a soname or a `..` spelling name nothing (probed).
  Checked in `image::search_list` only when the object has the tag.
- `--glibc-hwcaps-prepend` / `--glibc-hwcaps-mask`: the prepended names first, then the supported ones the mask names;
  empty names dropped; the prepended are not masked; the last copy of an option wins (probed). The list is built once
  in program mode (cold `entry::hwcaps_list`) and kept for `dlopen`, which searches it too (probed: the reference's
  dlopen loads from the prepended subdirectory).
- `--list-tunables`: libniji's `niji.*` tunables in the reference's line form, values from `GLIBC_TUNABLES` or the
  default. `--list-diagnostics`: libniji's `key=value` set (row 338). Both exit 0, need no program; the last mode
  option wins.
- Row 337 now refuses `--audit` alone; row 338 records the list contents (no compat option: the lists describe the
  running library). Row 338's reference column now names every key the reference has and libniji lacks (probe, `env
  -i`, indexes dropped: 75 key names vs 11): `uname.*`, `x86.cpu_features.*`, `dl_hwcap`..`dl_hwcap4`,
  `dl_hwcap_important`, `dl_hwcaps_subdirs_active`, `dl_dst_lib`, `dl_profile_output`, `dl_string_platform`,
  `path.prefix`, `path.sysconfdir`, `version.release`, `auxv.a_val_string`.
- Leg 3: tests for dlopen after `--glibc-hwcaps-prepend` and for `dl_platform` against the listed subdirectories (no
  test reached either); fuzz target `ld_cache` retyped to the `&[&[u8]]` names (static's fuzz-check was red).

## Start-up (perf user instructions, setarch -R, env -i, median of 51; before = run 1's products, after = edcbc1ab8)

| class | program | before | after | delta | job |
|---|---|---|---|---|---|
| Skylake-SP | empty main | 548,900 | 548,897 | -3 | mei `ldso-program-options-rest-2-count4` |
| Skylake-SP | three libraries | 797,794 | 797,635 | -159 | same |
| Zen 1 | empty main | 545,083 | 545,080 | -3 | keke `ldso-program-options-rest-2-zen-count4` |
| Zen 1 | three libraries | 793,869 | 793,710 | -159 | same |

No option adds instructions to an ordinary start; the hwcaps/platform retype (leg 2) takes the GOT slot and memcmp off it.
The ld.so code is unchanged after edcbc1ab8 (later commits: tests, docs, a fuzz target).

## Gate and checks (final code; mei role s4)

- Direct gate `ldso-program-options-rest-2-gate1`: 13 suites ok, 1063 passed, 0 failed, 1 ignored, SKIPPED 0, peak 2.9G.
- Lint `-lint6`: static and records check ok (clippy ok in `-lint5`; the gate ran clippy too).
- Plants `-plants1` (`cargo xtask plants`, 4 copies): 13 plants, 13 CAUGHT, 0 SURVIVED, 0 EQUIVALENT, 0 WARN.
- `cargo xtask test` `-xtest1`: rc 0 (nextest 7129 + 4875 + 128 + 46 passed, 0 failed).
- Full profile so + hot_text_products `-fullprof1`: rc 0 (hot_text_products 33 passed).
- Layout bins vs merge base 1d4d07280 (base products built in scratch, `-baseso`): per-bin byte totals identical
  (bins 0-8); the 42 differing lines differ only in `.llvm.<hash>` suffixes (0 lines once those are stripped). The
  change is in ld.so's code (feature `ld`), not libc's hot group.

## Rulings / informational

- INFORMATIONAL: `--list-diagnostics` could add the system-describing keys cheaply (`uname.*`, `dl_hwcap*` from the
  auxiliary vector); row 338 records their absence. Recommend a small follow-up if anyone uses the list for bug reports.
- Not this story: rtld-audit (`--audit`, LD_AUDIT, ruling #7514); the ld.so panic-location relocations (#7690).

## Next

Land (all five options match; row 337 is `--audit` only). Then rtld-audit as its own phase-sized story.
