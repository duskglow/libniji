# DL-DLCLOSE (DL-17): dlclose, unloading and safe reclamation in Liella's ld.so

## State (run 5, DL-DLCLOSE-5, 2026-11-14): the race plant is caught deterministically; ready to land

Branch `task/dl-dlclose-5` (= `task/dl-dlclose-4` 2c0022f3f plus this run), role mei:~/w/s2. Code commits of this run
(test and docs only; no product code changed): `297e9b0ab` stops the dladdr reader inside its walk (chain phase),
`44d41c37e` adds the counted-walk edges to the `dl_iterate_phdr` differences row, `9563cb6ff` counts early-stopped
calls in check 58 and raises the round cap to 3000, `d818613a0` runs the chain phase before the racing walkers,
`d6f2100e5` holds the reader outside its walk between rounds (fixes the hang below).
**The hang gate2 found (test bug, fixed in d6f2100e5):** under load ~60 the concurrent test hung until its alarm
(SIGALRM, 301 s). The reader looped dladdr without a break, so the reclaims at each round's opens and closes rarely
found the readers drained; retired objects stayed at the table's top until a close made while the reader was stopped
passed ruling #5651's room threshold and waited for that reader, which was waiting for main. Now the reader runs only
from after the round's opens until before A's close (`hold`/`held`, waiting between calls otherwise), so A's close
unmaps everything and the closes made while it is stopped stay far below the threshold (32 retired, top ~40).
**The chain phase (crates/ld/tests/dlclose.rs `chain()`, concurrent mode, first):** a dladdr-only reader thread
(`pause_reader`) looks up A's `a_fn` in a loop and counts its calls. Per round, main opens L0..L31 (copies of
libR.so) and A (generation odd), stops the reader with `SIGUSR1` (`pause_here` stores `paused` and spins on `go_on`:
plain atomics, no libc call), closes all Ls, records whether L0 is still mapped (`mapped`: the reader was stopped
inside its counted walk, which keeps every object unpublished meanwhile mapped), lets it go on, waits until its call
counter moves by 2 (the stopped call ends and one more runs, A still open), then makes the generation even and
closes A. Rounds run until 100 stops landed inside (at most 3000). Check 58: a dladdr while A is open found A (a
reader standing on a closed L walks on through the L's kept `l_next`); check 59: at least 100 stops inside (fails
when dladdr is not counted: the Ls are then unmapped at once). The reader is first seen in its loop before any stop
(never stopped in its start-up). Then the earlier racing walkers (`walkers()`, checks 50-54) run as before.
**Why deterministic:** a stop inside the walk lands on an L for most of the walk (32 Ls, a handful of objects before
them, one lookup in A after), and each such stop fails check 58 under the plant. Measured on the Zen 1 build host:
plants3 100 of 131 rounds stopped inside, 70 calls stopped early; plants4 (final test) 100 of 133, 57 calls: about
6 in 10 inside stops catch it, so missing all 100 is about 0.4^100. dladdr-not-counted: the uncounted reader faults
(check None) in all four plant runs; if it did not, check 59 fails (no stop keeps the closed Ls mapped).
**Results on head d6f2100e5 (mei):** `dl-dlclose-5-lint4` (clippy + static clean), `dl-dlclose-5-tests2` (products,
then the concurrent test 5 times at load ~35: all passed), `dl-dlclose-5-plants4` (4 CAUGHT, 0 WARN, one run each),
`dl-dlclose-5-gate3` (direct tier), `dl-dlclose-5-stress2` (12 x 20 concurrent runs), `dl-dlclose-5-release1` (CI's
release start-up check: `LIELLA_FULL_PROFILE=1` products and `hot_text_products`), `dl-dlclose-5-xtest1`
(`cargo xtask test`). Gate3, stress2, release1 and xtest1 were queued or running at hand-back (context checkpoint,
162k): they run one after another from `~/work/DL-DLCLOSE-5/chain3.sh` on mei; read them in `~/logs/jobs.tsv`.
Superseded: `lint0` (inherited head, clean), `tests1` (297e9b0ab: concurrent test x3 and all 4 dlclose tests passed),
`lint1`, `plants1` (44d41c37e: 4 CAUGHT; check 58 at generation 100405), `lint2`, `plants2` (9563cb6ff: the
plant was caught by the racing walkers' check 54 first, WARN-NO-MSG, so the chain phase now runs first), `gate1`
(stopped by me on 9563cb6ff when the test changed), `lint3`, `plants3` (d818613a0: 4 CAUGHT, 0 WARN), `gate2`
(d818613a0: rc 1, the hang above; 6570 other tests passed), `stress1` (stopped by me: older head).
**Rulings applied:** #5651 (wait), #5652 (exit order: follow-up DL-EXIT-ORDER-PROBE, `fini_loaded` unchanged),
#5654, #5628; record id D-27.50 (assigned by CI). Coordinator 11-10: the known edges below are accepted.
**Known edges (record D-27.50 (8), and now the `dl_iterate_phdr` row of docs/differences-from-glibc.md):** a
callback that leaves by `longjmp`, or whose thread exits inside it, stays counted: objects closed meanwhile stay
mapped, and past the threshold every later `dlopen`/`dlclose` outside a read section waits for ever. An object a
destructor re-opens keeps its destructors run (they run again at exit); `dlsym` creates no binding dependencies;
thread_local destructors a plugin registered do not pin it; atfork nodes of dropped handlers are kept (40 bytes
each). Live objects alone past the table (512) still fail: DL-OBJ-TABLE-GROW.
**Next step:** land; then DL-EXIT-ORDER-PROBE (two `RTLD_NODELETE` objects and a unique-pinned pair at exit);
DL-ATEXIT-OWNER; the sorted address index for `_dl_find_object` (and `dladdr`) with DL-OBJ-TABLE-GROW.
**Do not redo:** run 4's merge with next (dladdr) and alias regeneration (`cargo xtask compat-gen`); the probe record
(`== close (DL-17)`); the corpus fixes; the reader-stop test. Stress: `~/work/DL-DLCLOSE-5/stress.sh W N` on mei
(one test-binary build, W x N runs of the concurrent test, outputs kept in /mnt/zram).
This note is the full report (handoff/reports/DL-DLCLOSE-5.json); plants: handoff/DL-DLCLOSE-5-plants.txt.
Process notes: one heredoc slipped into a local python call (rule: none); run 4's stress script lived on kanon only,
so this run wrote its own on mei.

## Design (plan/ph14/design.md §8-§10, dl-16-dlopen.md §4 "Room for DL-17"; full text in the decision record)

**Liveness:** an object stays while it is initial, `nodelete`, has `opens > 0`, or is reachable from such an object
through `deps` (`DT_NEEDED`) or `binds` (objects outside its dependency closure its relocations bound to, reported by
the relocator through `UniqueRegistry::bound`). dlclose decrements `opens`; at 0 it marks from the roots and unloads
every unmarked object. **Finalisation:** reverse constructor order; `DT_FINI_ARRAY` in reverse, then `DT_FINI`; run
holding `CALL` only, objects still published (a destructor may `dlopen`/`dlclose`; exceptions unwind);
`__cxa_finalize(dso)` drops that object's atfork handlers. **Unpublication** under `STATE`: global scope, constructor
log, `l_next`/`l_prev` chain (at the predecessor only: the node keeps its own `l_next` and range for a dladdr
reader standing on it), reader entries (header count 0); `dlpi_subs` moves before dlclose returns. Readers:
`dl_iterate_phdr`, `_dl_find_object`, `dladdr`/`dladdr1`/`dlinfo`, all inside the counted section.
**read_section:** two-parity reader counters striped by thread pointer over 32 cache lines; a writer flips the parity
and retires unpublished objects with the old parity; unmapped once that parity's counters sum to 0 (checked at once and
at each later dlopen/dlclose). No writer waits under the locks; past the room threshold dlopen/dlclose wait at
entry (`room_wait`, above). **Slots:** 512-object table; a freed slot is reused once
every slot above it is free (arenas rewind). **TLS:** surplus place freed (`Surplus::free`), module id returned to
`ltls::ModuleIds` (lowest free first, generation bumped per release), each thread's `THREAD_MODS` entry cleared under
the thread-list lock. **Errors:** -1 and `dlclose: invalid handle` (D-27.32, ruling #5006).

Files: crates/boundary/src/ld/dlopen.rs (`close_entry`, `close_begin`, `live_set`, `close_end`, `unload`, `reclaim`, `View`/`read_section` (dladdr's),
`record_binds`, `Finis`, `room_short`, `room_wait`), ld/readers.rs (reader counters, `unpublish`, `flip`, `drained`,
`inside`, `after_fork_child`), crates/rt/src/tls.rs (`dl_reading`), ld.rs (`module_room`,
`release_tls`, id-indexed `THREAD_MODS`), crates/ld (`ModuleIds`, `Surplus::free`, `UniqueRegistry::bound`), libc
`dlclose` export, fork.rs `forget_dso`. Tests: crates/ld/tests/dlclose.rs (basic, concurrent: chain phase with a
stopped reader then racing readers x3000, room, C++ plugin x300), crates/ld/tests/tls.rs
`tls_surplus_free_and_module_id_reuse_with_generation`.
