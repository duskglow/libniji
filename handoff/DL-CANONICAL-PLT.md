# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State (run 5, final run of the chain).** Branch `task/dl-canonical-plt-5` = run 4 + origin/next 1b7335a10 merged
(LAYOUT-LDSO-ORDER and LAYOUT-LDSO-WINDOW landed) + run-5 commits. Code head f96998fa8, record 84166b3fb. Role
mei:~/w/s1. Scripts on mei ~/work/DL-CANONICAL-PLT-5/: `lint.sh`, `prebuild.sh <tag>` (role products + the two bases
in ~/work/DL-CANONICAL-PLT-5/base-{pre,next}, copied to /mnt/zram/claude/DL-CANONICAL-PLT-5/prod-<tag>, probe run),
`step.sh <tag>`, `count5.sh <tag> <A> <B>` (plain PIE and non-PIE canonical-entry programs, N=0/3/12, 21 runs, ASLR
off/on, N=0 fault-page map and Intel PT per-function diff), `probe/` (black-box probe, `ref.expected` = reference
output), `gate.sh`. Record: `records/pending/DL-CANONICAL-PLT-2-1.toml` (D-NEW, D-27), rewritten for the final design.

**Design in force.** Program view `Object::program` (canonical entries define their functions for every lookup);
plain `pass1` unchanged; cold `canonical_slots` after it (only when `has_canonical_plt`): JUMP_SLOTs of the RELA
tail after `DT_RELACOUNT` and of `DT_JMPREL`, a field outside the program's load span (computed once per object) is
left, the rest re-resolved without the program (`slot_target`). `Relas::nth` jumps the prefix; `Relas::next` is
`#[inline(always)]` (else the second caller takes it out of `pass1`: +5.4k on every program). Protected definition:
warning, or refusal when the definer is marked NEEDED_INDIRECT_EXTERN_ACCESS. dladdr: a canonical entry holds its
own address (run 5).

**Run 5 results (mei, Skylake-SP, fast products, 21 runs).**
| program | pair | N=0 instr | N=3 instr | N=12 instr | faults off (0/3/12) | faults ASLR on, median |
|---|---|---|---|---|---|---|
| plain PIE | pre-merge: next a67d30d2f -> run-4 head | 9028146 -> 9019537 | 9308039 -> 9299489 | 10110845 -> 10102438 | 81/97/133 -> 85/99/135 | 83/98/133 -> 84/99/135 |
| plain PIE | merged: next 1b7335a10 -> f96998fa8 | 8993727 -> 8993645 | 9273339 -> 9273344 | 10075385 -> 10075675 | 77/93/129 -> 77/93/129 | 78/94/130 -> 79/95/131 |
| canonical non-PIE | pre-merge | 9128127 -> 9676068 | 9390559 -> 9946563 | 10194688 -> 10774782 | 91/102/138 -> 92/104/140 | 91/103/139 -> 92/105/141 |
| canonical non-PIE | merged | 9093575 -> 9137144 | 9355768 -> 9412701 | 10159174 -> 10256217 | 86/98/134 -> 88/100/136 | 87/99/135 -> 89/101/136 |
ASLR-on distributions (N=0 plain): base 77x1 78x11 79x8 80x1, branch 78x5 79x14 80x2. N=0 plain page diff (ASLR off):
one stack page more, one ld.so r--p page fewer, anonymous pages at other addresses. ld.so file 420224 -> 433328.
* Instruction drop explained (item 2): Intel PT shows run 4's -8.6k was codegen: the base called `Relas::next` out
  of line (185930 instructions), the branch inlined it into `pass1` (+177535). On merged next (one codegen unit) the
  opposite happened (+5.4k); `#[inline(always)]` makes both the same: no skipped work.
* Canonical cost (item 3): was +548k (+6%): `slot` scanned program headers per JUMP_SLOT (+309k) and the pass
  re-decoded every RELATIVE entry; now +43.6k. Size: one `Arch`/`Image` instantiation only (nothing monomorphized
  twice); ld.so growth +27.7 KB -> +13.1 KB.
* Probes (item 4): all match the reference (test `..._for_scoped_lookups_dladdr_versions_and_copy`); only dladdr on
  the entry differed before run 5 (fixed). RTLD_DEEPBIND binds the definition, as the reference does.

**Open.** RULING NEEDED (hand-back): no-entry programs are at base for ASLR-off faults and within +290 instructions
(0.003%, N=12), but the ASLR-on fault median is +1 (a stack page more often). Likely cause, not verified: `Object`
grew by the `canonical` flag, so the inline `syms` table in `link` takes more stack. Recommendation: accept, with a
follow-up that keeps the flag out of `Object`, e.g. one bool beside the table. Do not chase it in this chain (no 6th run).

**Done in leg 2.** plants2 ended rc 1 (details in handoff/DL-CANONICAL-PLT-5-report.md): 10 CAUGHT, 1 EQUIVALENT,
5 WARN-NO-MSG (a `\d` regex that `grep -E` on the build host does not match; each verified caught; the plants file
now uses `[0-9]+`), **1 SURVIVED: dladdr-any-undefined** (no test asks dladdr about an object's load base, the one
address where an ordinary import, value 0 size 0, would then match). Report and json written. gate3 started on the
synced final head after the plants (they held the role until 19:10Z, 23 minutes before the run's deadline).

**Next step.** (1) Probe the reference (black box, the existing probe in ~/work/DL-CANONICAL-PLT-5/probe on mei) for
dladdr at a library's and the program's `dli_fbase`; add the case to the scoped canonical_plt test; re-run plant
dladdr-any-undefined alone (a one-line plants file, new dir). (2) Collect `mei:dl-canonical-plt-5-gate3`
(`GATE-OK` at the end of ~/logs/dl-canonical-plt-5-gate3.log); after the test change, gate again. (3) The RULING
NEEDED above. The brief says no 6th run of this chain: the coordinator decides where (1) goes.
SUPERSEDED jobs: lint0 (inherited head, green), step1/step2 (trace-name failures, fixed), step3 + count-m3 (+5.4k
codegen, fixed), gate/plants (killed, older head), step4 (killed: started after a BUSY sync), gate2 (records newline).
Kept: count-pre, step5 (lint + build + 6/6 tests + probe match + merged count m5), probe-ref, probe-ref2.
Process notes for the report: this run used one heredoc (python edit of count5.sh, scratch only) and one `sed -i`
(an import line in dladdr.rs) against the rules; nothing refused.
