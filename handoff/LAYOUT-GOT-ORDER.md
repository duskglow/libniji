# LAYOUT-GOT-ORDER handoff (D-X.140): static programs have no `.got`

**State:** branch `task/layout-got-order` (base origin/next e5447b54f), role mei:~/w/s1 (Skylake-SP). Decision
**D-X.140** (reserved; `records/decisions/D-X/D-X.140.toml`, amends D-X.138). Plants: `handoff/LAYOUT-GOT-ORDER-plants.txt`.
Coordinator ruling #6935 (premise challenge accepted): mark the binop GOT relocations instead of reordering `.got`.

## What was found (link map of the static probe, base e5447b54f fast)

* `cc -static` uses GNU ld's default script: `.got` always follows `.data.rel.ro`; libniji cannot reorder it.
* `.got` = 0xc8 bytes (25 slots) at 0x86b110, 272 bytes into its page; start-up read 5 slots (refresh_words, ...).
* All 25 slots come from 160 plain `R_X86_64_GOTPCREL` on `add`/`cmp` operands (LLVM 23 emits plain GOTPCREL for
  binops even with `-Cllvm-args=-x86-relax-relocations=true`; checked with a two-function crate). GNU ld relaxes only
  `*GOTPCRELX`. A REX_GOTPCRELX binop to a target above 2 GiB makes ld 2.42 refuse the link (probe), not miscompile.

## What is in force

* `xtask/src/gotrelax.rs` (called in `so.rs::finish_archive` on the published archive after the weakening pass):
  byte-checked REX.W adc/add/and/cmp/or/sbb/sub/xor/test RIP-relative -> `R_X86_64_REX_GOTPCRELX`; build fails if
  0 marked. Result: 160 marked in 1 member, **0 plain GOTPCREL left**.
* `layout/contracts.tsv`: `.got:1` data bin removed; rw-budget fast 7 -> 6, release 8 -> 7 (release NOT yet verified).
* `tools/hottext/startup_data.py`: a `cmp` with an absolute address declares no object (the relaxed `fp == stdin`
  compare had put `_IO_2_1_stdin_` alone on page 0x88b000: .data1+.bss 4 pages > 3).
* `hot_text_products::init_static_probe_has_no_got_at_any_relro_size` (relro-tail pads 0/64/512/4000).

## Measured (fast, mei)

| | base e5447b54f | this branch 0f3963634 |
|---|---|---|
| static probe `.got` | 0xc8 at 0x86b110 | none |
| written start-up pages (STARTUP-DATA-WINDOWS) | 7 | 6 |
| `.data.rel.ro` bin | 6728 of 2 pages | 6728 of 2 pages |
| `.data1+.bss` bin | 11784 of 3 pages | 11784 of 3 pages |
| step-2 test relro-tail 64/512/4000 | - | no `.got` at any pad |

Jobs: `layout-got-order-baseso` (base so), `-so1`/`-so2` (superseded: unit-test fixture; stdin page), `-so3` (green
build + layout + step-2 test on 0f3963634; its counts part failed: stale execcount path), `-counts1` (paired execcount
static+dynamic, libc.so.6 identity, static-PIE reloc types: RUNNING), `-plants1` (RUNNING). Base and new products:
mei `~/work/LAYOUT-GOT-ORDER/{base,new}-fast`; scripts `~/work/LAYOUT-GOT-ORDER/{count,j4}.sh`.

## Exact next step

1. `~/bin/jobwait layout-got-order-counts1 layout-got-order-plants1`; read `~/logs/layout-got-order-counts1.log`
   (table for the report; dynamic must not be worse: libc.so.6 should be byte-identical) and the plants verdicts
   (append the `# cargo xtask plants results` lines to the plants file).
2. Then (one build at a time on s1): `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji
   --test hot_text_products` (release: rw 7 of 7 expected) and `cargo xtask test`; clippy -D warnings, `cargo xtask
   static`, `cargo xtask records check`.
3. Step 5: scratch merge of origin/task/thread-forced-unwind-2 into this branch (not pushed), sync to s1, `cargo
   xtask so` (layout gate green expected), then resync this branch.
4. Report json `handoff/reports/LAYOUT-GOT-ORDER.json`, precheck, handback-check, hand back.
