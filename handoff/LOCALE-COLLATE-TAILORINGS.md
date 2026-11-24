# LOCALE-COLLATE-TAILORINGS (handoff, 2026-11-24): runs -2 to -4 done, the shared root

State: branch `task/locale-collate-tailorings-4` = run -3's `task/locale-collate-tailorings-3` (run -2's shared
root plus a merge of origin/next c4a2b6934) plus a merge of origin/next deead7890 (run -4).
Head: the commit carrying this note. Role: shiki:~/w/sh2. Scope (coordinator #7998, #8003, #8008): the shared
root only, per ruling #7958.

Run -4 (CI conflict fix only, the second one): the train stopped on `crates/locale/tests/reference.rs` again,
because next appended `code_point_order_compiles_like_the_reference_c_utf8` at the end of the file, where this
branch's generated-root test was. As the coordinator asked (11-10: "resolve, then move the new cases into their
own module"), the conflict takes next's side. This branch's `Root` source and
`generated_root_collation_is_the_references_en_us` moved unchanged into
`crates/locale/tests/reference/collate_root.rs`. That file is a child module of `reference.rs`, declared once
near its top with `#[path]` (the pattern of `crates/abi/tests/appendix_c`), and it uses the parent's helpers via
`super::`. Cases appended to the end of `reference.rs` no longer touch this branch's lines. D-31.32's `tests`
names the new file. Results: handoff/LOCALE-COLLATE-TAILORINGS-4-report.md.

## Decisions in force

* D-31.32 (`records/decisions/D-31/D-31.32.toml`): the root is generated from DUCET 15.1 plus
  `spec/locale/collate-root.txt`. Levels 1-3 are DUCET shifted, level 4 is code point order. There are two class
  rules, and every placement carries its probe evidence. The proof is the generated_root test against the
  reference's en_US.
* Coordinator #7958 (generate from DUCET), #7890 and owner #7791 (locale data by black-box probe).

## Done (do not redo)

* Probes and the survey (run 1): results/locale-collate-2026-11-23.md sections 1-6, with tools/probe/collate/.
* Root probe: the en_US order over all of Unicode (results section 7). The tie-break is code point order,
  there are two class rules, and 127 undefined ranges.
* `crates/collate-root` (niji-collate-root): the generator, lib plus bin `niji-collate-root REPO OUT`.
* `spec/locale/collate-root.txt` (228 lines), derived byte for byte by `tools/probe/collate/root_rules.py`.
* Test `crates/locale/tests/reference/collate_root.rs::generated_root_collation_is_the_references_en_us`:
  green.
* Plants: `handoff/LOCALE-COLLATE-TAILORINGS-2.plants` (12: 11 caught, 1 equivalent), re-run on each merged
  head (reports -3 and -4).
* Gate: escalates to FULL (Cargo.toml and the new rules path), so it is the train's. The changed crates' tests
  run instead (`cargo xtask so` + `cargo test -p niji-collate-root -p niji-locale`), plus precheck.

## Open (for the stacked run)

* collseq (bracket-range order) differs: the reference puts U+03B1 before U+0041 although strcoll orders A
  first. The test prints the count without asserting it. Settling it needs a probe of fnmatch and regex ranges.
* Root contractions beyond l·/L· are not probed. DUCET has more (Cyrillic, Tibetan and others).
* Run 1's open items still stand: expansion levels, 3-character contractions, and per-locale fillers for
  da/fo/is/kl/lt/lv/nb/nn/se.

## Exact next step (new story, stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2; size L)

1. Emit the root as the shared `LC_COLLATE` of the generated locale sources, through niji-collate-root.
2. Apply the per-locale tailorings: 16,151 rules over 320 locales (`results/locale-collate-2026-11-23/rules/`).
   Re-derive them against this root, not the probed en_US. Most should stay as they are, since the root equals
   en_US.
3. Add the 6 own-order tables (ja_JP, ko_KR, zh_CN, cmn_TW, lo_LA, km_KH), the collseq probe, and a
   probe-backed test per locale group. Put the new tests in `crates/locale/tests/reference/` modules like
   collate_root.rs, not at the end of reference.rs.

Plan rows to read: D-31.32, D-31.29 (localedef defaults). The full report is in
handoff/LOCALE-COLLATE-TAILORINGS-2-report.md.
