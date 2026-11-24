# KERNEL-RESIDUAL-PROFILE: handoff

State: branch `task/kernel-residual-profile`, cut from `origin/next` c83bc9844; investigation only. The full
report is `results/kernel-residual/README.md`, and the probes are in `results/kernel-residual/probes/`. No product
code changed.

Measured on mei (Skylake-SP, kernel 6.8.0-149-generic), products `~/wl/products/0495c4684…-release-dyn` (the
PERF-GAP-STRUCTURE-AUDIT's base). Kallsyms copy `~/work/kallsyms-mei-6.8.0-149-generic.txt` (pass it as
`--kallsyms`; it is valid only for the boot it was copied in). Scratch outputs: mei `~/work/kernel-residual-profile/out1`,
`out3`.

## Findings (numbers in the README)

* About half the measured kernel excess comes from the measurement setup. libniji's `ld.so` and `libc.so.6`
  pages are mapped by no other process, so each measured process pays first-map and last-unmap page accounting.
  Tested both ways: the reference with unshared copies gains +88k to +175k; libniji with its pages held loses 63k
  to 187k.
* Real excess, pages held: +100k / +146k / +283k / +393k (empty, `true`, `ls`, `sed`). Causes: whole-file views
  (calls plus PTE teardown), written pages (audit path 4), more mapping calls. Offsets: no `ld.so.cache`, locale or
  `stat` calls.
* The exact replay of the mapping calls confirms the coordinator's F-03 note. Views are most of it; reservations
  per object are small (3k to 10k). F-04: the loader stack accounts for 48% of the empty `main`'s written-page
  excess (about 31k).
* Parity estimate: empty and `true` reach it; `ls` and `sed` keep 0 to +100k and +60k to +180k (anonymous written
  pages, `libc.so.6` text faults).

## Open

* RULING NEEDED: measure start-up comparisons with libniji's pages held (holder process); bounds would move.
* Next measurement: the writers of `ls`/`sed`'s anonymous pages (+28/+35 faults), by call chain.
* Candidate stories: view-free loader (item 1, largest), exact-size heap first region and views (item 2), initial
  thread adoption before initialisers (item 6).

## Do not redo

* Frequency-mode sampling (biased; use the per-run fixed, varied periods in `krp.py`).
* Comparing libniji and the reference without equal page sharing.
