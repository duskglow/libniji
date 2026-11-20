# DYN-START-LINK-SHAPE: where the shipped libc.so.6 link's start-up cost goes

**State:** branch `task/dyn-start-link-shape` from origin/next b732d9b46 (the head measured below). Code commit
f19f46e5e (ld.so RELATIVE loop), then plants, record D-27.68, bound and this note. Build role mei s4 (Skylake-SP);
Zen 1 counts on keke with the same binaries. Scripts: mei `~/work/DYN-START-LINK-SHAPE/` (build.sh, relink.sh with
modes plain/defsym/ro1/relr, count.sh, pt.sh, profdiff.py, annot.sh, run1-4.sh). Scope (coordinator notes): the
read-only segment layout moved to LAYOUT-RO-STABLE; moving libresolv's code moved to LIBRESOLV-OWN-CODE.

## 1. Method: exact per-function attribution with Intel PT
`perf record -e intel_pt//u` of `env -i LD_LIBRARY_PATH=... PROG` (ASLR off), then `perf report --itrace=i1ie
--comms PROG --sort dso,sym -n`: the decoder walks every user instruction, so each is one sample. Exact, not
sampled: 0 trace errors in all runs; totals equal execcount's counter minus a constant 57-80 (env's environment
and the path, identical across sets). Limits: Intel only (the Skylake-SP host; the Zen 1 host has no PT), user
mode only, symbols as the products' symbol tables give them (inlined code counts to its caller). Per-instruction
counts: `perf script --itrace=i1i -F ip` counted per address, joined with objdump (annot.sh).

## 2. Attribution (Skylake-SP, release, execcount median of 101; instructions / faults)
| set | empty main | 3 libraries | clock_gettime |
|---|---|---|---|
| 20.1 (50abc44ed) | 781,900 / 59 | 1,033,274 / 75 | 815,840 / 63 |
| 20.2 (3f2a84520) | 785,385 / 57 | 1,036,759 / 73 | 819,325 / 61 |
| head b732d9b46 (shipped link) | 787,917 / 57 | 1,043,182 / 73 | 821,920 / 61 |
| head relinked by hand with the defsyms (control) | 787,917 / 57 | 1,043,182 / 73 | 821,920 / 61 |
| head relinked without the defsyms | 774,591 / 52 | 1,029,856 / 68 | 808,594 / 56 |
| head, `.rodata1` first (last run's attempt) | 784,137 / 57 | 1,039,402 / 73 | 818,140 / 61 |

Per-function differences above 100 instructions (PT; the same on all three rows unless noted):
* **head -> without defsyms: -13,321**, of which **-13,356 in ld.so `reloc::pass1`** (+110 `niji_machine_fill_sse2`);
  libc.so.6's own start-up code is identical (2,333 instructions on both).
* **20.1 -> 20.2: +3,487**: ld.so `Elf::read_phdr` +2,460, `map::plan` +236, `Elf::check_table` +235,
  `protect_relro` +161, `map::load` +138; libc.so.6 -77. All per-program-header work: 20.2's libc.so.6 has one more
  PT_LOAD (the read-only split), and `read_phdr` runs 58-63k instructions per start-up (it decodes the header table
  again on every walk).
* **20.2 -> head: +2,532 (empty main), +6,423 (3 libraries)**: module moves renamed most loader functions (link ->
  link::link, image_view, ...), so the table pairs renames; net growth: `read_phdr` +2,706, `check_versions` +4,649
  new against link's -3,724 (inlining moved), `dlopen::check` +529, `map::segment` +432.
* **head -> `.rodata1` first: -3,780**: `read_phdr` -2,583, `link` -266, `check_table` -235, `protect_relro` -161:
  one PT_LOAD fewer. On this host the attempt's 3-library row is -3,780 too, not +2,638 as on Zen 1 (last run):
  NOT VERIFIED why (handed to LAYOUT-RO-STABLE with this table).

## 3. The 13k and the 67 KB explained
The 67 defsyms (`__niji_fwd_<name>` = libresolv.so.2's exports) are GC roots: they keep libresolv's code alive in
libc.so.6 (+0x10880 = 67,712 B text, +0xEF0 .data.rel.ro, +67 dynsyms, +318 RELATIVE relocations: 1,777 vs 1,459,
one more RELRO page and a larger .rela.dyn: the 5 faults). None of that code runs at start-up; the whole cost is ld.so
applying the 318 extra entries at **exactly 42 instructions each** (13,356 = 318 x 42). All of it is ABI: libresolv.so.2
forwards to each alias, so no build-only fix keeps the exported ABI. The per-entry cost was the problem: 1,777 entries
in libc.so.6 alone = 75k instructions per start-up.

## 4. Fix: the RELATIVE loop (D-27.68)
The loop re-read the segment window through `Loads` (its `Cell` field cannot stay in a register across stores) and
updated the table position with a saturating add per entry. Now the window is a pair of locals refreshed only after a
miss (`Loads::window`), the position moves once per run. 42 -> 20 instructions per entry.

| program | CPU | 20.1 | 20.2 | head | fix | fix - head | fix, no defsyms |
|---|---|---|---|---|---|---|---|
| empty main | Skylake-SP | 781,900 / 59 | 785,385 / 57 | 788,080 / 57 | **749,900 / 57** | **-38,180** | 743,570 / 52 |
| 3 libraries | Skylake-SP | 1,033,274 / 75 | 1,036,759 / 73 | 1,043,427 / 73 | **1,005,260 / 73** | **-38,167** | 998,930 / 68 |
| clock_gettime | Skylake-SP | 815,840 / 63 | 819,325 / 61 | 822,083 / 61 | **783,901 / 61** | **-38,182** | 777,571 / 56 |
| empty main | Zen 1 | 781,710 / 59 | 785,195 / 57 | 787,722 / 57 | **749,542 / 57** | **-38,180** | 743,212 / 52 |
| 3 libraries | Zen 1 | 1,033,069 / 75 | 1,036,554 / 73 | 1,042,972 / 73 | **1,004,805 / 73** | **-38,167** | 998,475 / 68 |
| clock_gettime | Zen 1 | 815,679 / 64 | 819,164 / 62 | 821,754 / 62 | **783,572 / 62** | **-38,182** | 777,242 / 57 |

(Skylake-SP: 20.1 and 20.2 from run 1, head / fix / no-defsyms from one later run at a longer staging path, so
its head reads 163-245 above run 1's; compare within a run. Zen 1: all five sets in one run, same binaries.)
Every row is better than the head, 20.2 and 20.1, on both CPU classes; faults unchanged. Static programs: libc.a
holds no `reloc::pass1` (nm), so static start-up runs none of the changed code. The 318 defsym entries now cost
6,360 (20 each).

## 5. Options named (numbers, not built)
* **(a) DT_RELR for libc.so.6** (`-z pack-relative-relocs`; 1,777 RELA entries -> 75 RELR words, 288 relocations
  left): **+113,355** today, because the RELR loop calls `Loads::inside` per address (+78,144) and `Relrs::next` out of
  line (+51,031), against pass1 -17,695. Worth doing only after the RELR loop gets the window treatment (estimate:
  ~8-10 instructions per address, about -15-20k against the new RELA loop, and a .rela.dyn 42 KB smaller), NOT
  VERIFIED. ld.so owns both ends, so no ABI question for programs.
* **(b) libresolv's code in libresolv.so.2**: LIBRESOLV-OWN-CODE (owner 11-20). Measured upper bound: 6,360
  instructions and 5 faults per start-up of every process that does not load libresolv (fix vs fix-without-defsyms).
* **(c) the loop itself**: 20 instructions; the rest is two loop counters (chunks_exact + take), a spilled register
  and a null check in the boundary's `store`: ~3-5 more per entry (~5-9k) for a later pass.
* **(d) `Elf::read_phdr`**: 58-63k instructions per start-up and +~2.5k per extra PT_LOAD (section 2): the largest
  single item after pass1; a Loads-style cache of the decoded headers would remove most of it (INFORMATIONAL).

## Proof
Gate (direct tier) dyn-start-link-shape-gate1 green; plants (handoff/DYN-START-LINK-SHAPE-plants.txt) 0 survived;
start-up checks (`cargo xtask test`, release so + hot_text_products); counts above (jobs dyn-start-link-shape-fix1,
keke dyn-start-link-shape-zen1, run1 / pt2 / relr). Final-head jobs: see the report json.

**Next step:** land; then (c) and the RELR loop (a), and read_phdr (d), each measured with pt.sh.
