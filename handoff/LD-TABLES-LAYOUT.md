# LD-TABLES-LAYOUT (branch task/ld-tables-layout)

State: fix committed and pushed (527c0119b, on next 9d12e7ee3). Test, plants, gate, counts, decision row
and report still to do (next leg).

## Cause (found, with evidence)
* Reproduced on the diagnostic merge (next d258cd270 + origin/task/ld-relr-loop-2, a detached worktree in
  the run's scratch, never pushed): job `ld-tables-layout-repro1` (mei s2): atexit_owner net FAILS
  `got 'dA aX', want 'dA mainH aX mainC'`.
* /proc/PID/maps of that failing run (strace-delayed openat, scratch script): libA.so mapped at
  0x712ff617d000-0x712ff6182000, INSIDE ld.so's 60 KiB hole between its text (ends 0x712ff617d000) and its
  rodata (0x712ff618c000). ld.so's PT_LOADs: R 0x0, RX 0x1000+0x60227, R 0x71000, RW 0x7b020.
* The lookups matched objects by the whole PT_LOAD span: ld.so's span holds libA, the index sets NESTED and
  the walk returns the first object in chain order (ld.so). So `_dl_find_object(return address in libA)` =
  ld.so: the opted-in atexit owner is ld.so, so A's entries (mainH, mainC) do not run at A's unload. The
  same for dladdr (dlopen_process_objects one short, s27_dladdr abort). This is the coordinator's B1
  hypothesis: it explains the train 929/932 reds. "Main program missing" was a misreading: a library was
  attributed to ld.so. Layout-sensitive because whether a small library fits the hole depends on ld.so's
  text size (the hole between text end and rodata) and the allocation order.
* The kernel maps the program it starts and ld.so (interp) and leaves the gaps between segment runs
  UNMAPPED; `map::load` reserves the gaps of every object libniji maps (PROT_NONE), so only those two can
  have something else inside their span.

## Reference (black box, probe tools/probe/dl/holes/probe.c, R-6 in spec/behaviour/dynlink.md)
Program linked `-z max-page-size=0x200000`, gaps above its highest hole filled, L dlopen'd lands in the hole:
reference names L for L's code, the program for main; `_dl_find_object(main)` range = the run holding main
(its RX segment, not the whole span); an anonymous page in the hole: -1 and dladdr 0; atexit handler
registered by L runs at L's dlclose.

## Fix (527c0119b)
* `s27_dynlink::Extent` {Segment, Run(page), Whole}; `find_in(.., extent)`; `run()` grows the segment
  holding pc by every segment whose pages touch (cold).
* `readers::extent(i)`: Run for the kernel-started program (i == 0 and START_RUN_PATH == 0) and ld.so's
  own entry (START_INTERP if < START_N, else START_N); Whole otherwise. `readers::extent_holds(i, a)`.
* index.rs `fill`: each segment entry carries its object's extent (run for those two) as o_lo/o_hi.
* dladdr.rs `holds`: whole range AND `extent_holds`.
* After the fix the probe on libniji (job ld-tables-layout-fix2 + a rerun) gives exactly the reference's
  answers, kernel-started and under `ld.so PROGRAM` (there libniji maps the program with a reservation).

## Next step (exact)
1. Test `crates/ld/tests/address_holes.rs`: build tools/probe/dl/holes/probe.c (include_str!) the way
   atexit_owner.rs builds its program (niji-crt1.o, libc.so.6, ld.so, `-Wl,--dynamic-linker`, plus
   `-Wl,-rpath,<target/niji>`; run with env LD_LIBRARY_PATH=<target/niji> on the Command only), L from
   `void l_func(void) {} int l_register(void (*f)(void)) { return atexit(f); }` linked `-nostdlib -fPIC
   -shared <target/niji>/niji-owner/libc-owner.so`. Variants: (a) `-Wl,-z,max-page-size=0x200000`,
   kernel-started, mode `hole`; (b) the same under `ld.so PROGRAM`, mode `any`; (c) default link, `any`;
   (d) `-Wl,-z,noseparate-code`, `any`. Each asserts exit 0 and "PASSED", and for (a) also the anon line
   `_dl_find_object rc -1` / `dladdr rc 0`. Clause tag: as dl_find_object_index.rs's tests.
   It fails on the unfixed code in variant (a) (no dependence on ld.so's layout).
2. Plants file handoff/LD-TABLES-LAYOUT-plants.txt: extent() always Whole; dladdr holds without
   extent_holds; index fill o_lo/o_hi = (lo, hi). msg: the new test's name.
3. Decision row: `ops reserve --story LD-TABLES-LAYOUT D-27`, record under records/decisions/D-27/.
4. clippy + static on s2, affected gate, plants, `cargo xtask test` + `NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products` (start-up rule), STARTUP-REPROFILE empty
   main row both classes (unchanged expected: nothing on the start path changed; the index is built at the
   first reader).
5. INFORMATIONAL for the report: ld.so has a 60 KiB text-to-rodata hole (rodata at 0x71000 after text ending
   0x61227); closing it is a layout matter (LAYOUT-LDSO-*), no longer a correctness one.
6. Remove the scratch diagnostic worktree (`git worktree remove` of the run's scratch diag dir).

## Jobs so far (mei s2)
ld-tables-layout-repro1 (diagnostic build, reproduces), -fix1 (compile error, superseded), -fix2 (fixed build +
probe; on the pre-warning-fix head, superseded by the final gate).
