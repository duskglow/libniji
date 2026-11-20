# DYN-START-QUICK: drop the cheap-to-remove system calls of every dynamic start

State: branch `task/dyn-start-quick` (from origin/next bd8fbb61a). Code head 421f140be (`Keep the scratch arena to
the start's own view of the program`) on top of `Remove start-up system calls the loader does not need`, `Test that
a dynamic start makes none of the removed system calls`, `Keep the loader's NULL-folding decision`. Decisions in
force: D-4.22 (NULL folding), D-27.18 (image views), D-27.24 (early start), D-22.9 (thread mappings), D-20.1 / OD-8
(the internal signal).

## Items (brief order): removed, or left with the reason

| # | call(s) per start | where | outcome | saving |
|---|---|---|---|---|
| 1 | openat/read/close `/proc/sys/vm/mmap_min_addr` | ld.so's own copy of the boundary crate: its first copy or fill of 64 bytes or fewer takes the NULL-folding decision (dispatch.rs `resolve_fold_slow`) | **tried and reverted**: taking the decision as "off" in the loader-only build removed the 3 calls but cost about +33k USER instructions per start on every row (the loader's many small start-up copies then take the checked path). Fix-forward for DYN-START-RING: make the loader's own copies not go through the exported entries (inline fixed-size stores), then the read can go | 0 (3 possible) |
| 2 | `lseek(fd, 0, SEEK_END)` after `fstat` | image.rs `open_file`, `open_candidate` | **removed** for regular files (`st_size` from the fstat already made); any other kind keeps lseek (a block device reports its size only there) | 1 per loaded object |
| 3 | mmap 960 B + mmap 448 B, unmapped at the end of `link` | image.rs `image_view` / `scratch` | **removed**: `link`'s view of the program (`published` false) takes its buffers from a one-page, page-aligned, bump-only arena in ld.so's zero-filled data (each range handed out once; length 0 recorded, nothing to unmap). Every view read after the start (`adopt_start`, `own_object`, dlopen) is mapped as before: given the arena too (after2 head), `dladdr` of a dlopened object's address answered ld.so (conformance s27_dladdr, reproduced by a probe: `fname ld.so, fbase` below the object) and two dlclose tests failed; the exact mechanism in the adopt path was not traced (OPEN, informational) | 4 calls |
| 4 | `uname` | thread.rs `no_huge_pages` via `map_thread` (D-22.9) | **left**: decides a THP property of the initial thread's mapping; structural (DYN-START-RING) | - |
| 5a | `arch_prctl(ARCH_GET_FS)` | runtime.rs `start` | **removed**: a non-NULL `rtld_fini` (libniji's ld.so always passes one; the kernel passes NULL to a static program) tests the early-start flag first | 1 call |
| 5b | the early `ARCH_SET_FS` | ld/entry.rs step 2 | **left**: the loader reads thread-block words through %fs from step 3 on; DYN-START-RING | - |
| 6 | `rt_sigaction(32)` | cancel.rs `install` | **left, not provably safe**: conformance `s20_internal_signal.c` (OD-8, D-20.1) requires a handler for 32 at start that absorbs a raw 32 from the program; lazy would let such a signal end the process. RULING NEEDED | - |

## Measurements (dsq-probe.sh: kprobe.sh copy, median of 101, ASLR off; ~/work/dyn-start-quick on mei and keke)

The probe's user count grows with the LD_LIBRARY_PATH and program path lengths (Intel PT trace, mei: `ld_env`,
`search`, `link` cost per byte), so base is measured from `base00` (same length as `after2`/`after3`). Earlier
`base` rows (4 characters) are 55-70 instructions low; do not compare them.

| row | Skylake-SP base00 user / kernel / faults / syscalls | after2 | Zen 1 base00 | Zen 1 after2 | 20.2 syscalls (Zen 1) | glibc 2.39 (Zen 1) user / kernel / syscalls |
|---|---|---|---|---|---|---|
| n0 | 832,365 / 874,492 / 62 / 44 | 832,310 / 838,274 / 62 / 38 | 832,205 / 911,462 / 62 / 43 | 832,150 / 870,208 / 62 / 37 | 44 | 100,437 / 702,951 / 42 |
| n3 | 1,095,724 / 1,311,659 / 78 / 89 | 1,095,663 / 1,272,044 / 78 / 80 | 1,095,564 / 1,364,295 / 78 / 88 | 1,095,503 / 1,319,463 / 78 / 79 | 89 | 135,574 / 1,000,741 / 68 |
| nt | 867,445 / 908,226 / 66 / 44 | 867,390 / 877,467 / 66 / 38 | 867,314 / 951,497 / 67 / 43 | 867,259 / 912,815 / 67 / 37 | 44 | 102,576 / 707,843 / 42 |

after2 = head ba1fe1d5e (arena on every path); after3 (head 421f140be, arena only at start) must give the same start
numbers (the start path is the same; adopt runs only at a first dl* call). Kernel faults 3 on every libniji row.
Ratchet on after2: every counter equal or lower on every row, both CPU classes.

## Tests and plants

`init_dynamic_empty_main_makes_no_removable_start_system_call` (hot_text_products.rs, oracle `oracle/start_syscalls.c`,
ptrace; the environment goes to the traced program only, as arguments). Plants: handoff/DYN-START-QUICK-plants.txt
(5: size-by-lseek, arena-off, arena-overlap, arena-after-start (must fail s27_dladdr), get-fs-first).

## Jobs on mei:~/w/s1 (chain ~/work/dyn-start-quick/chain3.sh, started 18:39Z on head 421f140be)

In order: dyn-start-quick-lint3 (cargo clippy --all-features -D warnings + xtask clippy + static),
dyn-start-quick-after3 (so + probe into /dev/shm/claude-dsq/after3 + dlmap3 diagnostic: after3 must say `fname
.../libdla_syms.so`), then dyn-start-quick-plants3 (detached, /mnt/zram/claude/dyn-start-quick/plants-3),
dyn-start-quick-gate3, dyn-start-quick-xtest3, dyn-start-quick-fullprof3.
SUPERSEDED: dyn-start-quick-so/so2/so3/lint/base/plants/gate/xtest (older heads), -lint2/-after2/-gate2 (gate2 red:
s27_1_c_6 + the tracer exit 127, both fixed in 421f140be), -xtest2 (same plus 2 dlclose tests, fixed by the arena
scope), -plants2/-fullprof2 (killed, head ba1fe1d5e). Diagnostics (not proof): -ptdiag..5, -base00, -dlmap/2 (mei),
keke: -zenbase, -zencmp.

## Exact next step (next leg)

1. `~/bin/jobwait dyn-start-quick-lint3 dyn-start-quick-after3` on mei; check after3 rows = after2 rows above
   (user, faults, syscalls exact; kernel within noise) and the dlmap3 lines; Zen 1: copy after3's three files
   (ld-linux-x86-64.so.2, libc.so.6, niji-crt1.o) to keke ~/work/dyn-start-quick/after3/niji and run dsq-probe
   under jobrun (label dyn-start-quick-zen3).
2. jobwait gate3, xtest3, fullprof3, plants3 (plants: about 30 min of `so` per copy under load); fix what they find.
3. Write handoff/reports/DYN-START-QUICK.json (+ "jobs"), push, `cargo xtask precheck --jobs <labels> --head <sha>`
   on the role, then `ops handback-check`.
4. Hand-back: RULING NEEDED item 6 (recommend: keep eager until a ruling; the host C library installs no handler at
   start, so lazy would save 1 call per start). INFORMATIONAL: the arena-after-start dladdr mechanism not traced.
   Leftovers for DYN-START-RING: items 1 (fix-forward above), 4, 5b. Do not redo items 2, 3, 5a.
Process notes for the report: leg 1 used `sed -i` once on a scratch commit-message file and one `pgrep -f` (read-only);
leg 2 used `sed -i` once on a scratch script on mei (~/work, not the tree), which also damaged that script's run line
(caught, rerun as dlmap2). IMPROVEMENT candidate: the probe kit should pin its directory names' lengths (or use
fixed-length symlinks) so before/after user counts compare without a path-length artefact.
