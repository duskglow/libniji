# SYS-LIBMVEC: handoff (plan/package-parity-audit.md row 2; ledger M-0/M-1)

**State.** Branch `task/sys-libmvec-3` (SYS-LIBMVEC-3, continuing `task/sys-libmvec-2`), merged with
`origin/next` 2da237f41 (after SYS-DEV-LINKS-2). HEAD in handoff/reports/SYS-LIBMVEC-3.json. Role mei:~/w/s1.
Full report: handoff/SYS-LIBMVEC-report.md.

## Decisions in force
* D-10.124 (records/decisions/D-10/D-10.124.toml, id assigned by the coordinator): per-lane scalar method,
  accuracy = the scalar bound, errno/flags per lane, no compat option; row in docs/differences-from-glibc.md.

## Done
* Run 1 (SYS-LIBMVEC): `libmvec.so.1` (216 exports, 2 nodes) and `libmvec.a` generated from
  `spec/abi/versions.tsv` by `xtask/src/libmvec.rs`; `crates/core/tests/libmvec_products.rs` +
  `c/libmvec_lanes.c` (all 1512 lanes bit-exact vs libniji's scalar libm, shared and static).
* Run 2 (SYS-LIBMVEC-2): packaging (libc6/niji6 `libmvec.so.1`; libc6-dev/niji6-dev `libmvec.a`,
  `libmvec.so -> libmvec.so.1`), the record and differences row, swap-system.sh `SWAP_EXTRA_PKGS` /
  `SWAP_EXTRA_CMDS` and `tools/deb/swap-extra-libmvec.sh` (direwolf, cif2hkl).
* Run 3 (SYS-LIBMVEC-3): merge of origin/next (9d62fdff5; CI conflict in system.sh, check-system.sh,
  swap-system.sh resolved keeping both sides: next's libresolv.so.2 / libBrokenLocale.so.1, dev links, empty
  archives and gawk; this branch's libmvec entries and the `$include` variable). check-system.sh now also
  checks that libmvec.so resolves to libmvec.so.1 and libmvec.a is shipped (35250c7134), with three packaging
  plants (6414b1292). Results: handoff/SYS-LIBMVEC-report.md.

## Open / next
* Faster vector kernels: later work with their own measurements (each lane costs one scalar call today).
* The network part of parity.sh (index fetch, 4000-package scan) only changes with the reference; the offline
  inventory + compare was rerun on the merged packages (report).

## Not to redo
* No glibc/libm source read; ABI layout from GCC's clones and the black-box reference run.
* Do not regenerate versions.tsv wholesale (drops the R3 rows).
* The merge: keep `include=` a variable in swap-system.sh (SWAP_EXTRA_PKGS extends it).
