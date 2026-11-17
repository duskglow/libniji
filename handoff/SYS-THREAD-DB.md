# SYS-THREAD-DB: libthread_db.so.1 (handoff, checkpoint 1)

State: branch `task/sys-thread-db`, WIP head `649dc9493` (on origin/next `ebb670553`). Role mei:~/w/s4.
Stopped at the context checkpoint (~150k of the run); NOTHING BUILT OR TESTED yet. Job `sys-thread-db-build1` on mei (`cargo xtask clippy && cargo xtask so`, head 85fe2d0f9)
ended rc 1 on clippy::as_conversions in `Debug::new`; fixed in 649dc9493 (not rebuilt).
Nothing of this run is left running.

## Design (in force, to become the decision record)

* gdb `dlopen`s libthread_db into its OWN process (system C library), so the library imports
  nothing but the `ps_*` callbacks (+ possibly compiler-emitted `memset`). It is written in the
  C layer (`crates/libc-so/placeholders/libthread_db.c`): every entry is raw-pointer FFI, and
  `unsafe` is allowed only in boundary and C. The `crates/thread-db` Rust crate stays a stub
  (propose removing it, or keep for a later safe-Rust port).
* Built by the generated-library step (`xtask/src/placeholders.rs`: `LIBRARIES` + `DEV_LINKS`
  now include `libthread_db.so.1`): version nodes GLIBC_2.2.5/2.3/2.3.3 and the 40 exports come
  from `spec/abi/versions.tsv`, checked by `product_errors`. Bodies are `niji_ph_td_*`.
* How it finds the thread list (brief item 3): libc exports `__niji_thread_db@NIJI_PRIVATE`
  (`xtask/src/so.rs` LIBC_PRIVATE), a `#[used]` static in `crates/boundary/src/thread.rs` of type
  `niji_rt::thread::Debug` (17 u64 words: magic `nijitdb1`, version 1, word count, addresses of
  `HEAD`, `LIVE`, `TLS_IMAGE`, `offset_of!` offsets of tid/dtv in the thread block and
  next/tp/flags/start/stack_lo/stack_size/sched_prio in the descriptor, DESC_EXITED, dtv header 2).
  Cost: 3 RELATIVE relocations in libc.so.6 and static-PIE (none in non-PIE static).
* Handle `th_unique` = thread pointer (= `pthread_t`). `td_ta_map_lwp2thr` takes `fs_base`
  (`user_regs_struct` word 21, kernel UAPI) from `ps_lgetregs`; 0 = initial thread before TLS
  setup (get_info reports lid = `ps_getpid`). State: ZOMBIE if tid word 0 or DESC_EXITED.
* TLS: `td_thr_tlsbase` uses the thread's dtv when the loader made one, else (static) module 1's
  block = tp - round_up(memsz, align) from TLS_IMAGE. `td_thr_tls_get_addr` returns TD_NOCAPAB.
* Agents: static pool of 64 (no allocator). Events, TSD, xregs, suspend: TD_NOCAPAB/TD_NOEVENT.

## Open questions / blockers

* DYNAMIC programs cannot be thread-debugged yet: libniji's `ld.so` has no `_r_debug` /
  `_dl_debug_state` (`crates/ld/src/debug.rs` is a stub), so gdb never sees `libc.so.6` (no
  symbol lookup, no link_map for `td_thr_tls_get_addr`). Needs its own story (spec §27.4).
  This story's proof uses STATIC programs.
* gdb loads a libthread_db from `$pdir` only inside `auto-load safe-path`; the test must pass
  `-iex "set auto-load safe-path /" -iex "set libthread-db-search-path <target/niji>"`.
* Check that the initial thread's tid word (tb +0x84) is set in a static program (else gdb skips
  it as ZOMBIE).

## Next step (exact)

1. Sync, rerun clippy + `cargo xtask so`; fix what remains (watch `as` casts in `Debug::new`, the
   `#[used]`+`no_mangle` static, placeholders tests that count LIBRARIES).
2. `tools/deb/system.sh` (system flavour) needs the same two lines as `tools/deb/build.sh`;
   `tools/deb/check-system.sh:138` list may need libthread_db.so.1.
3. Test `crates/core/tests/thread_db_products.rs`: static libniji C program, 4 threads, `__thread
   int tv` distinct per thread, all parked at a barrier then a breakpoint function; gdb -batch:
   expect "[Thread debugging using libthread_db enabled]", 4 `Thread 0x... (LWP n)` lines,
   `thread apply all print tv` values; `generate-core-file`, then a second gdb on the core: same
   `info threads`. Skip with a printed reason when gdb is absent (mei has gdb 15.1).
4. Decision record: `cargo xtask records new decision D-NEW --series D-22 --story SYS-THREAD-DB`
   (the code comments say "decision record SYS-THREAD-DB-1": fix to the pending file name).
5. Plants (cargo xtask plants): fs_base index, D_NEXT walk, tlsbase size rounding, magic check.
6. Gate (direct tier) + start-up checks (touches thread start-up data: `cargo xtask test`,
   LIELLA_FULL_PROFILE hot_text_products), both package flavours, `tools/deb/parity.sh` rerun.

Plan rows to read: plan/package-parity-audit.md row 11; spec §22.10.
