# headers (SL-HEADERS-PLAN, -2), handoff

**State:** branch task/sl-headers-plan-2 (stacked on task/sl-headers-plan), head: see the report json.
Planning story: no code changed.

**Deliverables**
* `plan/headers.md`: §0 owner principle (2026-11-14), inventory, generation, proof, slices, §5 rulings
  (all answered 2026-11-14), §5.1 the legacy headers under R-H7.
* `plan/headers-inventory.tsv`: 478 header names with section and slice; the 27 legacy rows say
  ship (-dev include), ship (live purpose) or undecided.
* `handoff/SL-HEADERS-PLAN/SL-HEADERS-1..20.md`: the slice briefs, updated for the rulings.
* `records/pending/SL-HEADERS-PLAN-1.toml`: the pending decision (D-NEW, series D-X), one paragraph.

**Decisions in force:** owner rulings R4, R-H0..R-H7 and the owner principle (plan/headers.md §0, §5):
the ABI of what is built is exact; source compatibility is best effort, a small source change
(Aqours patch or upstream fix) beats imitating glibc further; `__LIELLA__` is always defined;
`__GLIBC__` and friends are defined too; only the `__USE_*`/`sys/cdefs.h` names SL-HEADERS-1 finds.

**-2 run**
* Fixed the red: the pending record's `body` had bullets (newlines in a table cell); now one paragraph.
* R-H7 evidence: all 12,608 `-dev` packages of the excalibur mirror (chain verified on keke: gpgv
  InRelease, SHA-256 of Packages.xz, Sources.xz and each .deb), libc `-dev` packages excluded, gnulib
  members unextracted, names only. 11 legacy headers are included by some package (ship); with
  `thread_db.h` and `proc_service.h` that is 13 shipped. 14 are undecided: SL-HEADERS-1 (b) scans the
  whole source archive (37,587 packages, 98.1 GB) for them. Scripts were scratch (not committed);
  slice 1 builds the tool in `validation/`.
* Checks: keke jobs sl-headers-plan-2-static0 (inherited head, red as reported),
  sl-headers-plan-2-size, sl-headers-plan-2-devscan, sl-headers-plan-2-refine, and the final static
  and clippy jobs named in the report json.

**Not to redo:** the inventory (verified chain), the -dev scan (§5.1 table).

**Next step:** dispatch SL-HEADERS-1 (now M: corpus scan plus the R-H7 source scan) and SL-HEADERS-2
(R-H0 granted, no longer blocked). Then 3, 4, 5 in order (plan/headers.md §4).

**Not verified:** no reference-header probe was run; `__isoc*`/`_chk` mappings are inferred from the
exports. The -dev hits include conditional and possibly non-Linux includes (counted, as the ruling's
wording says "includes it").
