# DYN-START-BISECT-1120: the +3,485 instruction dynamic start-up regression, 20.1 -> 20.2

**State:** branch `task/dyn-start-bisect-1120`, **DO NOT LAND** (evidence). Code commit 63e908556 (fix attempt),
5dc6d6b5d (record D-X.142, plants), then this note. Decision record D-X.142 (reserved) describes the attempt; if the
branch never lands the id stays unused. Build role keke n4 (Zen 1 host). Scripts: keke `~/work/DYN-START-BISECT-1120/`
(build.sh, count.sh = RELEASE-SNAPSHOT-1120B's startup.sh over N product sets copied to equal-length paths,
patchret.py, relink.sh, ro1first.ld, symdiff.sh, prof*.sh, profdiff.sh, chain*.sh).

## 1. Bisect (execcount, ASLR off, median of 101, release products, Zen 1 host)
Release products built in `~/wl/src/<40-hex sha>` (equal-length paths). User instructions / faults:

| tree | empty main | 3 libraries | clock_gettime |
|---|---|---|---|
| 50abc44ed (20.1) | 781,748 / 59 | 1,033,122 / 75 | 815,717 / 64 |
| b2485a7f1 (all merges up to test-reloc-etxtbsy) | 781,748 / 59 | 1,033,122 / 75 | 815,717 / 64 |
| 3f2a84520 (20.2: + asan-main-thread merge 458e9c223, views) | 785,233 / 57 | 1,036,606 / 73 | 819,202 / 62 |

The whole +3,485 (and -2 faults) is the **asan-main-thread merge** (code commit c723d5d78 per its handoff; others'
history not read). What it runs at start-up costs almost nothing: with `carry_initial_words` patched to return at
once in a copy of 20.2's libc.so.6 the count drops by **8** (job abl-1): no registry exists before `main`.

## 2. Cause found (part of it): a read-only segment split in libc.so.6
The merge grew libc's `.text` by 0x900 bytes. `.rodata1` (start-up read-only group, D-X.62/D-X.133) is 64 KiB
aligned and GNU ld's default script puts it after `.rodata`; `.rodata`'s end moved from 2 KiB below a 64 KiB boundary
to just past it, the gap became 62 KiB and the linker began a **second read-only PT_LOAD** at `.rodata1`
(12 -> 13 program headers). Gap on other keke builds: 92 B (19.2), 2,204 B (20.1, b2485a7f1, 6351dc969e) one segment;
55-63 KiB (3f2a84520, a1f6ccc837, 854da9f38d) split. It comes and goes with text size; current next splits.

## 3. Fix attempt and why it does not land
`xtask/src/so.rs`: a fragment (`libc-rodata1.ld`, `INSERT BEFORE .rodata`) puts `.rodata1` first in both libc
links; `cargo xtask so` checks one read-only PT_LOAD after the text starting on the group's alignment; unit test
`libc_so_read_only_data_is_one_segment_led_by_the_startup_group`. Paired count, fix built in `~/wl/src/5dc6d6b5d...`
(count-3; the role build, startup-1, gave identical numbers):

| program (Zen 1) | 20.1 | 20.2 | fix | fix - 20.2 | fix - 20.1 |
|---|---|---|---|---|---|
| empty main | 781,748 / 59 | 785,233 / 57 | 783,979 / 57 | -1,254 | +2,231 |
| 3 libraries | 1,033,122 / 75 | 1,036,606 / 73 | 1,039,244 / 73 | **+2,638** | +6,122 |
| clock_gettime | 815,716 / 64 | 819,202 / 62 | 818,012 / 62 | -1,190 | +2,296 |

Target (at or below 20.1) not reached and the 3-library row is worse: DO NOT LAND. Commit 63e908556's message
attributes all of the 3,485 to the split; the measurement above shows only about 1,250 (corrected in so.rs and D-X.142).
Relink experiment (relink-1: libc.so.6 relinked by hand from 3f2a's inputs without the NIJI_PRIVATE defsyms, default
order vs `.rodata1` first, both one segment): identical counts (768,350 / 1,019,724 / 802,318, 52/68/57 faults), so
the order alone costs nothing there; why the real build's 3-library row rises is NOT VERIFIED.

## Not done / open
* Skylake-SP and static counts not run (only Zen 1; the change touches only libc.so.6's link). Gate-1 and plants-1
  were started on 5dc6d6b5d and stopped when the count showed DO NOT LAND; `cargo xtask test` not run.
* Profiling: `perf record` on keke is capped at 2,000 samples/s (about 8 per start-up), so sampled profiles of a
  start-up are biased (prof-1/prof-3 unusable). The remaining ~2,230 instructions (and the 3-library rise) are
  likely address- or layout-dependent work in ld.so (the biased profile pointed at `read_phdr`, `check_table`,
  `Object::new`, `reloc::target`, `map::load`), not verified.
* INFORMATIONAL: the hand relink without the NIJI_PRIVATE defsyms has 67 KB less text and starts with about 13k fewer
  instructions and 5-7 fewer faults than the shipped link; not investigated (may be code only the forwarding aliases keep).

**Next step:** attribute 20.1 vs 20.2 vs fix per function deterministically (Skylake-SP host: `perf record -e
intel_pt//u` gives an exact user trace; or per-function ablation of ld.so as patchret.py did for libc), then decide
between this placement and a fix in ld.so's per-segment or per-address work.
