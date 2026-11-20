# DYN-START-QUICK: drop the cheap-to-remove system calls of every dynamic start

State: branch `task/dyn-start-quick` (from origin/next bd8fbb61a), commits `Remove start-up system calls the
loader does not need` and `Test that a dynamic start makes none of the removed system calls`; HEAD in
handoff/reports/DYN-START-QUICK.json. Decisions in force: D-4.22 (NULL folding), D-27.18 (image views),
D-27.24 (early start), D-22.9 (thread mappings), D-20.1 / OD-8 (the internal signal).

## Items (brief order): removed, or left with the reason

| # | call(s) per start | where | outcome | saving |
|---|---|---|---|---|
| 1 | openat/read/close `/proc/sys/vm/mmap_min_addr` | ld.so's own copy of the boundary crate: its first copy or fill of 64 bytes or fewer takes the NULL-folding decision (dispatch.rs `resolve_fold_slow`) | **tried and reverted** (commit `Keep the loader's NULL-folding decision`): taking the decision as "off" in the loader-only build removed the 3 calls but cost about +33k USER instructions per start on every row (Skylake-SP: n0 832,292 -> 865,213; n3 1,095,600 -> 1,129,279; nt 867,372 -> 903,647), because the loader's many small start-up copies (relocation stores, `copy_from_slice`) then take the checked path. Ratchet: reverted. Fix-forward for DYN-START-RING: make the loader's own copies not go through the exported entries (inline fixed-size stores), then the read can go | 0 (3 possible) |
| 2 | `lseek(fd, 0, SEEK_END)` after `fstat` | image.rs `open_file`, `open_candidate` | **removed** for regular files: `file_size` takes `st_size` from the fstat already made; any other kind (a block device reports its size only through lseek) keeps lseek, so no behaviour changes. The program's own `load` and `open_whole` (cache, list mode) make no fstat and keep lseek (not on the normal start path) | 1 per loaded object (n0: 1, n3: 4) |
| 3 | mmap 960 B + mmap 448 B, both unmapped at the end of `link` | image.rs `image_view` (`scratch`): the program's header table + span list, and its PT_DYNAMIC copy | **removed**: `scratch` takes them from a one-page, page-aligned arena in ld.so's zero-filled data (bump-only, CAS, never reused, so each range is handed out once; `maps` records length 0, nothing to unmap); a buffer that does not fit is mapped as before (any number of PT_LOADs still works, D-27.15). The same arena serves `adopt_start`'s views while it has room (they are kept for the process anyway) | 4 calls, 2 fresh anonymous pages -> 1 data page |
| 4 | `uname` | thread.rs `no_huge_pages` via `map_thread` for the initial thread's adopted descriptor mapping (D-22.9: kernels before 6.7 need MADV_NOHUGEPAGE on thread mappings) | **left**: the call decides a THP property of the mapping; the only syscall-free source of the kernel version is the vDSO's `Linux` ELF note, and whether the initial thread needs that mapping at all is structural (a descriptor in static storage needs neither mapping nor uname). Hand to DYN-START-RING | - |
| 5a | `arch_prctl(ARCH_GET_FS)` | runtime.rs `start` (static-or-dynamic test) | **removed**: when `rtld_fini` is non-NULL (libniji's ld.so always passes its termination function in %rdx; the kernel passes NULL to a static program) the early-start flag is tested first; the old test stays for any other start, so every path behaves as before and a static program still never touches the flag | 1 call |
| 5b | the early `ARCH_SET_FS` | ld/entry.rs step 2, the provisional thread block (L14-2) | **left**: the loader's own code reads thread-block words through the thread pointer from step 3 on, before the final TLS layout exists; dropping it needs every %fs access on the start path audited or the final block placed before linking: DYN-START-RING | - |
| 6 | `rt_sigaction(32)` | cancel.rs `install`, called at start | **left, not provably safe under the current contract**: the senders in-process (cancel `request`, setxid broadcast) could install first, but the conformance test `conformance/c/s20_internal_signal.c` (OD-8, D-20.1) requires "the library has a handler for 32 at start" and that a raw signal 32 raised by the program is absorbed by the library's handler (lines 175, 339-354). Installed lazily, a 32 from outside the library before first use would end the process. The probe shows the host C library installs no handler at start, so making it lazy is a decision change: RULING NEEDED (see report) | - |

## Measurements (dsq-probe.sh: kprobe.sh copy, median of 101, ASLR off; kit in ~/work/dyn-start-quick/kit on mei and keke)

Skylake-SP (mei), default `cargo xtask so` profile. base = origin/next bd8fbb61a (my patch reversed, built on the
role); after1 = items 1+2+3+5a (superseded head 6779186f8). 20.2 products are a release build, so not comparable
on user instructions/faults (profile differs); syscalls are.

| row | base user / kernel / faults / syscalls | after1 user / kernel / faults / syscalls | 20.2 syscalls | glibc 2.39 user / kernel / syscalls |
|---|---|---|---|---|
| n0 empty main | 832,292 / 877,195 / 62 / 44 | 865,213 / 829,869 / 62 / 35 | 45 | 106,444 / 723,923 / 46 |
| n3 3 libraries | 1,095,600 / 1,313,927 / 78 / 89 | 1,129,279 / 1,261,901 / 78 / 77 | 90 | 141,648 / 1,019,268 / 72 |
| nt clock_gettime | 867,372 / 913,138 / 66 / 44 | 903,647 / 864,773 / 66 / 35 | 45 | 108,554 / 727,520 / 46 |

after2 (final head, items 2+3+5a only) = job `dyn-start-quick-after2` on mei (log ~/logs/dyn-start-quick-after2.log,
traces /dev/shm/claude-dsq/after2/*.st). Expected: syscalls n0 38, n3 80, nt 38 (item 2: 1 per object; item 3: 4;
item 5a: 1); user instructions must be <= base on every row (if not: revert the item that raised it, ratchet).
Zen 1 (keke): NOT measured yet: copy ~/work/dyn-start-quick/{base,after2}/niji from mei to keke and run
`sh ~/work/dyn-start-quick/kit/dsq-probe.sh <niji dir> /dev/shm/claude-dsq/<name> glibc` under jobrun there.

## Tests and plants

`init_dynamic_empty_main_makes_no_removable_start_system_call` (crates/core/tests/hot_text_products.rs, oracle
`oracle/start_syscalls.c`, ptrace): no lseek, no read, no anonymous mapping under a page, no ARCH_GET_FS in a
dynamic empty main. Plants: handoff/DYN-START-QUICK-plants.txt (5).

## Jobs running on mei:~/w/s1 (chain ~/work/dyn-start-quick/chain2.sh, started on head ba1fe1d5e)

In order: dyn-start-quick-lint2 (clippy + static), dyn-start-quick-after2 (products + probe), then
dyn-start-quick-plants2 (detached, /mnt/zram/claude/dyn-start-quick/plants-2, 4 plants), dyn-start-quick-gate2
(affected --run), dyn-start-quick-xtest2 (cargo xtask test), dyn-start-quick-fullprof2 (NIJI_FULL_PROFILE so +
hot_text_products). SUPERSEDED (killed, older head): dyn-start-quick-so/so2/so3 (build + probe), -lint, -base
(base probe: its numbers above are valid, base = origin/next), -plants, -gate.

## Exact next step (next leg)

1. `~/bin/jobwait dyn-start-quick-lint2 dyn-start-quick-after2` on mei; fill the after2 row above; check the
   ratchet (user, kernel, faults, syscalls <= base on n0, n3, nt).
2. Zen 1 rows on keke (see Measurements).
3. jobwait plants2, gate2, xtest2, fullprof2; fix what they find (hot_text_products' instruction bound may say
   "more than 10% under: tighten DYN_START_INSTR" only if the start got much cheaper: tighten, never loosen).
4. Write handoff/reports/DYN-START-QUICK.json (+ "jobs"), push, `cargo xtask precheck --jobs <labels> --head <sha>`
   on the role, then `ops handback-check`.
5. Hand-back: RULING NEEDED item 6 (lazy internal-signal handler changes D-20.1/OD-8; recommend: keep eager until
   a ruling; the host C library installs none at start, so lazy would match it and save 1 call per start).
   Leftovers for DYN-START-RING: items 1 (fix-forward above), 4, 5b. Do not redo items 2, 3, 5a.
Process notes for the report: I used `sed -i` once on a scratch commit-message file and one `pgrep -f` check
(read-only) on mei; both against the rules, no effect on the tree.
