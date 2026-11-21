# DYN-START-SPEED-2 report

Result: the CI red (release startup-only faults 68 > 67 under the gate wrapper) is fixed at its cause; the
dynamic start-up fault count no longer depends on the file system holding ld.so and libc.so.6; the Zen 1 bounds
are measured. RELR (step 3) and the rest of the fault gap were not reached.

## Commits (branch task/dyn-start-speed-2)
- merge of origin/next 90f71362a (clean; brings DYN-START-PARITY-3's DYN_TRUE_BOUND [73, 67]).
- b849c7ce2 Place the loader's whole-file views at a 64 KiB-aligned address; 1b976ec1e wrapping arithmetic (clippy).
- 054bf52fd Zen 1 fast instruction bound 872,600 (was 0), measurements in the comment; plant file; handoff.
- the report commit (this file, handoff/reports/DYN-START-SPEED-2.json).

## Cause of the red (evidence)
Step 0: clippy + static green on the inherited head (lint0). Release, mei (Skylake-SP), probe = the test's own
staging, argv and environment (`~/work/DYN-START-SPEED-2/dynprobe.sh`), warm-up + 5 runs, every run identical:
origin/next 67 (ext4 /tmp) / 67 (tmpfs /tmp); merged head 67 / 68; merged head + fix 67 / 67. perf fault addresses
(`perf record -e minor-faults -d`): the only differing mapping is libc.so.6's whole-file read-only view (per
process 4 faults on ext4, 5 on tmpfs), touched by niji_ld::reloc::pass1 and reloc::target. The kernel placed the
view at 0x7ffff6e00000 (2 MiB-aligned) on ext4 and at 0x7ffff6f0c000 on tmpfs. So the extra fault is this branch's
loader reading view pages that straddle a 64 KiB window at tmpfs's phase, made visible by file-system-dependent
placement. The ldso-text.list hypothesis does not hold: that list is traced at every build (xtask so.rs
run_ld/ldso_layout) and cannot be stale. No bound changed.

## Fix
`map_view` (crates/boundary/src/ld.rs): reserve len + 64 KiB PROT_NONE, MAP_FIXED the file at the 64 KiB-aligned
address inside, unmap both ends. Used by every whole-file view (open_file, open_candidate, the `ld.so PROGRAM`
path, list mode open_whole, dlopen's candidate). The view's windows then follow its file offsets on every file
system. Cost: two more munmap calls and one mmap per object opened (kernel work, a few user instructions).
Test: `init_dynamic_startup_fault_count_does_not_depend_on_the_file_system` stages the same products under the
build tree (ext4) and on /dev/shm (tmpfs) with equal-length paths and requires one count (skips with the reason
when both are the same file system). Plant view-candidate-kernel (handoff/DYN-START-SPEED-2-plants.txt): CAUGHT.

## Measurements, final code (median of 21, ASLR off; the same mei-built products on both hosts)
| host | fast instr | release instr | host C library | faults fast ext4/tmpfs | faults release ext4/tmpfs |
|---|---|---|---|---|---|
| Skylake-SP | 855,582 | 803,431 | 101,863 | 73 / not probed | 67 / 67 |
| Zen 1 | 855,428 | 803,280 | 96,002 | 73 / 73 | 67 / 67 |
Release before DYN-START-SPEED: 1,579,278 (execcount). glibc faults 51 (execcount) on this program class.

## Jobs (mei role ~/w/s1)
Proofs on the final head: rel4 (clippy + static + release so + hot_text_products 30 passed, gate wrapper), gate2
(direct tier `affected --base origin/next --run`: 304 passed), plants3 (handoff/DYN-START-SPEED-2-plants.txt),
xtest2 (cargo xtask test), fin2 (cargo xtask so + hot_text_products fast, 30 passed; final head differs only in prose).
Measurement runs (not proofs; same code, products of 1b976ec1e): meas1 (release probe ext4/tmpfs), sky1
(Skylake-SP instructions + faults), keke:dyn-start-speed-2-zen1 (Zen 1).
Superseded: lint0 (inherited head, green); rel1 (merged head, 68 > 67 reproduced); rel2 (clippy arithmetic red,
fixed); rel3, gate1 (304 passed), plants2 (1/1 CAUGHT), xtest1 (green): all green on 1b976ec1e, before the Zen 1
bound edit; fin1 (clippy + static passed; hot_text_products ran on products built from the previous tree, so the
prefixed interpreter refused that libc.so.6 by its NIJI_PRIVATE build tag; fin2 rebuilt and passed); diag1 (that
diagnosis); prepnext (old ~/wl/bin/wl_prep.sh, fast build, failed copy; I removed the empty products dir it left),
nextrel (next's release products + probe), probe1 (probe script bug), probe2 (merged head ext4/tmpfs), plants1
(plant file not yet synced).

## INFORMATIONAL
- ~/wl/bin/wl_prep.sh on mei is an old copy (target/liella paths, fast profile); validation/wl_prep.sh in the
  tree is current. The current one's `seq` kit link failed for 90f71362a (log ~/wl/kit/90f71362a...-release/link-seq.log;
  not chased, products built fine).
- Process slips this run: one empty heredoc (`python3 - <<'X'` with no body, ran nothing) despite the rule; one
  amend of an already pushed commit, undone by `git reset --soft` to the pushed commit and a new commit (no force
  push). One ssh call backgrounded a `cp && jobrun` chain and held the session (the job itself ran).

## Next
RELR measurement (step 3); then the view's own 4 faults per process (read symbols and relocations through the
mapped segments), relocated data pages, start frame; profile items in handoff/DYN-START-SPEED.md.
