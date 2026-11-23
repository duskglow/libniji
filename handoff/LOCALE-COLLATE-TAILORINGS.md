# LOCALE-COLLATE-TAILORINGS (handoff, 2026-11-23): CHECKPOINT after step 1

State: branch `task/locale-collate-tailorings`, cut from origin/next 0495c4684; head = the commit carrying this note
(see `handoff/reports/LOCALE-COLLATE-TAILORINGS.json`). No library code changed: probes, data and this note only.

Why a checkpoint: the brief's steps 2-4 need the CLDR locale-source generator (LOCALE-SOURCES-GEN-2, queued for CI)
and LOCALE-PROBE-GEN-2 (held), neither on next. Challenge #7950; the coordinator agreed: finish step 1 as fully as the
budget allows, push the probes and results, hand back as CHECKPOINT; the continuation is a new story stacked on both.

## Done (step 1, plus the widened derivation)

* #7795 settled: on build hosts without the locales compiled, `setlocale` returns NULL and strcoll is C byte order.
  With the locales compiled (privately with LOCPATH, or installed), the reference applies tailorings. The two setups
  give identical results (md5 on keke and mei).
* The seven locales: sv_SE, es_ES, pl_PL and zh_CN are tailored. tr_TR is tailored (I and ı one letter, upper case
  first), not CLDR's way. ja_JP and ko_KR are own one-level models (hiragana before katakana; hanja after hangul).
* Survey of all 327 supported UTF-8 locales: 72 distinct orders; 206 identical to en_US; 16,151 tailoring rules in
  all over 320 locales; 7 own-order tables. Four weight levels; undefined code points sort first in three classes.
  36 locales have two-character contractions. The root equals DUCET 15.1 on the reference's defined set, apart from
  level-4 tie-breaks and about 1,150 placements (mostly Han variants).
* Everything is in `results/locale-collate-2026-11-23.md` (read it first) and `results/locale-collate-2026-11-23/`.
  Tools are in `tools/probe/collate/` (README).

## Decisions in force

None recorded (no D-row: no behaviour change). Basis: coordinator #7890 (follow the reference's order), owner
#7791 (locale data by probe). The #7791 allowance to view reference locale sources was not used.

## Open questions for the continuation

* Root: generate from `spec/unicode/15.1.0/allkeys.txt` plus the defined ranges plus about 1,150 rules, or commit
  the probed root? The first is smaller and traceable to a spec. Check that the level-4 tie-break is code point order
  before choosing it.
* The upper-first base (en_CA): a generated variant of the root, or rules?
* Own-order locales (ja, ko, zh_CN, cmn_TW, lo, km): full lists or rules?

## Not to redo

The survey: raw tables are reproducible in about 9 minutes (`survey_all.sh`). keke `~/work/locale-collate-tailorings/raw/`
holds them (gz, 80 MB) until cleaned.

## Exact next step (continuation story, stacked on LOCALE-SOURCES-GEN-2 and LOCALE-PROBE-GEN-2)

1. Rerun `collorder` for da, fo, is, kl, lt, lv, nb, nn and se with fillers chosen per locale (their level labels
   are unreliable; section 3 of the results page). Add the expansion probe and three-character contractions
   (section 5).
2. Generator: probe data to LC_COLLATE sections (root, then each locale as `reorder-after` rules plus
   `collating-element`s). libniji's localedef already parses every construct needed.
3. Tests per locale (the `collprobe` word lists, plus a sampled full-order comparison against `rules/`); plant: drop one
   tailoring.

Plan rows to read: none specific; the results page section 6 is the size estimate.
