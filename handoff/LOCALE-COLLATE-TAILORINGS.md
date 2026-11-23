# LOCALE-COLLATE-TAILORINGS (handoff, 2026-11-23): runs -2 and -3 done, the shared root

State: branch `task/locale-collate-tailorings-3` = run -2's `task/locale-collate-tailorings-2` (on run 1's
`task/locale-collate-tailorings`, cut from origin/next 0495c4684) plus a merge of origin/next c4a2b6934 (run -3).
Head: the commit carrying this note. Role: mei:~/w/s2. Scope (coordinator #7998, #8003,
#8008): the shared root only, per ruling #7958.

Run -3 (CI conflict fix only): the train could not merge -2 because next appended a transliteration test
(`strings_the_charmap_lacks_are_transliterated_as_the_reference_does`) at the end of
`crates/locale/tests/reference.rs`, where -2 appended `generated_root_collation_is_the_references_en_us`. The
merge keeps both tests; nothing else conflicted. Results: handoff/LOCALE-COLLATE-TAILORINGS-3-report.md. Per-locale tailorings and their wiring into the generated sources
wait for a run stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2, neither of which is on next.

## Decisions in force

* D-31.32 (`records/decisions/D-31/D-31.32.toml`): the root is generated from DUCET 15.1 plus
  `spec/locale/collate-root.txt`. Levels 1-3 are DUCET shifted, level 4 is code point order. There are two class
  rules, and every placement carries its probe evidence. The proof is the generated_root test against the
  reference's en_US.
* Coordinator #7958 (generate from DUCET), #7890 and owner #7791 (locale data by black-box probe).

## Done (do not redo)

* Probes and the survey (run 1): results/locale-collate-2026-11-23.md sections 1-6, with tools/probe/collate/.
* Root probe: the en_US order over all of Unicode. Results section 7 has the findings: the level-4 tie-break is
  code point order, there are two class rules, and 127 undefined ranges.
* `crates/collate-root` (niji-collate-root): the generator, lib plus bin `niji-collate-root REPO OUT`.
* `spec/locale/collate-root.txt`, 228 lines: 2 class, 127 undefined, 33 after, 1 same, 3 expand, 2 contract.
* `tools/probe/collate/root_rules.py` derives the file from the probe output. It reproduces the file byte for
  byte, and its simulation leaves 0 order and 0 level mismatches, so it exits 0.
* Test `crates/locale/tests/reference.rs::generated_root_collation_is_the_references_en_us`: green.
* Plants: `handoff/LOCALE-COLLATE-TAILORINGS-2.plants` (12: 11 caught, 1 equivalent); re-run on run -3's
  merged head (report -3).
* Gate: escalates to FULL (Cargo.toml and the new rules path), so it is the train's. The changed crates' tests ran
  as lct2-tests (`cargo xtask so` + `cargo test -p niji-collate-root -p niji-locale`), and precheck ran too.

## Open (for the stacked run)

* collseq (bracket-range order) differs: the reference puts U+03B1 before U+0041 although strcoll orders A first.
  The test prints the count without asserting it. It needs a probe of fnmatch and regex ranges.
* Root contractions beyond l·/L· are not probed. DUCET has more (Cyrillic, Tibetan and others).
* Run 1's open items still stand: expansion levels, 3-character contractions, and per-locale fillers for
  da/fo/is/kl/lt/lv/nb/nn/se.

## Exact next step (new story, stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2; size L)

1. Emit the root as the shared `LC_COLLATE` of the generated locale sources, through niji-collate-root.
2. Apply the per-locale tailorings: 16,151 rules over 320 locales (`results/locale-collate-2026-11-23/rules/`).
   Re-derive them against this root rather than the probed en_US; they should mostly be unchanged, because the
   root equals en_US.
3. Add the 6 own-order tables (ja_JP, ko_KR, zh_CN, cmn_TW, lo_LA, km_KH), the collseq probe, and a probe-backed
   test per locale group, extending the generated_root test pattern.

Plan rows to read: D-31.32, D-31.29 (localedef defaults). The full report is in handoff/LOCALE-COLLATE-TAILORINGS-2-report.md.
