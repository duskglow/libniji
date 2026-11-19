# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State (run 6 ended INCOMPLETE, needs a ruling).** Branch `task/dl-canonical-plt-6`, code head b6a56d27b
(then record/handoff commits). Role keke:~/w/n2 (Zen 1 host, 60 vCPU). Full run-6 report:
handoff/DL-CANONICAL-PLT-6-report.md. Run-5 history: handoff/DL-CANONICAL-PLT-5-report.md. Record:
`records/pending/DL-CANONICAL-PLT-2-1.toml` (design text = flag beside the table; Cost sentence = run-6 numbers,
marked NOT FINAL).

**Ruling #6618 (in force):** +1 ASLR-on fault median not accepted; move the flag out of `Object`, remeasure, add
the dladdr-at-load-base test. Run 6 did all three: faults now equal base (ASLR off: same pages; ASLR on median
78/94/130 = base), and the dladdr test catches plant `dladdr-any-undefined` (plants2: CAUGHT). BUT the flag beside
the table costs the plain path instructions: PIE without canonical entries +5137/+10471/+26796 at N=0/3/12
(run 5's flag-in-Object design: -82/+5/+288 on the same host). Exact single-step trace (N=0): ~9 instructions
more per symbol lookup (578 lookups): `resolve` +1734 (inlined `scope.entries` test), plain walk +2673,
`target` +578; the per-candidate closure is identical to base. Two tweaks (const generic `find`/`search`,
out-of-line `lookup_plain`) cut N=12 from +49.7k to +26.8k but cannot reach zero while the flag travels per lookup.

**RULING NEEDED (RUT RULE: two fixes of one kind):** how to carry the flag with no per-lookup cost. Options:
(b) keep it inside `Object` without growing it: encode "program with entries" in the `Hash` enum's spare
discriminant (`Object` stays 0x128 bytes, so no stack page; the test is again only on the rare non-exported
candidate path, run-5 instruction numbers); (c) monomorphise the relocation path on a const bool from
`relocate` down (plain copy byte-identical to base, about +12 KB cold ld.so text). Recommendation: (b).

**Things not to redo.** Reference probe (keke job dl-canonical-plt-6-probe-ref3; `probe/ref.expected`), the
dladdr load-base test (passes, catches the plant), base/run5/new3 product sets on keke
/dev/shm/claude/DL-CANONICAL-PLT-6/prod-{base,run5,new,new2,new3}.

**Tools on keke ~/work/DL-CANONICAL-PLT-6/** (scratch, not the tree): `step6.sh <tag>` (lint + products +
probe + loader tests), `count6.sh <tag> <A> <B> [runs]` (instructions/faults, ASLR off/on distributions),
`sstep.c` + `sstep_run.sh <count tag> <kind> <n>...` + `sstep_sum.py` (exact per-function user-instruction steps
by ptrace single-step: deterministic, matches perf stat to the instruction; use it, perf sampling is throttled),
`disdiff.sh <A> <B> <regex>...` (normalised disassembly diff).

**Exact next step.** Get the ruling; implement the chosen option; `step6.sh` + `count6.sh <t> base <new> 101` +
`sstep_run.sh <t> plain 0 12` must show plain within run-5 levels and faults equal base; then gate (direct,
`--base origin/next`), plants (`handoff/DL-CANONICAL-PLT-6-plants.txt`, update patterns), start-up checks
(`cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test
hot_text_products`), precheck, report.
