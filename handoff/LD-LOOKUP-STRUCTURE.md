# LD-LOOKUP-STRUCTURE: symbolic relocations answered by (definer, index) - DO NOT LAND

**State (leg 2, 2026-11-23 ~03:45Z):** branch `task/ld-lookup-structure` from origin/next 3e668ecce; code head
e5d3c5746 (the commits after it are this note and the report). Role keke `~/w/n4` (Zen 1 host). Brief:
~/state/briefs/LD-LOOKUP-STRUCTURE.md. **Verdict: DO NOT LAND** - the change measured worse on every row, so it
stays here as evidence and is not proposed for next. Decision id D-27.90 was reserved and is NOT used (no record
written; nothing lands). Report json: handoff/reports/LD-LOOKUP-STRUCTURE.json.

## What the code does (no answer or error changed by design)
- elf.rs `Verdefs::find_hashed` (has_hashed's scan, answering the `vd_ndx`); `has_hashed` delegates to it.
- symbols.rs `Needs` (8 slots from the lowest requirement index: name + (definer strtab addr, def index)),
  `Object::reference_want_need` (a recorded index answers from the table, no memo; else as before).
- reloc.rs `version_requirements(set, i)` (cold; one verneed walk, provider found once per file, `definer` =
  find_hashed + `version_name` check), built in `relocate_first`, lent to `pass1` -> `one` -> `target`;
  `relocate_deferred` uses an empty table.
- Test crates/ld/tests/reloc.rs `reloc_requirement_pairs_bind_as_names_do` (green on keke, head 2).
- Plants: handoff/LD-LOOKUP-STRUCTURE-plants.txt (3). NOT RUN: not needed for an evidence-only branch.

## Numbers (Zen 1 host, release products, STARTUP-REPROFILE kit copy, median of 101, user instructions)
| row | next 3e668ecce | head 3 e5d3c5746 | change | glibc 2.39 |
|---|---|---|---|---|
| n0 empty main | 189,656 | 198,313 | +8,657 | 96,040 |
| n3 three libraries | 281,219 | 290,538 | +9,319 | 132,752 |
| t /bin/true | 247,549 | 259,837 | +12,288 | 122,349 |
| per lookup (n200-np0)/200 | 1,449 | 1,483 | +34 | 674 |
| faults n0 / n3 | 58 / 70 | 59 / 71 | +1 / +1 | 50 / 66 |
Jobs: keke `ld-lookup-structure-base` (base), `ld-lookup-structure-head3` (head 3, also clippy + static green).
Head 3 has no out-of-line `pass1` closure (heads 1/2 had one: +101k; fixed). `one`, `target` and
`Object::lookup` are within 0x30 bytes of base; the new `version_requirements` is 0xd94 bytes.

## Why it does not pay (what is measured vs. not)
- Measured: the fixed cost (builder, ~1k instructions per requirement: find_hashed + version_name over libc's
  ~45 verdefs, plus provider search) exceeds what n0's 51 lookups save; and the per-lookup cost ROSE by 34
  instead of falling the ~90 expected from skipping the memo hit + version-name compare.
- NOT VERIFIED: why per lookup rose. Checked and ruled out: the origin compare (`Dynamic::strtab` is a field
  read, same address on both sides). Zen 1 has no Intel PT and the sampled profile is unusable (leg 1), so the
  hot-path cost was not split.
- The brief's -38k/-58k/-64k (to ~675 per lookup) is not reachable by this step: after LD-LOOKUP-COST the
  per-lookup profile is flat; the memo already made the version part cheap.

## Open question (RULING NEEDED, RUT rule)
Second fix of the same kind (lookup cost) with little or negative gain: rethink, not a third tweak. Recommend:
split the per-lookup 1,483 on an Intel PT host (Skylake-SP) per phase (decode / hash walk / version / store)
before any new design; the remaining gap to 674 is not in version matching.

## Exact next step
None on this branch. If a rethink wants the code: the commits 1bcb3a656..e5d3c5746 are self-contained.
