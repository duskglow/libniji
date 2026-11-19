# DL-CANONICAL-PLT-6 report (run 6: ruling #6618)

Result: INCOMPLETE. The flag is out of `Object`, faults equal base, and the dladdr load-base test catches the
surviving plant. The plain path still costs instructions, and fixing that needs a design ruling.

## Commits (run 6)
- 1c396a7ba Keep the canonical PLT flag out of the per-object lookup view (leg 1; flag in `Objects`/`Scope`/`Loader`; dladdr load-base test)
- 3a4a3d323 Monomorphise the canonical PLT lookup so the plain walk stays apart (const generic `find`/`search`)
- b6a56d27b Keep the plain symbol walk out of line behind the canonical test (`lookup_plain`)
- then: record Cost sentence (run-6 numbers, NOT FINAL), handoff and report.

## Checks on b6a56d27b (keke, job dl-canonical-plt-6-step3)
clippy (workspace, all targets, all features, -D warnings) and `cargo xtask static`: green. Products built; the
reference probe matches (including `dladdr_prog_base` / `dladdr_libdeep_base`); niji-ld tests canonical_plt 6/6,
reloc 22/22, symbols 9/9. NOT run on this head: direct-tier gate, the start-up checks, precheck (gate1 and
plants1 were started on 1c396a7ba and stopped when that head turned out slower).

## Counts (Zen 1 host, fast products, 101 runs each, against next 1b7335a10; instructions:u medians)
| program | base | run 5 (flag in Object) | 1c396a7ba | 3a4a3d323 | b6a56d27b |
|---|---|---|---|---|---|
| PIE plain N=0 | 8993723 | -82 | +6729 | +5137 | +5137 |
| PIE plain N=3 | 9272885 | +5 | +17319 | +11341 | +10471 |
| PIE plain N=12 | 10073582 | +288 | +49725 | +30330 | +26796 |
| non-PIE canonical N=0/3/12 | 9093421/9355165/10157220 | | | | +52273/+72658/+134243 |

(run-5 deltas are against the base column of its own count, job dl-canonical-plt-6-count1, m6.)
Faults, b6a56d27b vs base: ASLR off equal (76/92/129) and the same pages at N=0 (run 5 had one more stack
page); ASLR on medians equal 78/94/130 (N=0 distribution base 20/45/27/9 at 77/78/79/80 faults, new 19/46/29/7).
Canonical programs: +2 faults (cold canonical pass text), as in run 5. ld.so +13.4 KB.

Exact split (ptrace single-step tracer, job dl-canonical-plt-6-sstep1; total equals perf stat's +5137): plain N=0
has 578 symbol lookups: `resolve` +1734 (the inlined `scope.entries` test), `lookup_plain` +2673 compared with base's
`lookup`, `target` +578 (sets the Scope byte), `relocate_self` +90, `canonical_entries` +39. The per-candidate
closure (`find::<false>`) runs exactly as many instructions as base's. Plain N=12 (+26796 total): `lookup_plain` +23985
compared with base's `lookup` (about 2 more per object visited: the walk over the 7-word `Scope` compiles worse out
of line), `resolve` +1806, `target` +602, `canonical_entries` +183. Whatever carries the flag must leave `Scope`
and the walk exactly as base has them.

## Plants (job dl-canonical-plt-6-plants2 on b6a56d27b, stopped after plant 1 of 7)
dladdr-any-undefined: CAUGHT (`observations differ from the reference`): the plant run 5 left alive is
now covered. I stopped the job at hand-back so the role could sync for `cargo xtask static`, and because the
next design changes the planted lines. The other six plants did not run. The next run runs the plants file in full.

## RULING NEEDED
- How to carry "the program has canonical entries" with no per-lookup cost (RUT RULE: two tweaks of one kind
  failed to reach zero). (b) encode it in `Object`'s `Hash` enum spare discriminant: `Object` keeps its size
  (no stack page, which was the ruling's concern), the test is again only on the rare non-exported-candidate path
  (run-5 numbers: -82/+5/+288). (c) const-bool monomorphisation of the relocation path from `relocate` down
  (plain copy as base, ~+12 KB cold ld.so text). Recommend (b): it meets the ruling's aim (no fault, no stack growth)
  with run 5's measured instruction cost; it keeps the bit inside the view, which the ruling's literal wording excluded.

## INFORMATIONAL
- perf record on this host throttles (~160 samples per 10M-instruction start-up), so sampling can't locate
  0.3% differences; the single-step tracer is exact and takes seconds.
- Process slips this leg (nothing refused): one empty heredoc (`python3 - <<'X'` with no body, a no-op), and one
  `sed -i` on a scratch script (adding `--no-demangle`), not on the tree. Both are against the rules, and both are reported here.
- `kill-label` ended the gate1 job's wrapper but left its `cargo xtask affected` children; `kill-mine n2` ended them.

## Time
Most of the wall time went to three ~31-minute product builds (lint + `cargo xtask so` with trace at build).
Leg 2 started with the base and run-5 builds already done.
