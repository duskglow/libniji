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

after2 = head ba1fe1d5e (arena on every path). **after3 = final code head 421f140be** (arena only for link's view),
measured in the same job as its base00 (Zen 1) or the same day (Skylake-SP):

| row | Skylake-SP after3 user / kernel / faults / syscalls | Zen 1 base00 (same job) | Zen 1 after3 |
|---|---|---|---|
| n0 | 832,331 / 843,633 / 62 / 38 | 832,205 / 904,564 / 62 / 43 | 832,171 / 864,538 / 62 / 37 |
| n3 | 1,095,687 / 1,272,802 / 78 / 80 | 1,095,564 / 1,351,181 / 78 / 88 | 1,095,527 / 1,307,511 / 78 / 79 |
| nt | 867,414 / 880,967 / 66 / 38 | 867,313 / 944,379 / 67 / 43 | 867,283 / 905,058 / 67 / 37 |

Ratchet against base00 (origin/next, same path length): every counter equal or lower on every row on both CPU
classes. User -31..-37 instructions; kernel -31k..-44k (median of 101); faults equal; syscalls -6 (n0, nt), -9 (n3).
after3's user count is +21..+24 over after2 (the published-view test in `image_view`), still below base00. Kernel
faults 3 on every libniji row. The mei rows have one more openat than keke's on every variant (host environment).
Saving per item (syscalls per start): item 2: 1 per loaded object (1 on n0/nt, 4 on n3); item 3: 4; item 5a: 1.

## Tests and plants

`init_dynamic_empty_main_makes_no_removable_start_system_call` (hot_text_products.rs, oracle `oracle/start_syscalls.c`,
ptrace; the environment goes to the traced program only, as arguments). Plants: handoff/DYN-START-QUICK-plants.txt
(5: size-by-lseek, arena-off, arena-overlap, arena-after-start (must fail s27_dladdr), get-fs-first).

## Jobs on head 421f140be (mei:~/w/s1 chain ~/work/dyn-start-quick/chain3.sh; keke for Zen 1)

dyn-start-quick-lint3 rc 0; dyn-start-quick-after3 rc 0 (13 layout contracts hold; Skylake-SP rows above);
dyn-start-quick-gate3 rc 0 (7 suites ok, 1400 passed, 0 failed, 80 ignored; conformance 22 passed; peak 1.5G);
keke:dyn-start-quick-zen3 rc 0 (Zen 1 rows above); dyn-start-quick-dlmap3b (diagnostic) after3 `dladdr` names
`libdla_syms.so` like base00 (dlmap3 inside after3 and dyn-start-quick-dlmap3 found no conformance helper yet: rc 1
for base00 too; rerun after the gate built it). dyn-start-quick-xtest3 rc 0 (2505 s); dyn-start-quick-fullprof3
rc 0 (hot_text_products 32 passed). dyn-start-quick-plants3 SUPERSEDED (copy 1's baseline timed out at 900 s: its
traced `cargo xtask so` took 900-1386 s beside xtest3; killed). dyn-start-quick-plants4 (same file, --timeout 2400,
/mnt/zram/claude/dyn-start-quick/plants-4) PENDING at hand-back: both baselines ok at 20:31Z, plants running; the
role could not be synced to the handoff-only commits while it ran (sync BUSY), so precheck was not run.
SUPERSEDED: dyn-start-quick-so/so2/so3/lint/base/plants/gate/xtest (older heads), -lint2/-after2/-gate2 (gate2 red:
s27_1_c_6 + the tracer exit 127, both fixed in 421f140be), -xtest2 (same plus 2 dlclose tests, fixed by the arena
scope), -plants2/-fullprof2 (killed, head ba1fe1d5e). Diagnostics (not proof): -ptdiag..5, -base00, -dlmap/2 (mei),
keke: -zenbase, -zencmp.

## Next step

Code done (items 2, 3, 5a; do not redo). Open: RULING NEEDED item 6 (recommend: keep the handler eager until a
ruling; the host C library installs none at start, so lazy would save 1 call per start). INFORMATIONAL: the
arena-after-start dladdr mechanism was not traced. Leftovers for DYN-START-RING: items 1 (fix-forward above), 4, 5b.
Report: handoff/reports/DYN-START-QUICK.json.
Process notes for the report: leg 1 used `sed -i` once on a scratch commit-message file and one `pgrep -f` (read-only);
leg 2 used `sed -i` once on a scratch script on mei (~/work, not the tree), which also damaged that script's run line
(caught, rerun as dlmap2). IMPROVEMENT candidate: the probe kit should pin its directory names' lengths (or use
fixed-length symlinks) so before/after user counts compare without a path-length artefact.
