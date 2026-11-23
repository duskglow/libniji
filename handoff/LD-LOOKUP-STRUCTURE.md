# LD-LOOKUP-STRUCTURE: symbolic relocations answered by (definer, index)

**State (leg 1 -> refresh, 2026-11-23 ~03:20Z):** branch `task/ld-lookup-structure` from origin/next 3e668ecce.
Role keke `~/w/n4` (Zen 1 host, no Intel PT). Brief: ~/state/briefs/LD-LOOKUP-STRUCTURE.md. Decision id reserved:
**D-27.90** (not written yet: records/decisions/D-27/D-27.90.toml, shape as D-27.85). All commits are WIP-titled.

## What the code does now (no answer or error changed by design)
- elf.rs `Verdefs::find_hashed` (has_hashed's scan, answering the `vd_ndx`); `has_hashed` delegates to it.
- symbols.rs `Needs` (8 slots from the lowest requirement index: name + (definer strtab addr, def index)),
  `Object::reference_want_need` (a recorded index answers from the table, no memo; else as before),
  `Object::has_versym`, `Needs::records` (tests). `Request.origin` doc covers the requirement case.
- reloc.rs `version_requirements(set, i)` (pub, cold): one verneed walk (any error: empty table, old path);
  `provider_in_scope` (scope objects only: dlopen sets hold unloaded slots; load name first, then SONAME);
  `definer` (find_hashed, then checks `version_name(d) == name`, d > 1). Built in `relocate_first`, lent to
  `pass1` -> `one` -> `target`; `relocate_deferred` uses `static NO_NEEDS`.
- Test: crates/ld/tests/reloc.rs `reloc_requirement_pairs_bind_as_names_do` (pairs differ from the program's
  indices; fast == plain binding; preloaded unversioned f2 interposes). keke: reloc 29 + symbols 12 passed (head 2).
- Plants: handoff/LD-LOOKUP-STRUCTURE-plants.txt (3: wrong def index, find_hashed field, Want::Version for
  interposition). NOT RUN yet.

## Numbers (Zen 1, release, kit meas.sh copy at keke ~/work/ld-lookup-structure/kit, median of 101, user instr)
| row | next 3e668ecce (base-rel) | head 1 / head 2 (WIP) | glibc 2.39 |
|---|---|---|---|
| n0 empty main | 189,656 | 291,220 / 291,086 | 96,040 |
| n3 | 281,219 | 385,987 / 385,766 | 132,752 |
| t /bin/true | 247,549 | 354,979 / 354,854 | 122,349 |
| per lookup (n200-np0)/200 | 1,449 | 1,485 / 1,515 | 674 |
Head 1/2 +101k: the build moved `pass1`'s RELATIVE closure out of line (`pass1::{closure#0}` in nm; a call
per RELATIVE entry, 1,450 in libc). Head 3 (job `ld-lookup-structure-head3`, running at refresh: clippy, static,
release products, counts, hot_text_products count) builds the table in `relocate_first` instead; check with
`nm -S --demangle ~/work/ld-lookup-structure/head3-rel/ld-linux-x86-64.so.2 | grep 'pass1.*closure'` (must be none).
Faults 58 -> 59 on heads 1/2 (bigger frames); check on head 3.

## Judgement so far (read before more work)
Expected gain is small: the table saves the reference-side memo hit and the definer-side memo hit + version-name
compare (~90 per program lookup, ~20 per libc own-version lookup); the builder costs ~1k per requirement
(find_hashed + version_name over libc's ~45 verdefs). n0 has 4 program + 47 libc lookups: roughly break-even;
n200 / `/bin/true` should gain. The brief's -38k/-58k/-64k (reaching 675 per lookup) is NOT reachable by this
step: the per-lookup profile is flat (LD-LOOKUP-COST PT split). The sampled Zen 1 profile (kit/prof.sh) is
unusable (throttled; `env`'s glibc start pollutes it): use counts.

## Exact next step
1. `~/bin/jobwait ld-lookup-structure-head3`; read rows as above. If any row is worse than base: revert the
   code to origin/next (new commit, no history rewrite) or hand back DO NOT LAND with the numbers (rule: a change
   measured worse is reverted). If better on every row: continue.
2. Gate (`cargo xtask affected --run` under ~/bin/gate), plants (`cargo xtask plants handoff/LD-LOOKUP-STRUCTURE-plants.txt
   --dir /dev/shm/claude/ld-lookup-structure/plants-1 --timeout 600`), start-up checks (`cargo xtask test`;
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`), one
   build per role at a time. Tighten `DYN_START_INSTR` only if the level fell.
3. D-27.90 record, report json, precheck, handback-check. Jobs so far: ld-lookup-structure-base (base counts),
   -prof/-prof2 (profile, unusable), -head1, -head2 (superseded counts).
