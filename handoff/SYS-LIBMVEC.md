# SYS-LIBMVEC: handoff (plan/package-parity-audit.md row 2; ledger M-0/M-1)

**State.** Branch `task/sys-libmvec-2` (SYS-LIBMVEC-2, continuing `task/sys-libmvec`; base `origin/next` cc18a11ef),
HEAD in handoff/reports/SYS-LIBMVEC-2.json. Role keke:~/w/n2. Full report: handoff/SYS-LIBMVEC-report.md.

## Done
* Run 1 (SYS-LIBMVEC, 28a881e70): `libmvec.so.1` (216 exports, 2 nodes) and `libmvec.a` generated from
  `spec/abi/versions.tsv` by `xtask/src/libmvec.rs` (compiler `simd("notinbranch")` clones of one wrapper per
  scalar libm function, `.symver` per row); `crates/core/tests/libmvec_products.rs` + `c/libmvec_lanes.c` (all
  1512 lanes bit-exact vs libniji's scalar libm, shared and static; within 4 ulps against the reference).
* Run 2 (SYS-LIBMVEC-2):
  * Packaging (57abcb1c4): `libmvec.so.1` in niji6 / libc6, `libmvec.a` and the `libmvec.so -> libmvec.so.1`
    link in niji6-dev / libc6-dev (ref-files.tsv 697, 1643, 1981). `check-system.sh` allows the link and checks
    the soname. `libm.so` stays a plain link (R4). `symbols.py` needed no change (it reads every library in the
    directory against versions.tsv).
  * Records: `records/pending/SYS-LIBMVEC-2-1.toml` (D-NEW, series D-10: per-lane scalar method, accuracy = the
    scalar bound, errno/flags per lane, no compat option) and a row in docs/differences-from-glibc.md (math
    section, "The vector variants").
  * Black-box probe (reference, keke): `_ZGVbN2v_log({0,1})`, `log({-1,1})`, `exp({1000,1})`, `_ZGVbN4v_logf`
    set errno (ERANGE/EDOM/ERANGE/ERANGE) and the matching flag, as libniji's per-lane scalar calls do: no difference.
  * Plants: handoff/SYS-LIBMVEC-plants.txt (5 plants on the generator).
  * Swap tooling (6da8ce5c6): `swap-system.sh` takes `SWAP_EXTRA_PKGS` / `SWAP_EXTRA_CMDS`; `swap-cmds.sh` sources
    the extra file; `tools/deb/swap-extra-libmvec.sh` runs direwolf (gen_packets + atest) and cif2hkl (rock salt).

## Results
See handoff/SYS-LIBMVEC-report.md (gate, plants, package builds, swapped-root run).

## Open / next
* parity.sh rerun (row 2 gone) not done: it re-downloads the indices and scans 4000 packages (network, ~hours);
  libmvec's three files are now in the packages, so its file and soname rows can no longer be reported missing.
* Faster vector kernels: later work with their own measurements (each lane costs one scalar call today).

## Not to redo
* No glibc/libm source read; ABI layout from GCC's clones and the black-box reference run.
* Do not regenerate versions.tsv wholesale (drops the R3 rows).
