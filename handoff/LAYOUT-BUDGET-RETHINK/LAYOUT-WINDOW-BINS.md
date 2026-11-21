# LAYOUT-WINDOW-BINS: start-up text in 64 KiB bins whose fault count no unrelated byte can move

Id: LAYOUT-WINDOW-BINS
Kind: code
Size: L
Model: opus
Effort: max
Waits for: the owner's ruling on `plan/layout-budget-rethink.md` R1 (and R2 for the gate it feeds).

## The change

Design: `plan/layout-budget-rethink.md` section 3 (pending record `records/pending/LAYOUT-BUDGET-RETHINK-1.toml`).
Replace the single first-touch sequence of the hot group plus power-of-two cluster alignment (`group()`,
`cluster_align` in `xtask/src/hottext.rs`) with a bin plan:

1. Trace-at-build records, per grouped section, its **signature**: the set of probe modes (and phase clusters) that
   execute it. Write it to `target/product/<profile>/layout-bins.tsv` (generated: bin, section, bytes, signature).
2. A planner packs sections into **bins of at most 64 KiB**, grouped by signature so that each probe touches as few
   bins as possible (a handful of signature groups: solve exactly, deterministically). The 0-fault phases (time,
   strings, rebind, float) are bins, or parts of a bin, that the first call already touches.
3. Every bin after the first starts on a 64 KiB boundary (`sh_addralign` 65536 on its first section). The bytes
   between a bin's end and the next boundary are filled with **cold sections** (no probe runs them; best fit by size,
   from the always-linked Rust object), named to sort between the bins. Residual fill is the only padding.
4. The head bin: implement both variants behind one constant and measure them: (i) 64 KiB-anchored head (counts
   identical for every program; up to 64 KiB - 1 of fill ahead of it in a static program) and (ii) unanchored head
   (no fill; the head bin's own count is 1 or 2 per program, fixed per program). The owner picks with the numbers.
5. Remove `cluster_align`'s power-of-two rule for clusters that share windows (it fixes a cluster's own count, not
   which clusters share a window: LAYOUT-TOOL-BOUND-SHARED).

## Proof

* The pad sweep of `handoff/LAYOUT-BUDGET-RETHINK.md` (nine insertion points, 0-64 KiB): with bins, no count may
  change at any pad that does not grow a bin past 64 KiB (`pre`, `cold`, `ro`, `relro`, `data`, `bss`, `lbss`, and
  `tier`/`mid` pads placed in filler positions). Today's next moves counts at most points (section 2.2 of the plan).
* hot_text_products both profiles: every bound green; record the one-off count change per mode (expected 0 or -1,
  NOT MEASURED) and tighten the bounds that drop.
* Size: the fill bytes per profile, and the static probe's file size before/after.
* Plants (`cargo xtask plants`): a bin overfilled by one section (the phase test fails); a filler left out (fill
  grows, counts unchanged: equivalent, by design); the head anchor dropped in variant (i) (the `pre` pad test fails).
