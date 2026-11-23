# LD-TABLES-LAYOUT report

Branch task/ld-tables-layout, base next 9d12e7ee3. Commits: 527c0119b (the fix), c959c5b88 (run from the pages
that hold the address; the hole test; the index test's model), da1ffcc95 (D-27.89, plants, R-6 note), 1c650c001
(test names the trace check reads), then this report. All on the Skylake-SP build host's role.

## Cause (B1 explains the train 929/932 reds)
* Reproduced on the diagnostic merge (next d258cd270 + task/ld-relr-loop-2, scratch worktree, never pushed):
  `atexit_owner` net: `got 'dA aX', want 'dA mainH aX mainC'`.
* `/proc/PID/maps` of that failing run: libA.so mapped at 0x712ff617d000-0x712ff6182000, inside `ld.so`'s 60 KiB
  hole between its text (ends 0x712ff617d000) and its rodata (0x712ff618c000). `ld.so`'s PT_LOADs there: R 0x0,
  RX 0x1000+0x60227, R 0x71000, RW 0x7b020.
* The lookups matched every object by its whole PT_LOAD span. `ld.so`'s span held libA, the index set NESTED and
  the walk took the first object in chain order (`ld.so`): `_dl_find_object(return address in libA)` gave `ld.so`,
  so the opted-in atexit owner (D-27.29) of A's handlers was `ld.so` and they did not run at A's unload; `dladdr`
  likewise (dlopen_process_objects one short, conformance s27_dladdr abort). "The program missing from the tables"
  was a misreading: a library was attributed to `ld.so`.
* Why layout decides: the kernel maps the program it starts and `ld.so` and leaves the pages between their segment
  runs unmapped; whether a small library fits `ld.so`'s hole depends on `ld.so`'s text size (the gap between text
  end and the 64 KiB-aligned rodata) and on the order of mappings. Objects libniji maps reserve their gaps
  (PROT_NONE), so only those two can have something else inside their span.

## Reference (black box, probe R-6, tools/probe/dl/holes/probe.c)
Program linked `-z max-page-size=0x200000`, the free space above its highest hole filled so L lands in the hole:
`_dl_find_object` (link map, range, PT_GNU_EH_FRAME), `dladdr` and `dladdr1` (RTLD_DL_LINKMAP) name L for L's code
and the program for `main`, whose `_dl_find_object` range is the run holding it (its RX segment), `dli_fbase` its
lowest page; an anonymous page in the hole: -1 and 0; L's atexit handler runs at L's dlclose. Rerun this leg with
the dladdr1 and eh_frame checks added (job ld-tables-layout-t1): every check passes; also with `-z noseparate-code`.

## Fix (D-27.89, amends D-27.75)
`Extent` {Segment, Run, Whole} in `s27_dynlink::find_in`; `readers::extent(i)`: Run for the kernel-started program
and `ld.so`'s own entry, Whole otherwise. The index's segment entries carry the run as their range; `dladdr` checks
`extent_holds` after its range test; the walk computes the run in one `#[cold] #[inline(never)]` function. A run
starts from a segment whose pages hold the address, so the bytes between two segments of a run count on every path
(the index already did; leg 2 found the walk did not, and fixed it in c959c5b88). Nothing on the start path changed:
the index is built at the first reader. The libc-side backtrace (`backtrace.rs`, `unwind::in_object`) already
matched by segment. `__cxa_thread_atexit_impl` does not use the address lookups.

## Proof
* Before/after on the reproducing merge: before (job ld-tables-layout-repro1, last leg) atexit_owner fails as
  above; after, the same merge with the two fix commits applied (job ld-tables-layout-diagfix, diagnostic only):
  `ld.so` keeps its hole (RX 0x1000+0x60ae7, R 0x71000) and atexit_owner (2), dlopen_process_objects (7),
  dl_find_object_index (2) and address_holes (2) all pass.
* `crates/ld/tests/address_holes.rs`: the probe as the test program; L placed in the program's hole
  (kernel-started, 2 MiB holes) checks every lookup and the anonymous page; then as `ld.so PROGRAM`, the default
  link and `-z noseparate-code`. Fails without the fix whatever `ld.so`'s layout (plant extent-always-whole).
* `dl_find_object_index.rs`: its model now gives the program and `ld.so` their runs (`ld.so`'s text end is a probe
  point, so it checks the hole on every run).
* Plants (job ld-tables-layout-plants1): 4 of 4 CAUGHT, no WARN (handoff/LD-TABLES-LAYOUT-plants.txt).
* Direct gate (job ld-tables-layout-gate1): 15 suites, 860 passed, 0 failed, conformance 14 passed; peak 2.7G.
* Start-up checks: `cargo xtask test` (job ld-tables-layout-xtest): 7155 + 4878 + 129 + 46 passed, 0 failed;
  `NIJI_FULL_PROFILE=1 cargo xtask so && ... hot_text_products` (job ld-tables-layout-hottext): 33 passed.
* Fast layout bins against the merge base: bin 0 identical (104 entries, 60,744 B); bins 1, 3, 7 move by -29, +2,
  +3 B (rebuild noise in libc's time and stdio code, which this change does not touch); 70 diff lines, mostly
  `.llvm.<hash>` suffix renames.
* Start-up counts (STARTUP-REPROFILE kit `meas.sh`, release products, median of 101, ASLR off, Skylake-SP host;
  base = next 9d12e7ee3 built in a RAM copy, job ld-tables-layout-base; head job ld-tables-layout-startup):
  user instructions, faults and system calls identical on every row (min, median and max):

  | row | user instr base / head | faults base / head | syscalls |
  |---|---|---|---|
  | n0 empty main | 262,101 / 262,101 | 58 / 58 | 29 / 29 |
  | n3 three libraries | 368,742 / 368,742 | 70 / 70 | 59 / 59 |
  | t `/bin/true` | 322,486 / 322,486 | 58 / 58 | 29 / 29 |
  | n200 200 imports | 548,039 / 548,039 | 58 / 58 | 29 / 29 |
  | glibc g0 (same host) | 101,855 | 51 | 30 |

  Kernel instructions move both ways by up to 4% (n3 +3.6%, g3 on the reference -1.2% between the two runs):
  not a user-side cost. The Zen 1 class was not counted (this run's role is on the Skylake-SP host).

## INFORMATIONAL
* `ld.so` has a 60 KiB hole between its text (ends 0x61227 on next) and its rodata (0x71000). Closing it is a
  layout matter (LAYOUT-LDSO-*), no longer a correctness one.
* The Zen 1 class was not counted (this run's role is on the Skylake-SP host); no start-path code changed and the
  start-up bin is byte-identical, so no Zen 1 difference is expected.

## Leftovers
None running at the end of the run; the diagnostic worktree is removed.
