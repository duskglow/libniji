# LDSO-PROGRAM-OPTIONS-REST-2: the five small ld.so program-mode options (brief step 2, then step 5)

State: branch `task/ldso-program-options-rest-2` (on top of run 1's `task/ldso-program-options-rest`, base origin/next
1d4d07280), role mei `~/w/s4` (Skylake-SP class). Leg 1 commit e60a3be48: the code and tests. Run 1's note
(handoff/LDSO-PROGRAM-OPTIONS-REST.md) has the start-up method and the earlier counts. Decision id reserved: **D-27.88**
(not yet written in plan/07-decisions/). Step 0 (clippy + static on the inherited head): green, job `-check0`.

## Done in leg 1 (e60a3be48)

- `crates/ld/src/program.rs`: `--inhibit-rpath`, `--glibc-hwcaps-prepend`, `--glibc-hwcaps-mask` options (values,
  last copy wins); `--list-tunables` / `--list-diagnostics` as `Action::Tunables` / `Action::Diagnostics` (last mode
  option wins, a program after them is not opened; `--list-tunables --list` = missing program name, all probed);
  `UNSUPPORTED` and `Action::Unsupported` removed; help text updated. Pure helpers: `rpath_inhibited`,
  `hwcaps_subdirs`, `number`, `quoted_byte` (all panic-free).
- boundary: `image::search_list` (DT_RPATH/DT_RUNPATH for a search; checks `state::inhibit_rpath()` only when the
  object HAS the tag) used in link.rs, trace.rs, dlopen.rs `scope_of`. `List` now carries `platform` and `hwcaps`
  (computed once in niji_ld_start, replaces `features` and the per-ctx `Native::hwcaps`/`platform` calls);
  program_mode builds `entry::hwcaps_list` (cold, mmap'd, kept in `state::program_hwcaps` for dlopen's ctx).
  `ld/report.rs` (new, cold): `tunables()` (libniji's 7 niji.* tunables, alphabetical, values from GLIBC_TUNABLES or
  default; unset thresholds print 0x0) and `diagnostics()` (dl_hwcaps_subdirs, dl_pagesize, dl_platform, dso.*,
  env[]/env_filtered[] with an allowlist (LD_*, MALLOC_*, LC_*, LANG, LANGUAGE, GLIBC_TUNABLES, LOCPATH, GCONV_PATH,
  TZ, TZDIR, NLSPATH), path.rtld, path.system_dirs[], version.libniji, version.version, auxv[]; no uname/x86 keys).
- Test `ldso_program_inhibit_rpath_hwcaps_and_lists` + parsing asserts; 9/9 pass on mei (job `-build1`).

## Probe (reference ld.so 2.39, Skylake-SP host; scratch probe_rpath.sh)

--inhibit-rpath matches the object's load path exactly (`$A/liba.so` yes; `liba.so`, `$A/../a/liba.so`, the
program's own path: no); `''` and `':'` inhibit the program's RUNPATH, `'x:'` does not; space is not a separator;
applies to objects found via --preload and --library-path too. --list-tunables: `name: value (min: x, max: y)`, sizes
hex, ints decimal, string tunables `name:` (nothing after when empty), 0. --list-diagnostics: `key=value`, strings
quoted, `[0xN]` indexes, env names not on an allowlist as `env_filtered[0xN]="NAME"`.

## Next steps (in order)

1. Start-up counts, empty main and three libraries, both classes (mei Skylake-SP, keke Zen 1), same method as run 1
   (`-count2` / keke `-zen-count2`, see run 1's report for the command), base = run 1's head cba05985c vs this head.
   Must be <= base (brief: no instructions added to an ordinary start). If it rose: check RELR count of ld.so
   (new panic sites) and the List hwcaps/platform change first.
2. `docs/differences-from-glibc.md` row 337: shrink to `--audit` alone; add a row for the two lists' content
   (libniji's tunables / diagnostics keys, octal escapes) with D-27.88, compat: none (content is libniji's by nature;
   say why). Write D-27.88 in plan/07-decisions/ (section 27 file) and update docs/tunables.md if it mentions
   --list-tunables. spec/behaviour/dynlink.md "Program mode options" if it lists the refused ones.
3. Plants (docs/plants.md form, `cargo xtask plants`): rpath_inhibited first-entry rule, search_list check,
   hwcaps_subdirs mask/prepend, hwcaps_list keep for dlopen, tunables value line, quoted_byte escape, Ask last-wins.
4. `cargo xtask static`, direct gate, `cargo xtask test`, full-profile so + hot_text_products, precheck, report json.
