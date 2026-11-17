# SYS-LIBMVEC / SYS-LIBMVEC-2 / SYS-LIBMVEC-3: report

Handoff: handoff/SYS-LIBMVEC.md.

## Run 3 (SYS-LIBMVEC-3): branch `task/sys-libmvec-3`, role mei:~/w/s1 (Skylake-SP)

Cause of the last red: CI's merge with next conflicted in tools/deb (system.sh, check-system.sh,
swap-system.sh): SYS-CONTROL-PARITY-2, SYS-LIBRESOLV-3, SYS-DEV-LINKS-2 and the libm old-versions change had
landed on the same lines. No job of run 2 had failed.

Commits:
* 9d62fdff5 Merge origin/next (2da237f41): both sides kept. system.sh's libc6 list and libc6-dev archive list
  carry next's libresolv.so.2, libBrokenLocale.so.1, libBrokenLocale.a, libg.a, libmcheck.a,
  libpthread_nonshared.a and libmvec.so.1 / libmvec.a; the libmvec.so link sits beside next's dev links;
  check-system.sh's SONAME loop and layout allow both; swap-system.sh's base list gains next's gawk and stays
  the `$include` variable that SWAP_EXTRA_PKGS extends.
* 35250c7134 check-system.sh: libc6-dev's libmvec.so must resolve to libmvec.so.1 and libmvec.a must be there
  (next's dev-link and archive loops; before, libmvec.so was only allowed by the layout check).
* 6414b1292 Three packaging plants (handoff/SYS-LIBMVEC-plants.txt).

Jobs on mei, all on the code of 6414b1292 (later commits touch handoff/ only):
* `sys-libmvec-3-lint0` rc 0: step 0, `cargo xtask clippy` + `cargo xtask static` on the inherited head 45db63021.
* `sys-libmvec-3-lint` rc 0: `cargo clippy --workspace --all-targets --all-features -D warnings`,
  `cargo xtask clippy`, `cargo xtask static`.
* `sys-libmvec-3-pkg2` rc 0 (~/work/SYS-LIBMVEC-3/pkg.sh; both flavours from the role tree, NIJI_DEB_COMMIT
  6414b1292): check-system PASS (6 packages, 2.39+niji20261117.1-1); check-symbols PASS (4432 lines, 13
  libraries; 216 `_ZGV` lines, shlibs `libmvec 1 libc6 (>= 2.39)`); root-system pass=19 fail=0; coexist
  check.sh PASS (2 packages); libmvec.so.1 66872 bytes in libc6 and niji6, libmvec.a 86404 bytes and
  `libmvec.so -> libmvec.so.1` in libc6-dev and niji6-dev.
  Parity (offline: `parity.py inventory` + `compare` against the committed reference inventory, as
  SYS-DEV-LINKS-2 did): **audit row 2 closed**: the a rows libmvec.so.1, libmvec.a, libmvec.so, the b row
  libmvec.so.1 and the e row `symbols: libmvec.so.1 libc6 #MINVER#` are gone; left only
  `e shlibs: libmvec 1 libc6` (2.41 reference / 2.39 ours, the version shape every library's shlibs line has).
  Other new rows are next's (libresolv class c, libBrokenLocale/libresolv shlibs, symbols line count).
  Swapped trixie root (SWAP_EXTRA_PKGS=direwolf,cif2hkl): mvec-ldd, mvec-direwolf-gen,
  mvec-direwolf-decode, mvec-cif2hkl-powder same output and status; mvec-cif2hkl-xtal same status, 16
  |F|^2 values differ in the 7th digit (per-lane scalar results; D-10.124's differences row). RESULT
  same=60 differ=8 (the other 7: ldd-1..7, listing directory /lib vs /usr/lib, as in run 2). The script
  fails if a libmvec row other than xtal differs or a libmvec a/b gap row remains.
* `sys-libmvec-3-plants` rc 0: 8 plants, 8 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG (the 5 generator
  plants of run 2 and mvec-pkg-lib, mvec-pkg-link, mvec-pkg-archive, each by its check-system FAIL line).
* `sys-libmvec-3-gate`: `cargo xtask affected --base origin/next --run`; the xtask change escalates it to
  the workspace clippy + `cargo xtask test` (run 2's equal gate took 110 min: 12059 passed, 0 failed).
  Running at hand-back; the dispatcher reads it.
* Superseded: `sys-libmvec-3-pkg` (started with a placeholder NIJI_DEB_COMMIT, stopped with kill-label).
* Start-up checks not run: the change touches packaging and the libmvec generator, no start-up code.

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
  (Run 3 ran the offline inventory + compare step instead: row 2 closed, see above.)

## RULING NEEDED / INFORMATIONAL
* Pending record `records/pending/SYS-LIBMVEC-2-1.toml` (D-NEW, D-10): CI assigns the id.
* INFORMATIONAL: swap-system's ldd-1..7 differ on a trixie root (path prefix of the listing), independent of libmvec.
