# LAYOUT-LDSO-WINDOW(-2): ld.so growth no longer moves its start-up pages

State: branch `task/layout-ldso-window-2` (continues `task/layout-ldso-window`, origin/next de4cdfc8d merged in),
role mei:~/w/s1. Briefs: ~/state/briefs/LAYOUT-LDSO-WINDOW.md (spec), ~/state/briefs/LAYOUT-LDSO-WINDOW-2.md.
Record: records/pending/LAYOUT-LDSO-WINDOW-1.toml (D-NEW, series D-X; parts (1) span, (2) one codegen unit,
(3) cold `.bss`, with the measured result). Report: handoff/LAYOUT-LDSO-WINDOW-2-report.md.
Plants: handoff/LAYOUT-LDSO-WINDOW-2-plants.txt (7). Scripts on mei: ~/work/LAYOUT-LDSO-WINDOW/{proof.sh,count.sh,
pagewhere.sh,pad.patch}, ~/work/LAYOUT-LDSO-WINDOW-2/{relproof.sh,bsswhere.py}.

## The change (code head 3e2532328; later commits are a test, the record and notes)
* 395a85303 span: `ld-span.ld` (`INSERT AFTER .lbss`) rounds ld.so's mapped span to `LDSO_SPAN_BUCKET` (0x80000);
  `check_interp_span` refuses an unrounded interpreter.
* 9ce45257f + b374a57f5: `LD_RUSTFLAGS` adds `-Ccodegen-units=1`; `check_ld_units` refuses an archive with
  `ld_so-*-cgu.<n>` members, n > 0 (unit test `interpreter_archive_units_beyond_the_first_are_reported`).
* 3e2532328: `LDSO_COLD_BSS` = 4096: ld.so's `.bss.*` of 4 KiB or more go to `.lbss` (copy of the archive).
* 904906254: `cold_bss_products::abi_ldso_cold_bss_objects_are_lbss` (no ld.so object >= 4 KiB in `.bss`, >= 2 in `.lbss`).

## Measured (mei, Skylake-SP, fast, faults N=0/3/12 ASLR off, 21 runs each, all equal)
* proof3 (head 3e2532328): next 79/95/131 instr 9038k; nextpad 83/98/134; fix 78/93/129 instr 9003818;
  fixpad 78/93/129 instr 9003818. N=0 by mapping: fix 16 anon + 3 ld.so rw-p (next 17 + 3, nextpad 19 + 4).
* layout-bins.tsv (fast) identical next vs fix; libc.so.6 bytes identical next vs fix.
* relproof3 (release): next 76/92/128; fix 74/90/126; fixpad 74/90/126 (instr equal within 3).
* gate2 (FULL by escalation) green, plants2 7/7 CAUGHT, fullprof2 green (all on 4aaed116f).

## Exact next step
Done; handed back COMPLETE. Follow-up: measure DYN_TRUE_BOUND on a Zen 1 host and tighten it (not done: one host class).

## Do not redo
Cause analysis, relink rounds r1-r5, 64 KiB / max-page-size alternatives, proof-mei, relproof, proof2, proof3.
keke half of the proof: not run (an earlier refusal; do not retry that command).
