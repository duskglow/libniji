# ASAN-MAIN-THREAD: ASan registers the main thread under libniji (rest of GitHub issue #26)

**State:** branch `task/asan-main-thread`, code commit c723d5d78 (final head: the hand-back). Decision in force:
**D-22.20** (reserved for this story). Related: D-22.10 (threads libniji did not create), D-X.60 (the thread registry),
D-27.24 (start-up under libniji's `ld.so`), D-27.65 (`_dl_get_tls_static_info`, the first half of #26).

## The call that differed (brief step 1)
ASan's start-up log (`ASAN_OPTIONS=verbosity=2`) was the same under both C libraries, `SetCurrentThread` and
`T0: stack [...)` included. So the runtime did register its main thread; the record was gone by the report.
libasan's undefined symbols include `pthread_key_create`, `pthread_setspecific` and `pthread_getspecific`. Black-box
probe P-AMT-1: a key value set from `.preinit_array` reads back in `main` as 0x1234 under the reference and as NULL
under libniji. A value set from a program constructor survives in both.

**Cause in libniji:** libniji's `ld.so` runs `.preinit_array` and every shared object's initialisation before
`__libc_start_main`. Until `thread::init_initial` the process is not yet "owned", so the initial thread's words live in
its registry entry (`foreign_words`). `init_initial(false)` then mapped a new descriptor and switched to the thread's
own block, leaving the entry's words behind. Probe P-AMT-2, same program on both libraries: `errno` and a `uselocale`
choice set in `.preinit_array`, and a C++ `thread_local` destructor that a library constructor registers. The
reference keeps all of them (errno 77, locale kept, destructor runs at exit); libniji before the fix lost them all.

## The fix (step 2)
`crates/boundary/src/thread.rs`: `init_initial` (non-static) calls the cold `carry_initial_words`. It finds the
thread's registry entry by thread pointer, without claiming or mapping a chunk, and moves every non-zero word into the
block. The words: `errno`, `h_errno`, `tls_dtors`, `heap_cache`, `node`, the descriptor, `locale`, the ctype words,
`lconv`, `res`, `view`, `cache`, `wclass`. The loader's `flags` and the block-only `dlerror` are not moved.
`init_initial` then reuses that descriptor, or `adopt_into(block)` makes one there (`adopt()` = `adopt_into(words())`).

## Sanitizers after the fix (step 3; gcc 13 on the Skylake-SP build host; reference vs libniji)
* Programs linked with libniji's `ld.so` as their interpreter (`-Wl,--dynamic-linker`):
  * ASan: rc 1/1. `thread T0`, the allocation stack and SUMMARY match the reference. The only difference is the start
    frames: libniji's `start_under_niji_ld` / `start` / `__libc_start_main` stand where the reference has its own two.
  * LSan: rc 23/23, the same report apart from the frames.
  * TSan (`setarch -R`): rc 66/66, the same race. Which access prints first follows the threads' scheduling.
  * UBSan: identical.
* Run as `ld.so <prog>`, the sanitizers symbolise against `/proc/self/exe` = ld.so, so the SUMMARY lines name
  ld.so's frames. That comes from the run mode (as LDSO-TLS-STATIC-INFO noted), not from a libniji defect.
* No sanitizer still fails: nothing is left for a follow-up story.

## Tests and plants (step 4)
* `crates/ld/tests/tls_static_info.rs::initial_thread_keeps_words_set_before_main` (new, C only, needs no libasan).
  It checks `errno` (the library constructor and `main` both see it), the key value, the locale and the `thread_local`
  destructor.
* `tls_static_info_sanitizer_runtimes_start`: ASan now requires `thread T0`, `allocated by thread T0 here:`, SUMMARY,
  exit code 1 and no `CHECK failed`. A use-after-free case adds `freed by` and `previously allocated by`. The test is
  skipped with a reason when the toolchain has no runtime.
* Plants: `handoff/ASAN-MAIN-THREAD-plants.txt` (5): no carry at all, descriptor, errno, destructor list, locale.

## Not to redo / open
* Probe scripts are in the run's scratch (`~/work/ASAN-MAIN-THREAD/` on the build host: run1-run6.sh).
* `clear_words` (the registry's new-thread reset) does not reset `wclass`. Noticed in passing and not changed here
  (INFORMATIONAL).
* Start-up instruction counts were not measured. When nothing used the registry before `main`, the fix maps no
  registry chunk. Before, `adopt()` went through `foreign_words`, which needs one (from reading the code; not
  measured).
* Jobs (mei): asan-main-thread-gate-1 rc 0 (direct tier: clippy `--all-features`, xtask clippy, static, 197
  targets, 32 conformance programs; ran on c723d5d78, which equals the final head outside handoff/);
  asan-main-thread-startup-1 (`cargo xtask test`, `NIJI_FULL_PROFILE=1` so + hot_text_products);
  asan-main-thread-plants-2. Superseded: asan-main-thread-plants-1 (refused a plant pattern that matched two sites).
  asan-main-thread-startup-1 rc 0 (`cargo xtask test`: 3235 + 1203 passed; hot_text_products 30 passed).
  asan-main-thread-plants-2 rc 0: 5 CAUGHT, no WARN lines.
* Layout bins (§9), `target/product/fast/layout-bins.tsv` against the base 50abc44ed (built by
  asan-main-thread-base-layout-1). Start-up bin 0: the dynamic start `runtime::start<false>` (which inlines
  `init_initial`) went from 5902 to 5822 B (-80). No other start-up function grew. Bin 0's total moved 60646 -> 60694
  because a cold filler was swapped (`_IO_popen` 192 B for a 64 B one). Elsewhere: a few bytes of jitter in untouched
  functions (setlocale_env +16, atexit -5, block_addr +9, scratch_claim -11).
  That base build wrote its `target/product/fast/` into the primed base source tree on the host
  (`~/wl/src/<sha>/target`), although CARGO_TARGET_DIR pointed elsewhere (INFORMATIONAL).
* Next step: the coordinator can tell Yohane that #26 is complete (ASan's full report).
