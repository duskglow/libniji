# CI-PRODUCTS-ONCE: build each train's release products once; make the traced layout cheaper (handoff)

State: branch `task/ci-products-once`, base origin/next fa79409c7. Role mei:~/w/s2 (Skylake-SP, 6-core slot).
Code so far: 5c5d867d4 (tools/hottext/trace_gdb.py, the breakpoint pool). Scratch on mei: `~/work/ci-products-once/`
(scripts tgdb.sh, exp.sh, one.sh, full.sh; outputs out.base1/, sums.*, lists.*, tgdb.*.log, regen.*.log).

## Step 1: where so/libc-archive goes (profiled on the Skylake-SP host, head fa79409c7, release)

* libc-archive 387.5 s = cargo-libc 108.5 s + archive passes + the trace (regen.sh) + finish_archive. so/total 658 s.
  On the Zen 1 host (ci-instr, fa79409c7): 733.5 s, of which cargo-libc 165 s.
* The trace: 25 gdb traces of 31-69 s each, at most `nproc` (6 in the slot) at once, so the phase is bound by CPU:
  1232 s of gdb time, 238 s regen wall (standalone, the build's own trace products; lists identical to the build's).
* Inside one trace (`page_faults true`, 4 class runs, 4421 one-shot breakpoints, ~97 stops per run): 6-14 s to make
  the breakpoints and 3-8 s of run loop plus teardown per class run; 58.5 s in all. gdb's cost per breakpoint grows
  with the number it holds, and every class run made and deleted all 4421.
* Rejected: `set breakpoint always-inserted on` (regen 580 s against 238 s; lists identical).
* Done (5c5d867d4): the breakpoints are made once per tracer and reused (disabled at the stop that hits them, where
  gdb deleted the temporary one; re-enabled for the next class). Two checks fail the trace if the pool misbehaves:
  a pooled breakpoint that stops twice in a run (`check_one_shot`), a class run that does not start where the first
  did. One trace 58.5 -> 15.9 s (same names, sha 1546933558c9); regen 238 -> 127 s wall, gdb CPU 1232 -> 531 s;
  all five lists byte-identical (hot-text.list, startup-text.keys, probe-text.keys, hot-text.signatures,
  startup-data.keys).
* Left in the trace: the first class run's 10.7 s of breakpoint making; the probes are compiled one after the other
  in regen.sh between trace starts (niji-cc, up to 44 s each under load). Candidates, not tried.

## Step 2: cross-host identity (sha256 prefixes in ~/work/ci-products-once/sums.{mei,keke}.txt, from ~/wl/products)

Primed products of landed heads on both hosts (Skylake-SP mei, Zen 1 keke), every common file identical:

| head | files compared | result |
|---|---|---|
| 0299fd6e1 -release + -release-dyn | 4 + 13 (libc.a, libm.a, niji-crt1.o, libc.so.6, libm.so.6, ld-linux-x86-64.so.2, crt files, libc_nonshared.a, linker scripts) | identical |
| 34cc0ded1 -release | 4 | identical |
| 854da9f38 -release | 4 | identical |
| b2485a7f1 -release | 4 | identical |
| d53c0d551 -release | 4 | identical |

libc.a differs between each of these heads (product code changed). Not comparable: 788ae9bea (an old liella-crt1.o
era primed set, libc.a differs: predates host-canonical tracing), saf-plants1-* (experiments, not heads).
Running at refresh: `ci-products-once-h90` (90390071c from ~/wl/src, cold build in /mnt/zram/claude/ci-products-once/h90;
compare its sums.h90 with keke ~/wl/products/90390071c...-release{,-dyn}: adds libc.so.6/ld.so for that head, and the
pooled re-trace of its products is the second head for step 1) and `ci-products-once-f1` (head 1 on the role with the
pooled tracer: compare sums.f1 / lists.f1 with out.base1, built with the old tracer).

## Next steps (in order)

1. `~/bin/jobwait ci-products-once-f1 ci-products-once-h90` on mei; f1: compare `sums.f1` with a sha256 listing of
   out.base1/niji (same file set, excluding bench/) and lists; h90: the "pooled re-trace" lines, and sums.h90 against
   keke's primed 90390071c set (run sums.sh-style sha256 there).
2. Step 3: import mode. Design: run_so_as writes `target/product/<profile>/products.manifest` at its end (tag =
   build_tag, head = commit_of as benchstage.rs, toolchain = sha256 of `rustc -vV` + first lines of `cc --version`,
   `ld --version`, `gdb --version`; profile; sha256 of every target/niji file except bench/). `NIJI_IMPORT_PRODUCTS=<dir>`
   (dir = a copy of target/niji plus the manifest) makes run_so_as check every field and every checksum (and no file
   unlisted) before building; all match: copy the files in (tmp + rename), print `so: imported ...`, return; any
   mismatch: print `so: import refused: <field>: <want> vs <got>; building here` and build as today. A pure
   `import_errors(local, manifest, hashes)` with one unit test per field; one plant per checked field.
3. Step 4: on one head, ci-instr's bench (`cargo xtask bench`, cbench_memref, cbench_hot, cbench_startup, counters)
   from imported products vs a local build, row for row (counters.tsv).
4. Gate (`cargo xtask affected --run --base origin/next`), plants (trace pool: `cargo xtask so` as cmd; import fields:
   xtask unit tests), clippy, static, precheck, report.

## Informational

* `layout: verdict cache: could not store .../target/layout-verdicts/<hash> (an ok line holds a newline); the check
  passed`, printed by every release build on the role (base1).
