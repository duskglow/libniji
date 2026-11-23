# DL-CANONICAL-PLT-7 report (run 7: ruling #6659)

Result: COMPLETE. The flag rides in the program view's hash table (a
`u8` mark in the tail padding of `GnuHash` / `SysvHash`); `Object` and `Scope` keep base's sizes (compile-time
assertion); the plain PIE path is at or below base on instructions, and its faults equal base's.

## Commits (run 7)
- 187672a56 Carry the canonical PLT flag in the program view's hash table (run 5's shape, run 6's Scope byte,
  `lookup_plain` and const-generic find/search dropped; run 6's dladdr load-base test kept)
- 698260340 Test the canonical PLT flag on a SysV-hash program too (`--hash-style` gnu and sysv)
- a939e0bba Keep the canonical PLT test out of line on the rare candidate path (`#[cold]` `canonical_entry`)
- 54bda5888 + e5ca0f675 Keep the hash tables' mark a byte so the walk's layout stays base's (`bool` -> `u8`)
- 2103dccb0 Keep the canonical PLT test's symbol in registers and mark in place (`canonical_entry(i)`,
  `Object::mark_program(&mut self)`)
- 5f2a85053 and the hand-off commit: the record's Cost sentence (run-7 numbers), this report, the handoff.

## Why each step (single-step tracer splits, plain PIE, base = next 1b7335a10)
| head | N=0 | N=3 | N=12 | where (sstep) |
|---|---|---|---|---|
| run 5 (acceptance bar) | -82 | +5 | +288 | |
| 187672a56 (bool mark, test inline) | +3559 | +8963 | +25505 | all in `Object::lookup`'s body |
| a939e0bba (test cold) | +4490 | +9912 | +26509 | `Object::lookup` body +3534/+24954, unchanged to the instruction |
| e5ca0f675 (u8 mark) | +914 | +1045 | +1441 | lookup body equal base; closure +543/+603 (Sym stored to the stack to pass `&st`), `program` +120/+264, memmove of the 0x128-byte view |
| 2103dccb0 (index to the cold test, mark in place) | **-635** | **-489** | **+1** | closure -971, `mark_program` +47/+191, `relocate_first` mark test +40/+280 |

The `bool` mark's invalid values became `Hash`'s niche, so the discriminant moved (base: `cmp $2` on the byte at
0xe0; with the bool: `cmpb $2, 0x124`), and the chain loop gained a spill and reload per step. A `u8` has no niche.

## Final counts (2103dccb0; Skylake-SP host, fast products, 101 runs each, job dl-canonical-plt-7-step5)
| program | base median | new median | delta |
|---|---|---|---|
| PIE plain N=0 | 8993697 | 8993062 | -635 |
| PIE plain N=3 | 9273411 | 9272922 | -489 |
| PIE plain N=12 | 10075638 | 10075639 | +1 |
| non-PIE canonical N=0/3/12 | 9093616/9355860/10159443 | 9136733/9412413/10256296 | +43117/+56553/+96853 |

Faults, ASLR off: 77/93/129 equal (same pages but two anonymous ones 3 pages higher). ASLR on, distributions
(count at faults): N=0 base 10/42/41/8 at 77-80, new 3/42/49/7; N=3 base 7/79/14/1 at 93-96, new 8/59/29/5;
N=12 base 4/57/35/5 at 129-132, new 1/52/41/7; medians 78/94/130 -> 79/94/130, means +0.12/+0.22/+0.12, inside
base's own spread across this run's 101-run counts (N=0 median 79, 79, 78 in three counts; N=3 mean 94.36
and 94.09 in two).
Canonical programs: +2 faults (the cold canonical pass's ld.so text). ld.so 420224 -> 433384 bytes (+13,160).

## Gate and plants (code head 2103dccb0, synced tree of 5f2a85053; mei)
- dl-canonical-plt-7-gate1, rc 0, GATE-OK: `cargo xtask static`, `cargo xtask clippy`, `cargo clippy --workspace
  --all-targets --all-features -D warnings`, `cargo xtask records check`, direct tier `--base origin/next` 6828
  passed / 0 failed (786 skipped), `cargo xtask test` 5319 passed / 0 failed, `NIJI_FULL_PROFILE=1 cargo xtask so`
  and `hot_text_products` 27 passed, plain `cargo xtask so`. Peak memory 16G (page cache), oom_kill=0.
- dl-canonical-plt-7-plants1, rc 0: 8 plants, 8 CAUGHT, no WARN (results in handoff/DL-CANONICAL-PLT-7-plants.txt).
- dl-canonical-plt-7-step5: products, the reference probe (matches), niji-ld canonical_plt 6/6, reloc 22/22,
  symbols 9/9, the counts and the split above.
- Superseded (older heads): lint0, step1, base, count1, sstep1, step2 (a939e0bba and before), step3 (54bda5888,
  did not compile), step4 (e5ca0f675).

## Acceptance (ruling #6659)
- compile-time assertion `Object` 0x128, `Scope` 0x30 (base sizes): in symbols.rs, compiles.
- the flag is read only on the rare non-exported-candidate path (`#[cold] canonical_entry`) and once per object
  in `relocate_first` (whether to run the cold `canonical_slots`), as run 5 had it.
- plain PIE within run 5's -82/+5/+288 or better: -635/-489/+1.
- faults equal base ASLR off and on; canonical +2.

## INFORMATIONAL
- The `Hash` enum has no spare discriminant that fits a GNU-carrying variant: measured with a layout mimic, a
  4-variant `Hash` is 96 bytes against 88. The mark sits in the tables' tail padding instead, which meets the
  ruling's aim (no added fault, no stack page, nothing on the per-lookup path).
- A `bool` field added to a struct that an enum holds can move the enum's discriminant (niche) and change the
  code of every match on it; here it cost +25k instructions at N=12. Worth a line in the codemap or style notes.

## Process
- Leg 1: one empty heredoc (`python3 - "$S/t7/layout.rs" <<'EOF'` with no body, a no-op on scratch).
- Leg 2: 54bda5888 did not compile (two constructors still set `mark: false`); job step3 caught it at lint in
  seconds, e5ca0f675 fixed it.
