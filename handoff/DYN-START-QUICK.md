# DYN-START-QUICK: drop the cheap-to-remove system calls of every dynamic start

State: branch `task/dyn-start-quick-2` (DYN-START-QUICK-2, continuing `task/dyn-start-quick`; from origin/next
bd8fbb61a). Code head 8120ed904 (`Keep the static start's thread-pointer test at one word`) on top of 421f140be (`Keep
the scratch arena to the start's own view of the program`), `Remove start-up system calls the loader does not need`,
`Test that a dynamic start makes none of the removed system calls`, `Keep the loader's NULL-folding decision`; then
ade778cc4 (startup_instr_c limit 2161 -> 2155) and handoff commits. Decisions in force: D-4.22 (NULL folding),
D-27.18 (image views), D-27.24 (early start), D-22.9 (thread mappings), D-20.1 / OD-8 (the internal signal).

## DYN-START-QUICK-2: the CI instruction red (static startup_instr_c 2162 > 2161)

Cause: item 5a's `rtld_fini` test runs on every static start too (`test %r9; je`), before the ARCH_GET_FS query the
static program still makes; not a layout flip (ruling #7061 does not apply: the delta is in `runtime::start::<false>`'s
own instructions). Fix (8120ed904): the query writes into a zeroed word (`sys::thread_pointer_or_zero`) and `start`
tests only that word; a failed query leaves 0, read as static, as the old error-means-static mapping did. That drops
the result's test and branch from the static path. Measured on the limit's CPU (Zen 1, keke, the staged
`cbench_startup` binary with `CBENCH_COUNTERS=1 CBENCH_ROWS=startup_instr_c`, 3 rounds b/o/h interleaved, every
round exact): **merge base code 2157, CI-red head code 2159 (+2), final head 2151 (-6)**. Trees: one role build each
(`~/work/dyn-start-quick-2/stage1.sh` on mei: head, head minus the fix, head minus the branch's crates/boundary/src
diff; restored and `cmp`-checked), counted by `~/work/dyn-start-quick-2/count.sh` on keke (jobs
dyn-start-quick-2-stage1, keke:dyn-start-quick-2-count). CI's 2162 was a merge with a newer next (+3 over this
base), so the merged head should count about 2154. Limit tightened by the measured gain: 2161 -> 2155 (ALWAYS
FORWARD). Bytes: `runtime::start::<false>` is 5234 B (base), 5285 (CI-red head), 5355 (final): +121 B over base with
fewer instructions run (register allocation); the start-up layout and fault checks (xtask test, full-profile
hot_text_products) are in the proof chain below. Plant `tp-always-set` (handoff/DYN-START-QUICK-2-plants.txt): a
static program read as having a thread pointer must fail the static fault probes.

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

## DYN-START-QUICK-2 jobs (final code head 8120ed904; mei:~/w/s2, chain ~/work/dyn-start-quick-2/chain2.sh)

dyn-start-quick-2-lint0 rc 0 (inherited head 7a1643f28: all-features clippy, xtask static); dyn-start-quick-2-stage1
rc 0 (clippy + static on 8120ed904, then the three staged trees); keke:dyn-start-quick-2-count rc 0 (the counts
above; keke:dyn-start-quick-2-try/-try2 were command trials on the head tree, 2151 too);
dyn-start-quick-2-gate rc 0 (direct tier: 15 suites ok, 11045 passed, 0 failed, 1503 ignored; conformance 69
passed; SKIPPED 0; peak 15G page cache included, oom_kill 0); dyn-start-quick-2-plants rc 0 (6 plants: 5 CAUGHT by
their message; tp-always-set WARN-NO-MSG: caught by the products build itself, `trace-at-build: tracing failed` (the
build's static probes cannot start), so the named test never ran; the next plants file should add `tracing failed`
to its msg); dyn-start-quick-2-xtest and dyn-start-quick-2-fullprof: see the hand-back (JOBS line).

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

Code done (items 2, 3, 5a, and DYN-START-QUICK-2's static-path fix; do not redo). Item 6 is ruled (#7075, owner
"Make it lazy is fine"): DYN-START-RING makes the signal-32 handler lazy with a differences row; nothing left here. INFORMATIONAL: the
arena-after-start dladdr mechanism was not traced. Leftovers for DYN-START-RING: items 1 (fix-forward above), 4, 5b.
Report: handoff/reports/DYN-START-QUICK.json.
Process notes for the report: leg 1 used `sed -i` once on a scratch commit-message file and one `pgrep -f` (read-only);
leg 2 used `sed -i` once on a scratch script on mei (~/work, not the tree), which also damaged that script's run line
(caught, rerun as dlmap2). IMPROVEMENT candidate: the probe kit should pin its directory names' lengths (or use
fixed-length symlinks) so before/after user counts compare without a path-length artefact.
