# SYS-LIBMVEC / SYS-LIBMVEC-2: report

Branch `task/sys-libmvec-2`, role keke:~/w/n2 (Zen 1). Handoff: handoff/SYS-LIBMVEC.md.

## Commits (run 2)
* 57abcb1c4 Package libmvec and record its per-lane scalar method (tools/deb/build.sh, system.sh,
  check-system.sh; records/pending/SYS-LIBMVEC-2-1.toml; docs/differences-from-glibc.md; plants file).
* 6da8ce5c6 Let the swapped-root check run extra programs (swap-system.sh SWAP_EXTRA_PKGS / SWAP_EXTRA_CMDS,
  swap-cmds.sh sources the extra file, tools/deb/swap-extra-libmvec.sh).
* Handoff and report commits on top (handoff/ only).

## Jobs (keke), all on the code content of 6da8ce5c6
* `sys-libmvec-2-lint0` rc 0: clippy + static on the inherited head (nothing red inherited).
* `sys-libmvec-2-lint1` rc 0: records check (1 pending) + static after the records commit.
* `sys-libmvec-2-gate2`: `cargo xtask affected --base origin/next --run`, tier Direct; the xtask
  change puts xtask's commands in the gate, so it runs `xtask test` (nextest over the workspace) on 4 cores: still
  running at hand-back (clippy step ok), left for the dispatcher (PENDING JOBS).
* `sys-libmvec-2-plants2` rc 0: 5 of 5 CAUGHT, no WARN-RC-ONLY / WARN-NO-MSG (only WARN-DEV-SHM, the RAM dir note):
  mvec-vv-swap and mvec-static-local by `libmvec_lanes_equal_the_scalar_functions`; mvec-no-local,
  mvec-all-compat and mvec-float-as-double by the product build's own check (`cargo xtask so` refuses the mutant).
* `sys-libmvec-2-debs2` rc 0: both flavours built from the role tree as commit 6da8ce5c6 and checked:
  `check: PASS (2 packages)` (niji6, niji6-dev) and `check: PASS (6 packages, version 2.39+niji20261117.1-1)`.
  niji6 and libc6 hold libmvec.so.1 (66872 bytes); niji6-dev and libc6-dev hold libmvec.a and
  `libmvec.so -> libmvec.so.1`; libc6's symbols file lists the 216 `_ZGV` exports, shlibs `libmvec 1 libc6 (>= 2.39)`.
* Superseded: `sys-libmvec-2-gate`, `-plants`, `-debs` (stopped with kill-label after 10 min: the swap tooling
  commit followed, so they would not have tested the final content).

## Swapped root (evidence, not in JOBS)
`sys-libmvec-2-swap` (rc 1 by design of the comparison: any differing command): trixie root, libniji's libc6 and
libc-bin swapped in (dpkg rc 0), SWAP_EXTRA_PKGS=direwolf,cif2hkl. `ldd` in the libniji root: atest, direwolf and
cif2hkl load libmvec.so.1 (gen_packets does not). Same output and status as the stock root: mvec-ldd,
mvec-direwolf-gen, mvec-direwolf-decode (atest decodes the 14 frames gen_packets wrote), mvec-cif2hkl-powder.
mvec-cif2hkl-xtal: same status; 16 rows of |F|^2 differ in the 7th significant digit (0.14305675E+03 vs
0.14305682E+03): libniji's lanes are its scalar results and the stock root's are the vector variants (documented
4-ulp bound), the accuracy difference the new differences row records. RESULT same=58 differ=8: the other 7 are
ldd-1..7, which differ only in the directory the listing prints (`/lib/x86_64-linux-gnu/...` stock,
`/usr/lib/x86_64-linux-gnu/...` libniji): from the base command set, not from this story (INFORMATIONAL).

## Probe-settled behaviour
Reference (black box, keke): `_ZGVbN2v_log` of {0, 1} and {-1, 1}, `_ZGVbN2v_exp` of {1000, 1} and `_ZGVbN4v_logf`
of {0, 1, 1, 1} set errno ERANGE / EDOM / ERANGE / ERANGE and raise divbyzero / invalid / overflow / divbyzero,
as libniji's per-lane scalar calls do: no difference, so the record states it and no row lists it.

## Not done
* `tools/deb/parity.sh` rerun: it downloads the archive indices and streams a 4000-package sample (network, hours
  of host time); out of this run's budget. The three reference files (ref-files.tsv 697, 1643, 1981) are now in the
  packages and the soname is exported, so row 2 cannot reappear; a rerun belongs to the next audit refresh.

## RULING NEEDED / INFORMATIONAL
* Pending record `records/pending/SYS-LIBMVEC-2-1.toml` (D-NEW, D-10): CI assigns the id.
* INFORMATIONAL: swap-system's ldd-1..7 differ on a trixie root (path prefix of the listing), independent of libmvec.
