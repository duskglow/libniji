# LAYOUT-LDSO-WINDOW(-2): ld.so growth no longer moves its start-up pages

State: branch `task/layout-ldso-window-2` (continues `task/layout-ldso-window`, origin/next de4cdfc8d merged in),
role mei:~/w/s1. Briefs: ~/state/briefs/LAYOUT-LDSO-WINDOW.md (spec), ~/state/briefs/LAYOUT-LDSO-WINDOW-2.md.
Record: records/pending/LAYOUT-LDSO-WINDOW-1.toml (D-NEW, series D-X; body now one line, which fixes gate1's static
red "body (a table cell) has a newline"; parts (1)-(3) as below; part (3)'s claim still needs proof3's numbers).
Scripts on mei: ~/work/LAYOUT-LDSO-WINDOW/{proof.sh,count.sh,pagewhere.sh,pad.patch}, ~/work/LAYOUT-LDSO-WINDOW-2/
{relproof.sh (release pair), bsswhere.py <count dir> <ld.so> (touched ld.so .bss pages -> symbols)}.

## Commits on this branch (after 395a85303, the span bucket)
* 9ce45257f `LD_RUSTFLAGS` = `-Crelocation-model=pie\x1f-Ccodegen-units=1` (xtask/src/so.rs).
* (next) `check_ld_units` / `split_unit_members`: so refuses an interpreter archive with `ld_so-*-cgu.<n>` n>0
  members; unit test `interpreter_archive_units_beyond_the_first_are_reported`.
* 3e2532328 `LDSO_COLD_BSS` = 4096: run_ld copies the raw archive to target/product/ld-coldbss.a and runs
  `coldbss::move_cold_bss(.., LDSO_COLD_BSS)` (now takes `min`; libc passes `coldbss::MIN`); span script
  `INSERT AFTER .lbss`.

## What the jobs showed (mei, Skylake-SP, faults N=0/3/12 ASLR off; instr N=0)
* gate1 (old head 7aaab5365): clippy ok, static red only on the record (fixed above). plants1: 3/3 CAUGHT.
* proof-mei (fast, base 18cf8): next 79/95/131; fix 79/94/130; fixpad 82/98/134. The pad's extra faults were NOT
  text: fix vs fixpad by mapping = ld.so rw-p 3->4 and anon 17->19. Cause: fast (16 units, thin LTO) re-partitions
  the crates; fixpad's .data.rel.ro +0x70 (per-unit anon.* copies) pushed RELRO 0x1f98->0x2028 (>8 KiB, +1 rw page),
  and .bss input order changed (THREAD_MODS, readers::TABLE, 20 KiB each, moved between start-up statics, +2 anon).
* relproof (release = fat LTO, 1 unit, base 18cf8, old head without the cgu flag): relnext 76/92/128,
  relfix 76/92/128, relfixpad 76/92/128; instr 8194955/8194955/8194958. Sections equal in size, tables at the same
  offset in .bss: release was already insensitive. ASLR-on medians N=0 77/76/77 (next's two slots both 77: noise).
* proof2 (fast, merged base, head 9ce45257f = span + one unit): next 79/95/131 instr 9038321; fix 80/95/131 instr
  9003877 (-34k, -0.4%); fixpad 80/95/131 instr 9003877: INSENSITIVE. But N=0 +1 vs next: ld.so .bss pages 3 vs 2
  (bsswhere: groups at START_OBJS.., after THREAD_MODS, after readers::TABLE) -> commit 3e2532328.

## Running at refresh
* `layout-ldso-window-2-proof3` (mei): proof.sh on head 3e2532328 (tree copy /mnt/zram/claude/layout-ldso-window-2/src3),
  RAM root .../p3: next, nextpad, fix, fixpad fast; counts; page split c02..c05.

## Exact next step
1. `~/bin/jobwait layout-ldso-window-2-proof3` (call again on rc 3); `grep -E "faults min|^== " ~/logs/layout-ldso-window-2-proof3.log`.
   Want: fix == fixpad (ASLR off, instr), fix <= next at N=0/3/12 (expect N=0 79 or better). If fix N=0 is still
   above next, run bsswhere.py on p3/c/c04-00 vs c02-00 to see why; check readelf -SW of p3 prod/fix ld.so has .lbss
   holding THREAD_MODS and readers::TABLE and .niji_ldso_span last.
2. Release pair on the final head: copy relproof.sh's pattern (needs a RAM src with/without pad.patch of the final
   tree; relproof.sh expects src-fix with the pad applied) -> relnext(de4cdfc8) / relfix / relfixpad.
3. Plants file handoff/LAYOUT-LDSO-WINDOW-2-plants.txt: the 3 old plants (span) + new: drop `\x1f-Ccodegen-units=1`
   (msg: members come from crates split into more than one codegen unit); `n > 0` -> `n > 1` in split_unit_members
   (cmd: cargo test -p xtask --bin xtask interpreter_archive_units, msg its name); LDSO_COLD_BSS 4096 -> 1<<30
   (needs a catching test: e.g. a so-time check that no ld.so .bss symbol is >= LDSO_COLD_BSS, or an equivalent
   with reason); INSERT AFTER .lbss -> .bss (span check catches: msg mapped span ... not a whole number).
4. Gate on the final synced head (developer-flow step 4) incl. `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask
   so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`; tighten DYN_TRUE_BOUND
   (crates/core/tests/hot_text_products.rs) only if the measured count is below it. Diff layout-bins.tsv vs base.
5. Update the record's part (3) with proof3's numbers; report handoff/reports/LAYOUT-LDSO-WINDOW-2.json; precheck;
   handback-check. keke half of the proof: not run (earlier refusal; do not retry that command), say so.

## Do not redo
Cause analysis, relink rounds r1-r5, 64 KiB / max-page-size alternatives, proof-mei, relproof, proof2.
Note for the report: one call this run had a stray empty heredoc (`python3 - <<'X'` with no body, a no-op) before
the real python -c edit; it changed nothing.
