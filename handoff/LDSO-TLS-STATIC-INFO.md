# LDSO-TLS-STATIC-INFO: `_dl_get_tls_static_info` for sanitizer runtimes (GitHub issue #26)

**State:** branch `task/ldso-tls-static-info`, the code is in commit 3bee1e5ae (see the hand-back for the final head). Decision in force:
**D-27.65** (reserved for this story). Related: R14-12 (no `GLIBC_PRIVATE` node, so `NIJI_PRIVATE`), D-27.62 (the
`_dl_debug_state` precedent), D-27.20 (static TLS layout), R14-4 (surplus).

## What changed
* `crates/boundary/src/ld.rs`: `_dl_get_tls_static_info(size_t *size, size_t *align)`, cold and never inlined. `*size` =
  `SURPLUS_HI` (blocks plus the 64 KiB surplus below the thread pointer) + `INITIAL_TAIL` (the thread block above it).
  `*align` = `THREAD_TLS_ALIGN`. A null or misaligned pointer is not written (`tb::c_word`), so no new `unsafe` block.
* `xtask/src/so.rs`: added to `LD_EXPORTS` (exported at `NIJI_PRIVATE`).
* `crates/boundary/src/ld/dlopen.rs` `adopt_start`: when no start-time object names ld.so, it now joins the global
  scope last. This is a second cause found here: libniji's `libc.so.6` has no `DT_NEEDED` on ld.so, and libasan,
  libtsan and libubsan do not name it either. So `dlsym(RTLD_DEFAULT/RTLD_NEXT)` found none of ld.so's exports, and
  exporting the symbol alone would not have fixed #26. `adopt_start` runs on the first `dl*` call, not at start-up.
  The start's relocation scope is unchanged.
* `crates/ld/tests/tls_static_info.rs` (2 tests), `handoff/LDSO-TLS-STATIC-INFO-plants.txt` (5 plants).

## Probe-settled behaviour (black box, the reference ld.so 2.39 on the Zen 1 build host; sources in the run's scratch)
* P-TLSINFO-1: the reference exports `_dl_get_tls_static_info@@GLIBC_PRIVATE` and reports the same values in the main
  thread and in a created thread. Sizes: no TLS 4224; a program block of 1000 B aligned to 64 gives 5248 (+1024); a
  library block of 4096 B aligned to 128 gives 8384 (+4160); both give 9408. Alignment: 64, or 128 with the library.
  A static-pie program has no such symbol.
* P-TLSINFO-2: a library that names only `libc.so.6` finds `_dl_debug_state`, `_dl_get_tls_static_info` and
  `__tls_get_addr` through `RTLD_DEFAULT` and `RTLD_NEXT`. libniji before the change found none of them.
* P-TLSINFO-3: the size changes by exactly the `glibc.rtld.optional_static_tls` tunable (0 / 512 / 1024 give -512 / 0 /
  +512) and shrinks with `glibc.rtld.nns`. Its smallest value (2816) is larger than the thread block's span above the
  thread pointer in a created thread (2368), so the size counts the thread block. libniji's meaning follows from this.

## Sanitizers (gcc 13 toolchain on the build host; ld.so run as a program, `LD_LIBRARY_PATH` = products first)
* ASan: before the change it jumped to PC 0. After: it reports `heap-buffer-overflow` (the issue's reproducer).
* LSan: before the change, SIGSEGV. After: it reports the leak.
* TSan: with ASLR off (`setarch -R`) it reports the same data race as the reference. With ASLR on, the reference
  fails too ("unexpected memory mapping"), which is a host setting. libniji with ASLR on gives SIGSEGV instead of
  that FATAL line (INFORMATIONAL, not chased).
* UBSan: worked before and after.
* HWASan: cannot run here, neither the reference nor libniji ("requires a kernel with tagged address ABI").
* No symbol is still missing in these runs. In as-program mode the sanitizers name the program's frames after
  ld.so (`/proc/self/exe` is ld.so). That comes from the run mode, not from this change.

## Not to redo
* Running a reference-libc tool (`timeout`) with `LD_LIBRARY_PATH` set to libniji's products makes the reference's
  loader stop with an assertion. Put the setting inside `env`, after `timeout`.
* Over ssh, never chain anything before the detached `setsid nohup` command (it held the channel open for 10 minutes).

## Open / next step
* The coordinator can ask Yohane to verify #26 with gcc 14 (here gcc 13 was used).
* RULING NEEDED: none. The global-scope change follows the probe and is recorded in D-27.65.
