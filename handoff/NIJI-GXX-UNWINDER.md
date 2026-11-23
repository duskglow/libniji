# NIJI-GXX-UNWINDER (handoff, also the full report)

State: branch `task/niji-gxx-unwinder` (from origin/next b04f71229), role mei:~/w/s2. Head: see
`handoff/reports/NIJI-GXX-UNWINDER.json` (or `git log -1`).

## Decisions in force
* D-22.24 (records/decisions/D-22/D-22.24.toml, amends D-22.23): niji-gcc / niji-g++ dynamic links add
  `niji-unwinder.o` (csys/start/unwinder.S, built by `cargo xtask so` as a START_FILES entry, shipped coexist
  niji6-dev under the sysroot) and `--export-dynamic-symbol=__niji_program_unwinder`. The entry returns the
  program's hidden `_Unwind_ForcedUnwind` (arg 0) / `_Unwind_GetCFA` (arg 1) through weak hidden GOT refs, 0 if
  not linked. `crate::cancel::no_unwinder` (cold) now: first step, D-22.23 walk finds work -> look up the
  entry (loader dlsym) -> unwind with it, else FATAL as before; later step -> look up only when the exit's
  class word shows an unwind was started. Plain-C exits with nothing to unwind take the same path as before.
* Why not the other two options: dynamic libgcc_s from niji-g++ needs the system dir on the run-time search
  (wrapper keeps it in the sysroot); dlopen of libgcc_s on first need = a second unwinder beside the
  program's static copy, and the reference's own `-static-libgcc -static-libstdc++` build aborts in that
  arrangement (table below).

## Reference probe (mei, Skylake-SP class, glibc 2.39, g++ 13.3; `forced_unwind.cc`, LD_DEBUG=files)
| build | NEEDED | libgcc_s.so.1 present | libgcc_s hidden (bind /dev/null) |
|---|---|---|---|
| g++ default | libstdc++, libgcc_s, libc | rc 0, 22 lines (dtors run, caught_fu=1, cancel result=-1); libgcc_s needed by the program | 127 (loader) |
| -static-libgcc | libstdc++, libc | rc 0, same 22 lines; libgcc_s needed by libstdc++.so.6 | 127 (loader) |
| -static-libgcc -static-libstdc++ (= niji-g++'s link) | libc, ld.so | 134, `Aborted`, no lines; libgcc_s dynamically loaded by libc.so.6 | 134, `libgcc_s.so.1 must be installed for pthread_exit to work` |

## Tests and plants
* `crates/core/tests/deb_manifest.rs::coexist_niji_gcc_threads_unwind_with_the_programs_own_unwinder`:
  coexist debs unpacked as sysroot; niji-g++ forced_unwind.cc, niji-gcc forced_unwind.c `-fexceptions` and
  the buffers build; each output == the same source built with g++/gcc defaults on the reference; no
  libgcc_s NEEDED; plain niji-gcc C links the entry but no unwinder.
* Plants: `handoff/NIJI-GXX-UNWINDER-plants.txt` (5).

## Jobs (mei)
* build1 (rc 0, head 4f73d7636's code; record/docs commits not yet synced then): cargo xtask clippy, so,
  `deb_manifest` 6 passed (the new test ran, not skipped), `forced_unwind_products` 6 passed.
* Running on the synced head (record + docs + handoff included): `niji-gxx-unwinder-gate2` (direct gate,
  `affected --run --base origin/next`), `niji-gxx-unwinder-plants2` (5 plants, dir plants-2).
* Superseded: niji-gxx-unwinder-gate, niji-gxx-unwinder-plants (killed seconds after start: role tree lacked
  the record/docs commits). probe-ref is a diagnostic probe (rc 1 = the script's last grep), described above.

## Open / not covered
* A niji-g++ program that also has libgcc_s.so.1 loaded (a dlopen'd plugin needing it): the loader lookup
  finds libgcc_s first and the exit unwinds with it beside the program's static copy. Not tested.
* #25's rows "before" for niji-g++ are RELEASE-SNAPSHOT-1122D's (134 + FATAL); the plant
  no-program-unwinder reproduces that state on this head.

## Exact next step (next leg)
1. `~/bin/jobwait niji-gxx-unwinder-gate2 niji-gxx-unwinder-plants2` on mei (one call, timeout 600 s); read
   `~/bin/test-summary ~/logs/niji-gxx-unwinder-gate2.log` and the plants log's CAUGHT/WARN lines; fix reds.
2. Plain-C cost: the path for a thread with nothing to unwind is unchanged by construction (no_unwinder returns
   None after `handler_on_stack()` false, as before; later steps add one load+compare). If a pthread_exit /
   create-join bench row exists (`grep -n "pthread_exit\|thread_create" benchmarks/*.tsv docs/benchmarks.md`),
   run `cargo xtask bench --compare --counters <row>` as a jobrun job and record before/after.
3. On mei: `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`, then
   `cargo xtask precheck --jobs niji-gxx-unwinder-gate2,niji-gxx-unwinder-plants2 --head <sha>`.
4. Write handoff/reports/NIJI-GXX-UNWINDER.json (with "jobs"), push, `ops handback-check`.
