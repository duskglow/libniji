# LOCALE-COLLATE-TAILORINGS (handoff, 2026-11-23): run -2, the shared root (leg 1 done)

State: branch `task/locale-collate-tailorings-2` (on top of run 1's `task/locale-collate-tailorings`, cut from
origin/next 0495c4684); head = the commit carrying this note. Role mei:~/w/s3. Scope (coordinator #7998, #8003,
#8008): the shared root only, per ruling #7958; per-locale tailorings and wiring into the generated sources wait for
a run stacked on LOCALE-PROBE-GEN-2 and LOCALE-SOURCES-GEN-2 (neither on next).

## Done (leg 1)

* #7958 condition 1 (tie-break probe), answered by the coordinator: code point order holds except for explicit rules.
  Fresh probe on mei: `collorder en_US.UTF-8` over U+0001-U+D7FF and U+E000-U+10FFFF (8 s; the std range reproduces
  run 1's counts exactly). Findings: listing the reference order with DUCET 15.1 shifted weights reproduces the level
  of all adjacent pairs but 45 of 50,708 (23 are the known "level 5" expansion artifact). The shifted-ignorable group
  (7,361 cps) is in CODE POINT order at level 4, not DUCET primary order. Two class rules explain almost everything:
  han-variant-block (1,508 CJK radicals/compat/enclosed/kanbun ideographs form a block before U+4E00, in DUCET order)
  and format-first (Cc/Cf/Variation_Selector with all-zero DUCET weights first, cp order). Reference defines only
  U+4E00-U+9FA5 of the unified ideographs (ext A, B... undefined).
* `crates/collate-root` (niji-collate-root, lib + bin `niji-collate-root REPO OUT`): builds the root LC_COLLATE
  source from spec/unicode/15.1.0/{allkeys,PropList,UnicodeData}.txt + `spec/locale/collate-root.txt` (2 class
  rules, 127 undefined ranges, 22 `after`, 1 `same`, 3 `expand`, 2 `contract`, each class with its evidence).
  Workspace member + dev-dependency of niji-locale.
* Test `crates/locale/tests/reference.rs::generated_root_collation_is_the_references_en_us`: compiles the root with
  libniji's localedef library in-process and compares with the reference's compiled en_US: all 1,112,063 code points
  in reference strxfrm order, order + level (collorder's filler test) of each adjacent pair, plus the corpus (strcoll,
  wcscoll). GREEN on mei (job lct2-t5, 89 s): 0 pairs differ, corpus clean.
* Scratch (same run, $S = ~/w/.scratch/LOCALE-COLLATE-TAILORINGS-2/): derive.py (+ tiebreak2.py helpers, emit_rules.py)
  derived the rules from en_US.all.tsv; the expand/same/contract lines were then settled by the test (header says so).

## Open (record in the report, not fixed here)

* collseq (bracket-range order) differs: reference puts U+03B1 before U+0041 though strcoll orders A first; the test
  prints the count and does not assert it. Needs its own probe (fnmatch/regex ranges) in the stacked run.
* Root contractions beyond l·/L· (DUCET has many: Cyrillic, Tibetan...) are not probed; the corpus found only l·.
* run 1's open items (expansion levels, 3-char contractions, per-locale fillers for da/fo/is/kl/lt/lv/nb/nn/se) stand.

## Exact next steps (leg 2)

1. Commit the derivation as `tools/probe/collate/root_rules.py` (merge $S/derive.py + the helpers it imports from
   $S/tiebreak2.py + $S/emit_rules.py into one file; the rules file header names it) and add it to
   tools/probe/collate/README.md. Add section 7 to results/locale-collate-2026-11-23.md (the findings above; the
   rules file header cites "section 7").
2. Decision row: `python3 ~/src/liella-ops/bin/ops reserve --story LOCALE-COLLATE-TAILORINGS-2 D-31` and record the
   root design (DUCET shifted L1-3, cp L4, class rules, probe-backed test) citing #7958; read §10 of the
   developer-flow reference first.
3. On mei:~/w/s3: `cargo xtask clippy` and `cargo xtask static` (new crate: watch for codemap/traceability checks;
   the test's doc says `no spec clause`). Fix, commit.
4. Plants file `handoff/LOCALE-COLLATE-TAILORINGS-2.plants` (docs/plants.md format), e.g. drop
   `class format-first`, drop `class han-variant-block`, change one `after` level, make `shifted` keep variable
   weights; cmd `cargo test -p niji-locale --test reference generated_root`, msg
   `generated_root_collation_is_the_references_en_us.*(FAILED|panicked)`. Run with jobrun label lct2-plants,
   `--dir /mnt/zram/claude/locale-collate-tailorings-2/plants-1`.
5. Gate: jobrun lct2-gate, `cargo xtask affected --run` (direct tier; if FULL-escalated, follow the brief).
6. Rewrite this note, report JSON (`handoff/reports/LOCALE-COLLATE-TAILORINGS-2.json`, with "jobs"), push, precheck,
   `ops handback-check`, hand back. Remaining-work size for the stacked run: tailorings for 320 locales (16,151
   rules) + 6 own-order tables + collseq probe: size L (estimate in results section 6 still holds).

Jobs so far (mei): lct2-lint0 (clippy+static on the inherited head, rc 0), lct2-probe-root, lct2-so, lct2-t1..t5
(t5 = the green test run; t1-t4 superseded: compile errors and earlier rule versions).
