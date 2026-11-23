# TLS-DYNAMIC-BLOCKS: on-demand TLS blocks for dlopened objects without DF_STATIC_TLS (DL-19)

State: branch `task/tls-dynamic-blocks`, head bfce16483 (all pushed). Brief:
~/state/briefs/TLS-DYNAMIC-BLOCKS.md. Role: mei:~/w/s1. Leg 1: 89 calls; leg 2: ~115 calls.

## Done (do not redo)

* Code compiles, clippy and static clean (on 73c8f7af6; bfce16483 adds only report.rs + records
  + docs and is NOT yet synced/clippied: two syncs returned rc 4 BUSY while plants ran).
* Leg 2 fixes: `block_image` copies the image byte-wise through atomics (the TLSDESC dynamic
  trampoline saves only FXSAVE state; ld.so's memcpy dispatches to AVX2 kernels: disassembly of
  block_alloc/block_image now shows only mmap/munmap; libc's `__niji_thread_walk` reaches only the
  lock and callbacks). `initial_place` and readers.rs `tls_module` now map the `DYNAMIC|align`
  word to `NO_STATIC` (dlinfo TLS_DATA and dlpi_tls_data gave garbage: s27_dlinfo failed).
* Tests: `conformance/c/s27_dlopen_tls_dynamic.c` (test fn `s27_3_f_0`, replaces the clause stub;
  helpers dld_gd, dld_desc (gnu2), dld_ie, dld_huge, dlt_big): 64 x 16 KiB with an early and a late
  thread, dlinfo/dl_iterate_phdr NULL before first access, IE object after, 200 open/close cycles
  (statm growth <= 64 pages), OOM child exit 127 + text, tunable 262144 re-exec opens dlt_big.
  dlt_leak is now initial-exec. s27_dlinfo now requires NULL before first use. s01 `s1_8_c_0` gets
  the new field and a parse check. Job `tls-dynamic-blocks-check3` (73c8f7af6 tree): s27_dlopen_
  tls_dynamic, _tls_surplus, s27_dlinfo, s27_dl_iterate_phdr: 4 passed on reference and libniji.
* Table D re-run (job `tls-dynamic-blocks-tabled`, results/tls-surplus/run.sh): libniji loaded=64
  ok with threads=0 and 1 (was 4), the reference 64. (That job's trailing s27_dlinfo FAIL was the
  pre-fix code; fixed since.)
* Records: D-27.94 (amends D-27.39), D-27.95 (amends D-27.49) (ids reserved), differences rows
  (dlinfo row narrowed to DF_STATIC_TLS objects; new surplus row, no-compat reason),
  docs/tunables.md row, `--list-tunables` line (report.rs). `records check` ok.
* Plants file mei:~/work/tls-dynamic-blocks/plants.txt (copy: scratch plants.txt): old-rule-surplus,
  no-free-thread-exit, no-free-dlclose, no-image, tunable-ignored, all on
  `cargo xtask affected --only conformance:s27_dlopen_tls_dynamic --with-exports --run`.
  Job `tls-dynamic-blocks-plants2` RUNNING (started on 73c8f7af6; the planted files are the same at
  bfce16483 except report.rs, not planted). SUPERSEDED: `tls-dynamic-blocks-plants` (killed: stale code).

## Exact next steps

1. `~/bin/jobwait tls-dynamic-blocks-plants2` on mei; all 5 must be CAUGHT (list any WARN).
2. Sync (check rc != 4), clippy + static on bfce16483+.
3. COORDINATOR NOTE 15:06Z (owner #8009, replaces brief step 3): decide the surplus default by
   measurement, 64 KiB vs the reference's 1712 bytes: pthread_create+join instructions per call
   (THREAD-CREATE-COST: 36,092 libniji vs 2,266 reference; say whether the surplus is part of the
   gap), RSS per thread with 100 idle threads, usable stack left in a thread created with
   PTHREAD_STACK_MIN and with a 64 KiB stack, start-up rows. Rule: 64 KiB costs effectively nothing
   per thread -> keep it with the differences row; otherwise match the reference (1712 bytes, no
   row; tunable = value + 1200 as the reference). Report the table and which branch applied. Idea:
   build a scratch variant with `ltls::SURPLUS` = 1712 (crates/ld/src/tls.rs:21) in a copy dir and
   compare (wl_percall thread_create_join; a small probe for RSS/stack). If over budget:
   `ops extend-budget` once.
4. Bench rows (brief 2): `__tls_get_addr`/TLSDESC per-access before/after (`grep -rn tls benchmarks/`);
   fast path unchanged by design.
5. Gate: `cargo xtask affected --run` (63 targets, 148 conformance programs selected: a new helper
   counts for every program) under jobrun; start-up checks (`cargo xtask test`,
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test
   hot_text_products`); layout-bins diff vs merge base; precheck; report JSON; handback-check.

## Open questions / rulings

* IE (`TPOFF`) from one dlopened object into another's on-demand block fails `cannot allocate memory
  in static TLS block` (before, every block had a surplus place). Rare; INFORMATIONAL.
* Lazy blocks (brief) vs eager (probe story's recommendation); a failed allocation exits 127 as the
  reference does.

## Rule slips (report in hand-back)

Leg 1: one `sed -i` (entry.rs comment) and one python in-place rewrite (reloc.rs); should have been
Edit-tool edits; not refused, content verified. Leg 2: none.
