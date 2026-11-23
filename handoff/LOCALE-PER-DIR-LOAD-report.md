# LOCALE-PER-DIR-LOAD: report

Branch task/locale-per-dir-load, base origin/next 0495c4684 (nothing landed on next since the brief, so "merged with
next" holds). Decision D-7.21 (records/decisions/D-7/D-7.21.toml, amends D-7.7); spec Appendix D.1 amended.

## Commits (all on this branch)
- b08ef95cf Search locale directories in the order the reference does (mod.rs, locale_data, s07, spec, D-7.21)
- 5790b0304 Count start-up faults with the probes' own locale archive (hot_text_products)
- 73082e031 Contain the probes' locale namespace and add the search-order plants
- a9e89300f Name the host classes of the locale probes correctly
- WIP handoff commits (70474c538, 8151c3fb1) and this report.

## Behaviour settled by probes (reference rules in D-7.21)
1. LOCPATH unset: the archive is looked up under the name's archive form only (codeset normalized); a record holding
   the category decides.
2. Then the name's candidates (Appendix D.1 order), each in every LOCPATH directory and then in /usr/lib/locale,
   which is searched even when LOCPATH is set (the archive is not).
3. The first place holding the category file decides: a codeset mismatch there is "not found", no later candidate
   (de_DE.UTF-8@euro -> NULL).

## Probe table: setlocale, reference vs libniji (mei, Skylake-SP, per-locale directories, no archive)
Rows compared on the returned name and the loaded data (ALL, CTYPE codeset, MSG, TIME day name):

| table | rows | differ before | differ after | errno-only differences after |
|---|---|---|---|---|
| round 1, system layout (/usr/lib/locale dirs) | 31 | 2 | 0 | 1 |
| round 1, LOCPATH tree of made-up names | 39 | 2 | 0 | 3 |
| round 2, 9 layouts (LOCPATH + default, LOCPATH first, two dirs, missing category, missing LOCPATH, archive and dirs in a namespace) | 41 | 14 | 0 | 0 |
| round 3, archive + dirs in a namespace | 16 | no before table kept | 0 | 4 |
| coordinator #7885 (LOCPATH empty / missing / other tree; loc.c of PRINTF-NO-HEAP-3) | 20 | (NULL in libniji per #7885) | 0 | 18 (successful loads) |

Before: en_US.utf8 / en_US NULL in libniji, de_DE.UTF-8@euro accepted, LOCPATH never fell back to /usr/lib/locale,
a later LOCPATH directory's more specific name lost. After: every returned name and every loaded category agrees.
Errno-only rows: a failed name, reference errno 0 vs libniji ENOENT (existing D-7.18); a successful load after a
missed location, reference leaves ENOENT, libniji leaves errno untouched (errno is unspecified on success).
Outputs: mei ~/work/locale-per-dir-load/{after1,after2,after3,loc7885}.txt and out-{before,after}-*.txt.

## hot_text_products init_data_clusters (step 3, and coordinator note #7884)
Cause: the hot-text trace runs on the canonical data of tools/hottext/canon_host.sh (an archive holding en_US.UTF-8
only, redirected over /usr/lib/locale/locale-archive), so it never runs the directory search
(name::candidates / in_dir); the measured probes ran on the host's layout. On a host with per-locale directories
the measured runs mapped about 13 locale files and ran untraced code whose placement is chance: tool 58 / locale 57
faults (archive 44 / 43), and the tool count read 58 or 59 by text pad. Fix: the measured probes now run on the
same data model as the trace: a test-made archive (host localedef) bind-mounted on /usr/lib/locale in a user and
mount namespace (falls back to the host layout with a NOTE when localedef or user namespaces are missing). Bounds
unchanged. Measured, one static probe, true / locale / tool: mei host layout 10/57/58, contained 10/43/44; keke
(Zen 1, archive) 10/43/44 both ways; tool 44 at all 10 text pads.

## Start-up layout (§9): target/product/fast/layout-bins.tsv, merge base vs head
Base 0495c4684 built in a RAM scratch copy on mei (job locale-per-dir-load-base-so; the trace is host-independent,
canon_host.sh). Bins 0-8 total: +0, +559, -492, -39, +0, +0, +0, +52, -9 B (net +71 B; bin 0 identical). Traced
sections that changed are all locale: find_named 2274 -> 1364 B, global_locale::replace 3161 -> 1673 B, the
candidates instance of the archive path (3772 B) and Category::string (201 B) left the group (the archive lookup no
longer walks candidates), codeset_matches 289 B joined (signature locale,rebind,rebind.pre,tool): -6.1 KB of traced
locale text. Every other differing row is a `cold` filler; a few sections (getenv, atexit, bindtextdomain, ...)
moved from bin 1 to bin 2 at unchanged size. Output: mei ~/work/locale-per-dir-load/bins_diff.txt.

## Proof (jobs on mei, role ~/w/s1, head a9e89300f code)
- locale-per-dir-load-gate2: direct gate, 15 suites ok, 6529 passed, 0 failed, 76 ignored; conformance 2/0; peak 3.6G.
  Suites include locale_data, s07_locale_clauses, locale_products, catalogs_products, hot_text_products.
- locale-per-dir-load-plants (73082e031; mod.rs and locale_data.rs unchanged since): 5 plants, 5 CAUGHT, 0 WARN
  (handoff/LOCALE-PER-DIR-LOAD-plants.txt; includes the brief's plant, the per-locale directory path removed).
- locale-per-dir-load-xtask-test2 (`cargo xtask test`, a9e89300f): rc 0; 7188 + 4879 + 129 + 46 tests passed, 0
  failed (printf_corpus_products passed in both passes); fast layout contracts ok.
- locale-per-dir-load-fullprof2 (NIJI_FULL_PROFILE=1 so + hot_text_products, a9e89300f): RED, one contract:
  `WINDOW-BIN-BYTES BROKEN: mode tool: 2 windows < windows-release 3: the level improved; tighten the budget`.
  Fixed by commit 5ab4d3876 (windows-release tool 3 -> 2, recorded in D-7.21); rerun as
  locale-per-dir-load-fullprof3 on the final head.
- locale-per-dir-load-probe-after: the probe tables above.

Final head (5ab4d3876 code, handoff-only commits after it): the jobs above ran before the budget tightening, so
they are SUPERSEDED for the record (all green except fullprof2) and rerun on the final head:
- locale-per-dir-load-fullprof3: rc 0; every layout contract ok (release); hot_text_products 33 passed, 0 failed.
- locale-per-dir-load-gate3 (direct gate), locale-per-dir-load-plants2 (same plants file, RAM copy),
  locale-per-dir-load-xtask-test3 (`cargo xtask test`, starts when gate3 ends): still running at hand-back
  (11:04Z: plants2 4 of 5 CAUGHT, the fifth running; gate3 in its second pass, no failure so far).
- Precheck on 797ec2164: every check ok but the job records (the reruns above answer them); clippy and static clean.

## INFORMATIONAL
- WL gate start-up counts on mei now see per-locale directories (about 14 more faults per en_US.UTF-8 program than
  with an archive): a host data change, not a libniji change.
- Hosts with per-locale directories (Debian locales-all) run name::candidates / in_dir at start-up, which the trace
  never records (its data model is an archive). A second trace data model with directories would group that path;
  not done here (a layout story, not this one).
- Dynamic start-up probe reads 65 against DYN_TRUE_BOUND 73 with or without this change: a bound to tighten.
- Reference errno 0 vs libniji ENOENT on a failed name: existing D-7.18, not chased.

## Things not to redo
Probes (rounds 1-3 and #7885), plants, the hot_text cause analysis.
