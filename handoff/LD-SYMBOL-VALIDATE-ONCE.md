# LD-SYMBOL-VALIDATE-ONCE: the loader's symbol lookup over tables validated once at map time

**State (leg 2, 2026-11-23 ~19:30Z):** branch `task/ld-symbol-validate-once` from origin/next 3bf4b7342; code head
aea01b407, record head 0898cf520 (both pushed). Role keke `~/w/r5` (Zen 1). Full report:
`handoff/LD-SYMBOL-VALIDATE-ONCE-report.md`; record `handoff/reports/LD-SYMBOL-VALIDATE-ONCE.json`.

## Decisions in force
* **D-27.96** (this story): relocation symbols (`GLOB_DAT`, `JUMP_SLOT`, `64`) bind through a quick walk over
  tables `Object::build` validated once (`validate`, cold; result in `GnuHash::quick`, tail padding), versions by
  index (`reloc::Wants`, `symbols::quick_id`); anything unvalidated or uncovered goes to the exact walk.
* D-27.77 (LD-LOOKUP-COST): the exact walk and the pass memo stay as they were for every other lookup.

## Results (details in the report)
* Per imported symbol 1,525 -> 996 user instructions (reference 643). Target 300 not reached.
* Nine distribution binaries: user instructions down 8-19% each (ls 1.46 -> 1.18 of the reference, sed 1.34 ->
  1.10, grep 1.11 -> 0.95); kernel instructions and faults unchanged; outputs identical.
* Overlap: subsumes the LD-LOOKUP-RETHINK chain's D2 (versions by index) and D3 (flat path over a validated view);
  D1b (walk-level Bloom, on `origin/task/ld-lookup-rethink-2`, held) touches `Object::lookup` and the exact path only:
  no textual overlap with this branch's quick path; its gain still applies to dlsym and the exact walk.

## Not to redo
* The review of the quick walk against `Object::lookup` / `symbols::lookup` (two divergences fixed in aea01b407).
* The profile: exact single-step split of the 996 (report, "Where the remaining 996 per import go").
* Measurement kit on keke `~/work/ld-symbol-validate-once/`: `prod.sh <tag>` (release products copy),
  `mtag.sh <tag> [imports|all]` (wl_dyn_layout + `meas.py`), `trace2.sh`/`trace3.sh` (single-step splits).

## Exact next step
If this branch lands: a continuation toward 300/symbol (report, "Leftovers / next"): the walk without definition
copies or re-checks, a two-byte-step name hash, a leaner relocation decode; measure with `mtag.sh` on the same
probe. Read D-27.96, D-27.77 and the audit's section 3.1 first.
