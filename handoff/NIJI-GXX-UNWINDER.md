# NIJI-GXX-UNWINDER (handoff, also the full report)

State: branch `task/niji-gxx-unwinder` (from origin/next b04f71229), role mei:~/w/s2. Code head 9675a622e;
the final head (report commits on top) is in `handoff/reports/NIJI-GXX-UNWINDER.json`.

## Decisions in force
* D-22.24 (records/decisions/D-22/D-22.24.toml, amends D-22.23): niji-gcc / niji-g++ dynamic links add
  `niji-unwinder.o` (csys/start/unwinder.S, built by `cargo xtask so` as a START_FILES entry, shipped in the
  coexist niji6-dev under the sysroot) and `--export-dynamic-symbol=__niji_program_unwinder`. The entry returns
  the program's hidden `_Unwind_ForcedUnwind` (arg 0) / `_Unwind_GetCFA` (arg 1) through weak hidden GOT refs,
  0 if not linked. In `crate::cancel`: `forced_unwind` and `no_unwinder` keep the base shape (loader check,
  D-22.23 walk, message); after the walk finds work, cold `with_program_unwinder` looks the entry up (loader
  dlsym) and starts the unwind with it, else the FATAL as before. A later step (after a cleanup buffer) goes to
  cold `later_step`, which looks up only when the exit's class word shows this exit started an unwind.
* Why not the other two options: dynamic libgcc_s from niji-g++ needs the system dir on the run-time search
  (the wrapper keeps the search in the sysroot); dlopen of libgcc_s on first need = a second unwinder beside the
  program's static copy, and the reference's own `-static-libgcc -static-libstdc++` build aborts in that
  arrangement (table below).

## Reference probe (mei, Skylake-SP class, glibc 2.39, g++ 13.3; `forced_unwind.cc`, LD_DEBUG=files)
| build | NEEDED | libgcc_s.so.1 present | libgcc_s hidden (bind /dev/null) |
|---|---|---|---|
| g++ default | libstdc++, libgcc_s, libc | rc 0, 22 lines (dtors run, caught_fu=1, cancel result=-1); libgcc_s needed by the program | 127 (loader) |
| -static-libgcc | libstdc++, libc | rc 0, same 22 lines; libgcc_s needed by libstdc++.so.6 | 127 (loader) |
| -static-libgcc -static-libstdc++ (= niji-g++'s link) | libc, ld.so | 134, `Aborted`, no lines; libgcc_s dynamically loaded by libc.so.6 | 134, `libgcc_s.so.1 must be installed for pthread_exit to work` |

## #25's three rows (destructors run, the forced-unwind catch is entered and rethrows, cancel joins with PTHREAD_CANCELED)
* Before (RELEASE-SNAPSHOT-1122D, base b04f71229): niji-g++ build 134 with the D-22.23 FATAL; g++ build (system
  toolchain, libgcc_s dynamic) rc 0, 22 lines. The plant no-program-unwinder reproduces the "before" on this head.
* After (this head, test `coexist_niji_gcc_threads_unwind_with_the_programs_own_unwinder`): niji-g++
  forced_unwind.cc, niji-gcc forced_unwind.c -fexceptions and the cleanup-buffer build all exit 0 with output
  byte-identical to the same source built with g++/gcc defaults on the reference (22 lines for the C++ row); no
  libgcc_s in DT_NEEDED.

## Plain-C counts (wl_percall, dynamic kit, instructions:u per call, base b04f71229; spec in ~/work/niji-gxx-unwinder/thread_exit.c on mei)
| case | glibc | base | first try (dbf7e68e3) | 10f220ea1 | 1f8df4b8b | final 9675a622e |
|---|---|---|---|---|---|---|
| thread_exit_join | 9529.3 | 89942.6 | 97986.6 | 91328.6 | 90066.6 | **89942.6** |
| thread_cleanup_exit_join | 15180.3 | 93411.6 | 101482.6 | 94824.6 | 93541.6 | **93417.6** (+6.0) |
| thread_create_join | 2266.0 | 36092.1 | 36092.1 | 36092.1 | 36092.1 | **36092.1** |

Cause of the first try's +8044: the D-22.23 walk pays thousands of instructions per frame, and its cost follows
the shape of the frames it passes (a bigger `no_unwinder` with more saved registers; the symbol sizes of all 101
unwind/backtrace symbols were unchanged). The final shape keeps both walked frames as in the base. The remaining
+6.0 on the cleanup-buffer exit is `later_step`'s class-word check (a call, a load, a compare), which C++ needs
after a cleanup buffer: a measured cost, recorded in the report.

## Tests and plants
* `crates/core/tests/deb_manifest.rs::coexist_niji_gcc_threads_unwind_with_the_programs_own_unwinder`: coexist
  debs unpacked as sysroot; the three builds above vs the reference; plain niji-gcc C links the entry but no
  unwinder.
* Plants: `handoff/NIJI-GXX-UNWINDER-plants.txt` (5). The first plan's `.S` plant (entry-swapped) was refused by
  `xtask plants` (no `niji_machine_` label in csys/start/unwinder.S); replaced by entry-args-swapped (the same
  contract broken from the library side).

## Jobs (mei)
* Final head 9675a622e: niji-gxx-unwinder-lint3 (clippy -D warnings + static, rc 0), -percall4 (table above),
  -plants6 (5 of 5 CAUGHT, no WARN), -startup3 (rc 0: `cargo xtask test`, full-profile so, hot_text_products
  33 passed), -gate5 (direct gate, rc 1: every suite green but hot_text_products, whose 10 failures all read
  `target/niji/.hot-lists: No such file`: kill-mine had ended gate4's `cargo xtask so` midway), -hotfix (that
  suite after `cargo xtask so`: 33 passed, 0 failed).
* niji-gxx-unwinder-gate6: the direct gate rerun on the final head, rc 0: 15 of 15 suites, 5859 passed, 0 failed,
  216 ignored, conformance 39 passed, peak 4.7G (gate5 is SUPERSEDED).
* Superseded: gate2 (rc 0, 5859 passed) and plants2 (refused .S plant) on 09f147ceb; plants3, startup, gate3,
  plants4, gate4, plants5, startup2 killed when a count fix changed the code; percall/percall2/percall3 are the
  earlier columns above; lint/lint2 earlier heads (lint2 ran on 10f220ea1: the sync was BUSY).

## Open / not covered
* A niji-g++ program that also has libgcc_s.so.1 loaded (a dlopen'd plugin needing it): the loader lookup finds
  libgcc_s first and the exit unwinds with it beside the program's static copy. Not tested.

## Exact next step
Done in run 1 (landing is the coordinator's). For the record: `cargo xtask precheck --jobs <those + percall4> --head <sha>` on the role,
report json, `ops handback-check`.
