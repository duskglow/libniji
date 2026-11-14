# LAYOUT-CANONICAL-SET: handoff (run 2, LAYOUT-CANONICAL-SET-2)

State: branch `task/layout-canonical-set-2` (stacks on `task/layout-canonical-set`, next e47a71997). Role shiki:~/w/sh1
(Haswell-EP). Decisions in force: owner rulings R1-R4 of `plan/layout-budget-rethink.md` (#5743-#5746; this is R3),
D-X.50 amended by the pending record `records/pending/LAYOUT-CANONICAL-SET-2-1.toml` (D-NEW, series D-X).

## Done (do not redo)

* `tools/hottext/startup_data.py` writes `startup-data.keys` as a set DECLARED BY RULE from the linked probe alone:
  data addressed by (1) the start-up functions' operands, RIP-relative and absolute (`mov $<addr>`: the linker relaxes
  GOT loads in a static non-PIE link; the old `#`-only scan missed environ, ARGC, the csys/src/machine CPU words), (2) the
  operands of page_faults.c's own functions (its statics left out), (3) one level of pointers from those objects;
  `.lbss` left out (D-X.22 cold class; `global_locale::SLOTS` would count a page `true` never writes).
* perf traces (both probes) run under `env -i LANG=C PATH=/usr/bin:/bin setarch -R`; traced object outside the
  declared set -> build fails `STARTUP-DATA-DECLARED`, naming it and the CPU.
* startup_data.sh compiles page_faults.c to an object first and passes it (clause 2).

## Evidence

* Run-1 finding corrected: with source 2 (isolated link) the old operand rule missed ~38 traced objects, not 2;
  the cause was the absolute operand form. With the new rule nothing traced is outside the set.
* Same probe binaries on three host classes (jobs layout-canonical-set-2-xhost-{shiki,mei,keke}): declared list
  byte-identical, 99 objects, md5 735c49e2...; traced 57 (Haswell-EP), 58 (Skylake-SP), 58 (Zen 1), each a subset.
* Old keys (68) vs declared (99, fast, current link): +16 ro objects (11,139 B, C locale tables via C_SET),
  rw + stdout, `_IO_2_1_stdout_`, LOADER_RSEQ(_TP), COPY_MIN, EARLY_DONE, CPU words, C_SET; - RESERVE_MEM (.lbss).
* Release build on the new rule: all 12 contracts hold; STARTUP-DATA-WINDOWS 1 ro window, 8 of 8 written pages;
  hot_text_products 26/26 (job layout-canonical-set-2-startup-full). Fast profile: job layout-canonical-set-2-checks
  (clippy, static, cargo xtask test).
* 11-14 table: it is `anon.*` 1,640 B (tunables), reached only from `config::glibc_tunable_type` / `config::warnings`
  (not start-up functions in a fixed environment; GLIBC_TUNABLES in a host's environment would run them). Anon objects
  carry no key; the fixed trace environment and canonical text keys keep it out on every host.
* Plants (handoff/LAYOUT-CANONICAL-SET-2-plants.txt, job layout-canonical-set-2-plants): 2/2 CAUGHT by message.

## Open / next step

1. Brief item 4 not done: byte-identical `libc.a` and the four lists from one tree BUILT on each host class (needs a
   role on mei and keke; today only the probe-level identity above is proven).
2. The pointer clause has no plant (no traced object depends on it on these hosts: NOT VERIFIED that it is needed).
3. Release written pages sit at budget (8 of 8): base count on next not measured in this run.

Plan rows to read: `plan/layout-budget-rethink.md` 5, D-X.50, D-X.22, D-X.64, D-X.70.
