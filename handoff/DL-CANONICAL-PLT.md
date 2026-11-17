# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State.** Branch `task/dl-canonical-plt-3` (stacks on dl-canonical-plt-2; merge base next e035b3034; not
rebased). Code head 1cfd6cd07. Role mei:~/w/s3; scripts ~/work/DL-CANONICAL-PLT-3/ on mei (count.sh: equal-length
RAM copies, 41 single runs, ASLR on/off; pt.sh: lossless Intel PT per-function counts; step2.sh: static, clippy, so,
canonical_plt tests, count, pt). Decision: the pending record `records/pending/DL-CANONICAL-PLT-2-1.toml` (D-NEW,
series D-27); its body still describes the run-2 mechanism (Request::jump_slot): update it to the design below.

**Design now (run 3).** `target`, `one`, `pass1` and `Request` are back to their source on next. `Object::program`
(inlined) scans the program's hashed symbols once (GNU: symoffset..count; SysV: 1..nchain) and marks the view only
when a canonical PLT entry exists (`has_canonical_plt`); the lookup closure accepts such entries for that view only.
`relocate_first` and `relocate_deferred` test `syms[0].has_canonical_plt()` once per object and then go to the
cold `pass1_canonical` / `tables_canonical`: a `JUMP_SLOT` resolves via `slot_target` (re-resolves with the program
excluded when the first answer is an undefined symbol, i.e. the canonical entry) and the protected check
(`protected_canonical`) runs there; every other entry goes through `one`.

**Where the +5.5k went (run 3, measured).** Equal-length paths, 41 runs: the delta was real and deterministic
(N=0 +5458, N=3 +5574, N=12 +5910). Lossless PT trace: all in `reloc::target` (+6450; `one` -807, lookup closure
-545): the `jump_slot` argument and `Target.protected` caused register spills on every symbol relocation. A first
cut with the test inside `pass1` left +2686 (pass1 loop codegen +2877) and +2 faults.

**Counters at 1cfd6cd07 (job dl-canonical-plt-3-step3, ASLR off, min/med/max of 41):** instructions base -> branch
N=0 9028049 -> 9027904 (-145), N=3 9307696 -> 9307435 (-261), N=12 10109444 -> 10108761 (-683); faults N=0 82 -> 83
(+1, open), N=3 98 -> 98, N=12 134 -> 134. layout-bins.tsv and hot-text.signatures identical to base. PT diff:
pass1 now inlined into relocate_first; `init::init_order` newly out of line (+214 instr, new symbol): the likely
source of the N=0 fault.

**Next step.** (1) Remove the N=0 +1 fault: check `nm` for init_order / relocate_first placement vs the hot group
(target/niji/.hot-lists); keep pass1 out of line (`#[inline(never)]` on pass1, measure) or otherwise restore base
inlining; recount with step2.sh <new tag>. (2) Update the pending record body. (3) Plants: rerun
`handoff/DL-CANONICAL-PLT-plants.txt` adjusted to the new code (patterns naming `jump_slot`/`protected` field no
longer exist; add plants for the has_canonical test, slot_target's exclusion and the scan range) in a new dir
/mnt/zram/claude/DL-CANONICAL-PLT-4/plants-1. (4) Gate: ~/work/DL-CANONICAL-PLT-2/gate.sh (static, clippy, records
check, affected --base origin/next, xtask test, full-profile so + hot_text_products).
