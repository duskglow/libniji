# LAYOUT-BYTE-BUDGETS: per-story gate on the bytes of each start-up bin, from the link map

Id: LAYOUT-BYTE-BUDGETS
Kind: code
Size: M
Model: opus
Effort: high
Waits for: LAYOUT-WINDOW-BINS landed, and the owner's ruling R2 of `plan/layout-budget-rethink.md` (it changes
what blocks a landing).

## The change

Design: `plan/layout-budget-rethink.md` section 4 (option G2).

1. `cargo xtask layout-check` (and the products step of `cargo xtask test`) reads `layout-bins.tsv` and the link
   map of the static probe it already links, and checks, per profile: each text bin's bytes <= 65,536; the start-up
   read-only bin <= its window; each written start-up bin <= its page count x 4,096. No program is run: the
   verdict is a function of the archive and the canonical start-up set.
2. A verdict names the bin, its bytes, its budget and headroom, and its largest members; with next's
   `layout-bins.tsv` at hand (the train keeps it per landed revision), the members that are new or grew, in bytes.
3. Every build prints one line per bin (bytes, budget, headroom), so the margin is visible before anyone crosses it.
4. The runtime fault bounds of `crates/core/tests/hot_text_products.rs` keep running per story as non-blocking
   reports (a mismatch against the bins' prediction prints the class and address); the train measures them, and
   the release ratchet tightens them (owner's ruling R2).

## Proof

* On next: every bin within budget; the printed headroom matches the link map (hand check of one bin).
* Replay the 11-14 cases on their merged trees: setlocale-ctype-env-4 must be red naming the setlocale functions
  and bytes; regcomp-fallible-alloc-2 green; tb-foreign-words-low-10 red only if its table is in a start-up bin
  under the canonical set (LAYOUT-CANONICAL-SET).
* Plants: a bin's budget check inverted; a section left out of the byte sum; the member diff dropped (verdict
  without names fails its message test).
