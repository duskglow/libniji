# LAYOUT-CANONICAL-SET: handoff (run 1, INCOMPLETE: budget checkpoint, no code change)

State: branch `task/layout-canonical-set` on next e47a71997, handoff files only. Role shiki:~/w/sh1 (Haswell-EP).
Design: `plan/layout-budget-rethink.md` section 5 (on origin/task/layout-budget-rethink until it lands); owner
rulings R1-R4 yes (#5743-#5746). Run budget: 50 calls / 90k context. Reading the design, LT-4 and the tooling took
most of the context, so this run stops at a checkpoint after one measurement.

## What is already canonical (LT-4, keep, do not redo)

* Text: `tools/hottext/trace_gdb.py` answers `liella_machine_cpuid` / `xgetbv0` from a virtual CPU for each class and
  merges the runs, so `startup-text.keys` and `hot-text.list` already hold the union of the CPU classes' paths.
  `runner.sh` gives the program only its LANG/TZ (no inherited environment). `canon_host.sh` takes /etc files and the
  locale archive from the tree.
* Item 1 of the brief (fixed environment) is therefore done for text. Still open: argv0, cwd and path length are not
  fixed (mktemp directories of fixed length; check), and ADDR_NO_RANDOMIZE is the gdb default only.

## Where the host dependence remains (found this run)

`tools/hottext/startup_data.py` (run by `startup_data.sh`, which regen.sh calls last) builds `startup-data.keys` from
three sources: (1) and (2) `perf record -e page-faults -d` of `page_faults true` and of the isolated link, on the
**real CPU**, with the **inherited environment**, no setarch -R; (3) the RIP-relative operands of the start-up
functions (host-free, since the text keys are canonical). Sources 1 and 2 follow the host. They are the likely way
the 11-14 1,640-byte table joined only on the CI host (NOT VERIFIED: not reproduced on this host class).

## Measurement (job `layout-canonical-set-measure`, shiki, Haswell-EP, fast trace products of next e47a71997)

Script: `~/work/LAYOUT-CANONICAL-SET/measure.{sh,py}` on shiki (imports startup_data.py; source 1 only).

| set | objects | bytes |
|---|---|---|
| one-level RIP-operand rule over the 95 start-up functions (145 ranges) | 30 | 66,606 (65,536 of it `heap::RESERVE_MEM`, .bss) |
| perf faults, inherited env | 3 | 528 |
| perf faults, `env -i LANG=C PATH=... setarch -R` | 3 (same) | 528 |
| perf objects the operand rule misses | 2 | `startup_names::STARTUP_NAMES` 512, `process::AUXV_WORDS` 8 |

Readings: on this host the environment does not change the traced data; the operand rule (data closure, one level)
costs about 1 KiB besides the heap reserve, which is not near the 2-4x of the call closure; the rule misses two
objects reached through pointers, so a subset check with the rule as written would be red on every host. The rule
needs one more clause (data objects the start-up code reaches through pointers held in data or registers, e.g. data
referenced by relocations of an operand object: NOT MEASURED), or those two are declared by name.

## Exact next step

1. Run the same measure job on the Skylake-SP (mei) and Zen 1 (keke) roles, sources 1 and 2, and find the 1,640-byte
   object (`nm -S` size 0x668) in the host whose perf set has it; that names the rule that misses it (IFUNC candidate
   data, a CPU-classed path, or the environment).
2. In startup_data.py: declared set = operand rule (+ the clause above, + operands of every IFUNC candidate of a traced
   resolver, + both pages of each CPU-classed pair); run perf under `env -i` with fixed variables and `setarch -R`;
   check perf set ⊆ declared, red naming the object and the CPU class. Write only the declared set to startup-data.keys.
3. Plant: drop one rule clause, expect the subset check's message. Proof: identical lists and libc.a on the 3 classes.

Plan rows to read: `plan/layout-budget-rethink.md` 5, `plan/layout-rethink.md` 3.3, D-X.50, D-X.67, D-X.88.
