# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State (run 7, leg 1 ended by REFRESH).** Branch `task/dl-canonical-plt-7`, code head a939e0bba. Role mei:~/w/s4
(Skylake-SP). Earlier runs: handoff/DL-CANONICAL-PLT-6-report.md, handoff/DL-CANONICAL-PLT-5-report.md. Record:
`records/pending/DL-CANONICAL-PLT-2-1.toml` (design text updated to the run-7 shape; its **Cost sentence still
holds run-6 numbers: replace it** with run 7's final counts).

**Ruling #6659 (in force; brief ~/state/briefs/DL-CANONICAL-PLT-7.md has the acceptance list).** Run 7 went back
to run 5's code shape (files restored from this story's own commit f96998fa8: `Object::program`,
`has_canonical_plt`, cold `canonical_slots`; base `Scope` and walk; run 6's Scope byte, `lookup_plain` and
const-generic find/search are gone), kept run 6's dladdr load-base test, and:
- The `Hash` enum has NO spare discriminant for a GNU-carrying variant (measured with a layout mimic on mei: a
  4-variant Hash is 96 bytes against 88; 3 variants with only an extra SysV one stay 88). So the flag is a
  `mark: bool` in the tail padding of `GnuHash` / `SysvHash` (elf.rs, `pub(crate) marked/mark`), read through
  the Hash match. Compile-time assertion in symbols.rs: `Object` 0x128, `Scope` 0x30 (base sizes; it compiles).
  Report this as INFORMATIONAL (the ruling's literal "spare discriminant" does not exist; the padding meets its
  aim), with the numbers.
- 187672a56 (flag read inline in the candidate closure): lint green, probe matches the reference, niji-ld tests
  canonical_plt 6/6, reloc 22/22, symbols 9/9. Counts (job dl-canonical-plt-7-count1, 101 runs, base = next
  1b7335a10 products /mnt/zram/claude/DL-CANONICAL-PLT-7/prod-base): plain PIE N=0/3/12 base 8993727/9273339/
  10075386, new +3559/+8963/+25505 (TOO HIGH); faults equal base (ASLR off 77/93/129 same stack pages; ASLR on
  medians 79/94/130 equal); canonical non-PIE +2 faults, +49.0k/+67.7k/+124.1k instructions. Single-step split
  (job dl-canonical-plt-7-sstep1): all in `Object::lookup`'s body (+3534 N=0, +24954 N=12): the flag read was
  hoisted to every lookup call.
- a939e0bba: the test moved into `#[cold] #[inline(never)] Object::canonical_entry` (called only for a
  non-exported candidate). Also: the non-PIE test now links the program with GNU and with SysV hash
  (`--hash-style`), since each style carries the flag on its own path (no other test had a SysV program with
  canonical entries).

**Running at refresh:** mei job `dl-canonical-plt-7-step2` (~/work/DL-CANONICAL-PLT-7/step2.sh: lint + products
new2 + probe + loader tests, then `count6.sh m7b base new2 101`, then `sstep_run.sh m7b plain 0 12`).

**Exact next step.** `~/bin/jobwait dl-canonical-plt-7-step2` on mei; read ~/logs/dl-canonical-plt-7-step2.log
(grep "plain N=\|canon N=\|probe\|test result\|LINT\|== plain" and the sstep top lines). Accept if plain PIE
is within run 5's -82/+5/+288 (or better) and faults equal base; if any N is worse, the sstep split names the
function (fix, recount). Then: update the record's Cost sentence (commit), sync, start the gate
(`~/work/DL-CANONICAL-PLT-7/gate.sh` under `~/bin/jobrun dl-canonical-plt-7-gate1 gate -- ~/bin/gate 16G sh
<script>`: static, clippy x2, records check, direct tier `--base origin/next`, `cargo xtask test`, full-profile
so + hot_text_products, plain so) and the plants (`handoff/DL-CANONICAL-PLT-7-plants.txt`, 8 plants, `--dir
/mnt/zram/claude/DL-CANONICAL-PLT-7/plants-1 --timeout 900`, label dl-canonical-plt-7-plants1); then precheck,
report handoff/DL-CANONICAL-PLT-7-report.md + handoff/reports/DL-CANONICAL-PLT-7.json, handback-check.

**Tools (mei ~/work/DL-CANONICAL-PLT-7/, ported from run 6; RAM dir /mnt/zram/claude/DL-CANONICAL-PLT-7):**
`step6.sh <tag>`, `prebuild6.sh`, `count6.sh <t> <A> <B> [runs]`, `sstep_run.sh <t> <kind> <n>...`, `disdiff.sh`,
`gate.sh`, `base7.sh` (base products, done), `layout.rs` (the layout mimic).

**Process notes for the report:** one empty heredoc slipped in leg 1 (`python3 - "$S/t7/layout.rs" <<'EOF'` with
no body, a no-op on scratch; reported). Jobs so far: lint0 (inherited head, green), step1, base, count1,
sstep1 (all rc 0; superseded by step2 as they measured 187672a56).
