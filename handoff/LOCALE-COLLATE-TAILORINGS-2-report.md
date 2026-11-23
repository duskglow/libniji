# LOCALE-COLLATE-TAILORINGS-2 report (run 1, legs 1-2, 2026-11-23)

Result: the shared root of the generated `LC_COLLATE` is built from DUCET 15.1 plus a 228-line rules file, and it
equals the reference's en_US collation for every code point and the corpus (coordinator decision #7958; D-31.32).
Per-locale tailorings and wiring into the generated sources are out of scope (#7998, #8003, #8008). They belong to
the stacked run.

## Commits (branch task/locale-collate-tailorings-2, on run 1's branch)

* Leg 1: the root probe, `crates/collate-root` (niji-collate-root), `spec/locale/collate-root.txt`, and the test
  `generated_root_collation_is_the_references_en_us` (several WIP commits).
* Leg 2: `tools/probe/collate/root_rules.py` (the derivation, with README row), results section 7, the record
  D-31.32, checked lookups in the generator (clippy `indexing_slicing`), plants, and this report.

## Test, gate and plant results (mei, role s3)

* lct2-lint2: `cargo xtask clippy` + `cargo xtask static`, rc 0. lct2-lint1 (`records check` passed, then clippy
  red on two map indexings) is superseded by the fix.
* Gate: `cargo xtask affected --list` escalates to FULL (job lct2-aff-list) on Cargo.toml,
  crates/collate-root/Cargo.toml, crates/locale/Cargo.toml and spec/locale/collate-root.txt, so FULL is the train's.
  Instead, lct2-tests ran `cargo xtask so` + `cargo test -p niji-collate-root -p niji-locale`: rc 0, every suite
  green. The generated_root test took 113 s; it reports 1,112,063 code points with 0 adjacent pairs differing and a
  clean corpus.
* Plants (`handoff/LOCALE-COLLATE-TAILORINGS-2.plants`): lct2-plants (plants-1) caught 10 of 11. The survivor was
  `after 1E9B A784 2` → 3, which is equivalent: the later `same A784 A785 2` rewrites the anchor's level 2, so the
  order and levels are identical. It is now declared `equivalent:` with that reason, and the level placement is
  proved on `after 1DE3 A75B 3` → 2 instead. lct2-plants2 (plants-2, the 12-plant file, rc 0): 11 CAUGHT, 1
  EQUIVALENT, 0 SURVIVED, no WARN lines.
* Derivation check (local, no job): `root_rules.py` on the en_US probe writes the committed rules file byte for
  byte. Its simulation leaves 0 order and 0 level mismatches; the one model-level difference is U+A784, which the
  `same` line settles. Exit 0.

## Probe-settled behaviour (results/locale-collate-2026-11-23.md section 7)

* Level 4: among characters equal at levels 1-3, code point order holds. The shifted-ignorable group (7,361 code
  points) is in code point order, not DUCET primary order.
* Class han-variant-block (1,508 code points) and class format-first (467 code points).
* Defined set: 50,709 code points in 127 undefined ranges. Only U+4E00-U+9FA5 of the unified ideographs are
  defined.
* Placements: 33 after, 1 same, 3 expand, 2 contract (l·, L·).

## INFORMATIONAL

* collseq (the order of bracket ranges in fnmatch and regex) differs from the reference: U+03B1 sorts before
  U+0041 there. The test prints the count without asserting it; the stacked run needs a probe of its own for it.
* Root contractions beyond l·/L· are not probed.
* The handoff from leg 1 said "22 after". The file has 33 after lines; section 7 and D-31.32 give 33.

## Refusals: none. Leftovers: none running (all jobs ended).

## Next

A new story, stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2 and sized L: emit the root into the generated
sources, add the 16,151 tailoring rules and the 6 own-order tables, the collseq probe, and per-locale tests
(handoff/LOCALE-COLLATE-TAILORINGS.md, "Exact next step").
