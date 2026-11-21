# LOADER-FOLDED-VERIFY: handoff

State: branch `task/loader-folded-verify`, cut from `origin/next` (b732d9b46). Commits: the test
(`conformance/c/s27_dlerror_threads.c`, the `s27_1_c_5` stub replaced by a real test), then the fix
(`crates/boundary/src/dlerr.rs` `thread_ending`/`release`, one call in `crates/boundary/src/thread.rs`
`finish`), decision record D-27.67, plants file `handoff/LOADER-FOLDED-VERIFY-plants.txt`, this note and
`handoff/reports/LOADER-FOLDED-VERIFY.json`.

## Result per folded story

* **DLERROR-THREAD-STATE (s27_1_c_5): behaviour was right, one defect found and fixed.** The old branch held
  only a test design (no code). Ported as `s27_dlerror_threads` (dynamic-link conformance, runs on glibc 2.39
  and on libniji's ld.so): the text once then NULL; a successful `dlsym` clears it; 1000 lock-step rounds of
  two threads (A fails a `dlsym`, B reads NULL and fails a `dlopen`, each then reads only its own text); a
  thread created while A's text is pending reads NULL; the main thread's text survives both. All of that
  passed on libniji before any change. The defect: the per-thread record (8 KiB private mapping) was never
  released at thread exit (plan/ph14/dl-16-dlopen.md §4 left it to a "DL-15 thread-exit hook" nobody
  wrote). Measured on the base: 2000 threads that fail and exit grow VmSize 49056 kB -> 65056 kB (exactly
  8 KiB each); the reference grows < 2 MiB (passes). A reused thread mapping cleared the word, so no text
  ever leaked to another thread: only memory. Fix: `finish` calls `dlerr::thread_ending()` after every
  destructor; one load when the thread never failed, cold `release` otherwise (clear the word, munmap).
* **TLS-GET-ADDR-LATE-MODULE (s27_3_c_0): already covered, nothing to port.** DL-16d (D-27.39) landed
  `conformance/c/s27_dlopen_tls_surplus.c`: GD/LD/IE/TLSDESC in `dlopen`ed objects from a thread created
  before the `dlopen`, one after, and 32 created during it; initial image, zeros past it, alignment, a block
  of each thread's own, a relocation against the program's TLS; plants in
  `handoff/DL-DLOPEN-TLS-SURPLUS-plants.txt` include `walk-skips-live-thread` (an existing thread missed)
  and `walk-no-publish` (new threads). It passed on both libraries here (run conf1). The old story's probe
  adds nothing this program does not check (its "keep early threads parked" lesson is moot: T1 is alive
  across the comparison). Design note: libniji places these blocks at `dlopen` in the static surplus, not
  lazily on first access; the clause's "allocated on first access" is the reference's mechanism, not an
  observable contract, and D-27.39 records the choice.

## Decisions in force

D-27.67 (new, this story), D-27.2, D-27.37 item 4, D-27.39.

## Jobs (keke, role n1)

`loader-folded-verify-conf1` (base + test only: libniji FAIL on the VmSize check only, reference pass; TLS
program pass on both: the evidence for the defect), then on the fix head: `-lint`, `-plants`, `-gate`,
`-xtest`, `-hottext`. Results: see the report json and the hand-back.

## Not to redo

The glibc-side behaviour (P-6) is probed and recorded in spec/behaviour/dynlink.md; the TLS half needs no
new test.

## Next step

Land. The coordinator closes DLERROR-THREAD-STATE and TLS-GET-ADDR-LATE-MODULE with this result.
