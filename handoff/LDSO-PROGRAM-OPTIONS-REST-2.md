# LDSO-PROGRAM-OPTIONS-REST-2: the five small ld.so program-mode options (brief step 2, then step 5)

State: branch `task/ldso-program-options-rest-2` (on top of run 1's `task/ldso-program-options-rest`, base origin/next
1d4d07280), role mei `~/w/s4` (Skylake-SP class). Run 1's note (handoff/LDSO-PROGRAM-OPTIONS-REST.md) has the
start-up method. Decision **D-27.88** written (records/decisions/D-27/D-27.88.toml, `amends = ["D-27.80"]`; not yet
run through `cargo xtask records check`).

## Commits

- e60a3be48 (leg 1): the five options, `ld/report.rs`, `image::search_list`, test
  `ldso_program_inhibit_rpath_hwcaps_and_lists` (details: leg 1's list below).
- edcbc1ab8 (leg 2): hwcaps names carried as `&[u8]` (Arch::hwcaps, search::Ctx, Cache::lookup, program::find,
  hwcaps_subdirs, state/entry/report; tests retyped) and `X86_64::platform` = `hwcaps.len() >= 2`. Why: leg 1's
  `core::str::from_utf8` call in `hwcaps_list` added a GOT slot = one RELR offset every start (+36 steps in
  relocate_self), and platform's name compare was a memcmp call.
- leg 2 docs commit: D-27.88, differences row 337 shrunk to `--audit` + new row 338 (list contents, compat none),
  docs/tunables.md one sentence.

Leg 1 details: `crates/ld/src/program.rs` options (last copy wins), `Action::Tunables`/`Diagnostics`, helpers
`rpath_inhibited`, `hwcaps_subdirs`, `number`, `quoted_byte`; boundary `image::search_list` (inhibit check only when
the object has DT_RPATH/DT_RUNPATH); `List` carries `platform`, `hwcaps` (computed once in niji_ld_start);
`entry::hwcaps_list` (cold, mmap'd, kept in `state::program_hwcaps` for dlopen); `ld/report.rs` (cold).

## Start-up counts (perf user instructions, setarch -R, env -i, median of 51; head2 = run 1's products = cba05985c code)

| class | program | head2 | edcbc1ab8 (head4) | delta | job |
|---|---|---|---|---|---|
| Skylake-SP | empty main | 548,900 | 548,897 | -3 | mei `ldso-program-options-rest-2-count4` |
| Skylake-SP | three libraries | 797,794 | 797,635 | -159 | same |
| Zen 1 | both | | | | keke `ldso-program-options-rest-2-zen-count4` (RUNNING at leg end) |

e60a3be48's products (head3, superseded) measured +65 / -91: job `-count3`, profile `-prof3`. Step profile of head4
(e): niji_ld_start +78 (platform/hwcaps computed once there), `Arch::platform` -57, niji_machine_equal -15, link -10;
RELR offsets 415 = 415. Products: mei /mnt/zram/claude/ldso-program-options-rest-2/c1/{head2,head4}; keke
~/work/ldso-program-options-rest/c1/{head2,head4}. Scripts: mei ~/work/ldso-program-options-rest-2/ (count4.sh,
fndiff.py, relrnet.sh, probe_diag.sh).

## Probe (reference ld.so 2.39)

Leg 1: --inhibit-rpath exact load-path match, `''`/`':'` = program, space no separator. Leg 2 (`-prof3` log):
`--list-diagnostics` env value `a"b\c<TAB>d<0xe9>e`: reference `\"`, `\\`, tab as `\001`, 0xe9 as `\771`; libniji
`\011`, `\351` (octal of the value) -> row 338. Reference lists 41 glibc.* tunables. NOT YET VERIFIED by leg 2:
that the reference's diagnostics have uname and x86 keys libniji lacks (leg 1's note; row 338 says so) - check with
`... --list-diagnostics | cut -d= -f1 | cut -d. -f1 | cut -d[ -f1 | sort -u` on both, fix row 338 if wrong.

## Next steps (in order)

1. `jobwait` keke `ldso-program-options-rest-2-zen-count4`; add Zen 1 rows (must be <= head2).
2. Verify the diagnostics key claim (above); `cargo xtask records check` + `cargo xtask static` on mei s4.
3. Plants file handoff/LDSO-PROGRAM-OPTIONS-REST-2-plants.txt (docs/plants.md form; `cargo xtask plants`, dir
   /mnt/zram/claude/ldso-program-options-rest-2/plants-1): rpath_inhibited first-entry rule, search_list check,
   hwcaps_subdirs mask/prepend, hwcaps_list keep for dlopen, tunables value line, quoted_byte escape, Ask
   last-wins, platform len>=2 (search_cache / dlopen $PLATFORM tests).
4. Direct gate (`cargo xtask affected --run`), `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`, layout-bins diff vs merge base, precheck,
   report (handoff/LDSO-PROGRAM-OPTIONS-REST-2-report.md + reports json), handback-check.
