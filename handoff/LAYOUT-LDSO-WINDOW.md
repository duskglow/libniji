# LAYOUT-LDSO-WINDOW: ld.so growth no longer moves its start-up pages

State: branch `task/layout-ldso-window`, base origin/next 18cf8293f, code head 395a85303. Role mei:~/w/s1.
Brief: ~/state/briefs/LAYOUT-LDSO-WINDOW.md. Decision: records/pending/LAYOUT-LDSO-WINDOW-1.toml (D-NEW, series D-X).
Plants file: handoff/LAYOUT-LDSO-WINDOW-plants.txt (3 plants, all on the build's span check).

## Cause (measured on mei, Skylake-SP, Linux 6.8, ASLR off; scripts in ~/work/LAYOUT-LDSO-WINDOW on mei)
* The kernel maps the interpreter top-down with its END at the mmap base (0x7ffff7fff000 ASLR off), so ld.so growth
  moves its START: the start-up text group (start of .text) changes 64 KiB window phase (+1 text fault), and every
  mapping below ld.so moves (vDSO, libc, the thread block).
* The anonymous page: libc's thread registry `foreign_words`/`shadow_entry` (crates/boundary/src/thread.rs) hashes
  the thread pointer into a 64-slot table (an 896-byte slot); at some thread-pointer addresses the slot crosses a
  page (+1 anon). The thread block is mapped below ld.so, so it moves with ld.so's size.
* ld.so's own RW pages do not move with text size (GNU ld ends RELRO on a page, .data starts there; text growth
  shifts the RW segment by whole pages). The predecessor's "+1 RW" was not reproduced with the simulated pad.

## Change (395a85303)
xtask/src/so.rs: `LDSO_SPAN_BUCKET` = 0x80000; the interpreter link gets `-T target/product/ld-span.ld`
(`.niji_ldso_span (NOLOAD) : { . = ALIGN(0x80000); }` `INSERT AFTER .bss`); `check_interp_span` refuses a linked
interpreter whose span (lowest PT_LOAD page to the page end of the highest) is not a whole number of buckets.

## Measured so far (relinks of the role's grouped inputs; ~/work/LAYOUT-LDSO-WINDOW/exp.sh, rounds r3, r5)
| variant (pad = simulated cold text) | N=0/3/12 faults ASLR off | instr N=0 |
|---|---|---|
| origin/next | 79/95/131 | 9038390 |
| origin/next + 46 KiB pad | 81/95/131 | 9038392 |
| 64 KiB rounding, pad 0 / 46 KiB | 80/94/130 / 79/94/130 | equal |
| 512 KiB bucket, pad 0 / 20000 / 46 KiB / 70000 / 120000 | 79/94/130 for all | 9038390 (+-2) |
| `-z max-page-size=0x10000` | 79/95/131 (kernel does not align the interp base) | |
count.sh artefact: whatever binary sits in the FIRST slot reads ~265 fewer instructions (slot order swapped
proved it; proof.sh repeats "next" in slot 1). Not a property of any build.

## Jobs running at refresh (mei), started on head 395a85303
* `layout-ldso-window-proof-mei`: proof.sh, RAM copies under /mnt/zram/claude/layout-ldso-window/p1: origin/next
  and the branch, each with and without pad.patch (the real 46 KiB cold fn); count.sh 21 runs N=0/3/12, ASLR off+on
  (slots: next(dummy) next nextpad fix fixpad = s01..s05); per-page split for c02..c05; build logs p1/so-*.log.
* `layout-ldso-window-gate1`: clippy -D warnings, static, records check, affected --run --base origin/next,
  xtask test, release so + hot_text_products --nocapture, fast so + hot_text_products --nocapture.
* `layout-ldso-window-plants1`: cargo xtask plants handoff/LAYOUT-LDSO-WINDOW-plants.txt, --dir
  /mnt/zram/claude/layout-ldso-window/plants-1 --timeout 3600.

## Exact next step
1. `~/bin/jobwait layout-ldso-window-proof-mei layout-ldso-window-gate1 layout-ldso-window-plants1` on mei (call
   again on rc 3). Read summaries: `grep -E "faults min|^== |proof.sh|: LOAD" ~/logs/layout-ldso-window-proof-mei.log`,
   `~/bin/test-summary ~/logs/layout-ldso-window-gate1.log`, the plants summary line.
2. Proof: fix vs fixpad must be equal at N=0/3/12 ASLR off (and ASLR-on medians); fix vs next no worse, faults and
   instructions. Expected (from the relinks): N=3/12 improve by 1.
3. hot_text_products: the dynamic probe prints its count; if it is below DYN_TRUE_BOUND [89, 86]
   (crates/core/tests/hot_text_products.rs) tighten that bound to the measured level (bounds only tighten).
4. keke: NOT DONE. The step was refused (see REFUSED below); the proof stands on mei only unless the coordinator
   arranges a keke run. Do not retry that command.
5. Write the report (this file, "Report" section), handoff/reports/LAYOUT-LDSO-WINDOW.json with "jobs", push,
   `python3 ~/src/liella-ops/bin/ops handback-check LAYOUT-LDSO-WINDOW --head <sha> --jobs "<list>"`.

## REFUSED (auto-mode classifier, call 71)
`cd ~/w/LAYOUT-LDSO-WINDOW && . ~/src/liella-ops/tools/vm_ssh.sh; vm_ssh_cmd keke; git archive HEAD | "${VSSH[@]}" 'D=/dev/shm/claude/layout-ldso-window/srcfix; rm -rf $D; mkdir -p $D && tar -x -C $D && grep -c LDSO_SPAN_BUCKET $D/xtask/src/so.rs'; "${VSSH[@]}" 'setsid nohup ~/bin/jobrun layout-ldso-window-proof-keke count -- sh -c "bash ~/work/LAYOUT-LDSO-WINDOW/proof.sh /dev/shm/claude/layout-ldso-window/p1 ~/wl/src/18cf8293f411497163625bfa1afed94d9f67bb33 /dev/shm/claude/layout-ldso-window/srcfix" >/dev/null 2>&1 </dev/null &' </dev/null; vm_ssh_cmd mei; (then the same proof job on mei)`
(shipping the branch's source to a RAM dir on keke for the keke half of the proof). The mei half was started alone.

## Do not redo
The cause analysis and the relink rounds r1-r5 (results above); the 64 KiB and max-page-size alternatives.
