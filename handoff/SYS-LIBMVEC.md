# SYS-LIBMVEC: handoff (plan/package-parity-audit.md row 2; ledger M-0/M-1)

**State.** Branch `task/sys-libmvec` (from `origin/next` cc18a11ef), HEAD in handoff/reports/SYS-LIBMVEC.json.
Role mei:~/w/s2. Run 1 stopped at the context checkpoint (~150k): core library built and proved, rest not started.
Verdict: INCOMPLETE (WIP commit; do not land as it stands).

## Done (commit 28a881e70, WIP)
* `spec/abi/versions.tsv`: + the reference's 216 `libmvec.so.1` rows (48 at GLIBC_2.22, 168 at GLIBC_2.35, all
  `default` FUNC). `spec/tools/abi_extract.py`: `libmvec.so.1` in `LIBS`, new `--versions-only` mode. A full
  `--versions-only` run on the reference system (Ubuntu glibc 2.39) gave 4816 rows = committed 4600 + 216, but the
  committed table also has two hand-added `sched_getattr`/`sched_setattr@GLIBC_2.41` rows (R3), which a full regen
  drops, so only the 216 were spliced in after libm.so.6. PROVENANCE and the spec's Appendix C row count (4818) updated.
* `xtask/src/libmvec.rs` (new; called from `so.rs` right after `run_libm`): per scalar function one wrapper
  `__niji_mvec_<f>` calling libniji's libm `f`, declared `__attribute__((simd("notinbranch")))`. The compiler emits
  the four unmasked vector-ABI clones (b SSE, c AVX, d AVX2, e AVX-512) with the table's lane counts (probe: GCC 13's
  clone names equal the reference's 216 names exactly, incl. `bN4vvv`/`cN8vvv`/`eN16vvv_sincosf`); a `.symver` per
  export names the clone (a wrong lane count would name an undefined clone: the build fails). Element type =
  register width / lanes (64 double, 32 float); shapes `v`, `vv`, `vvv` (sincos only); anything else is an error.
  Version script names every export under its node with `local: *` in the first (a bare `local: *` hid the
  `.symver` names too: first build exported nothing). Checked by `placeholders::product_errors` + DT_NEEDED libm.so.6;
  `placeholders::check_no_private` now includes libmvec.so.1. `libmvec.a`: same wrappers, exports as global
  `.set` aliases. `libmvec.so` -> `libmvec.so.1` link in target/niji.
* `crates/core/tests/libmvec_products.rs` + `crates/core/tests/c/libmvec_lanes.c`: calls all 216 exports directly
  through the ABI (vector types; `vvv` pointer vectors split by the ISA's integer register width: 128 bits for b and c,
  256 for d, 512 for e, as GCC's clones read them) and compares each lane with the scalar call.
  `exact` mode against libniji, dynamic and static; `ulps 4` against the host's reference library (black-box proof
  that the program's register layout is the reference's). CPU without an ISA class: skipped and named.

## Results (mei, AVX-512 host)
* `sys-libmvec-so3`: xtask unit tests 5/5; `cargo xtask so` printed `so: libmvec.so.1 (2 nodes, 216 exports, checked
  against spec/abi/versions.tsv), libmvec.a, libmvec.so` (that job's rc 101 is the wrong `-p` name in its last step).
* `sys-libmvec-test3` rc 0: 3/3; `lanes: checked 1512, 0 differ (bit-exact)` (shared and static),
  `lanes: checked 1512, 0 differ (ulps bound)` against the reference.
* Superseded: sys-libmvec-so1 (local:* hid exports), sys-libmvec-so2 (`\x40f` hex escape in the archive source),
  sys-libmvec-test1/-test2 (test package name; C macro `f##f` pasted the parameter twice).

## Remaining (exact next steps)
1. Packaging: `tools/deb/build.sh` (line ~177 so list: + libmvec.so.1 in libc6; ~200 archives: + libmvec.a; dev
   link libmvec.so in libc6-dev) and `tools/deb/system.sh` (lines 43, 76). Check `tools/deb/symbols.py` produces a
   libmvec.so.1 symbols/shlibs entry (it filters versions.tsv by library). libm.so stays a link (R4: no `simd`
   declarations in libniji headers, so no `AS_NEEDED` linker script needed).
2. Decision record: `cargo xtask records new decision D-NEW --series D-10 --story SYS-LIBMVEC` (method: compiler
   simd clones over scalar libm; each lane bit-identical to libm, tighter than the vector variants' documented 4-ulp
   bound, slower than vector kernels; scalar errno/flags behaviour carried per lane). Differences row in
   docs/differences-from-glibc.md (neutral wording) with "no compat option" reason: results are within the
   reference's documented bound, a compat flag would need the reference's kernels. RULING NEEDED if the coordinator
   wants errno-setting per lane treated differently.
3. Plants (`cargo xtask plants`, file handoff/SYS-LIBMVEC-plants.txt), e.g. in xtask/src/libmvec.rs: shape `vv`
   body passes `(y, x)` (msg: DIFF|lanes_equal.*FAILED); `"float"` <-> `"double"` mapping swapped (build error);
   first node without `local: *` (unversioned wrapper export: product check); a scalar name mapped wrong (e.g. strip
   the trailing `f`). In the C test, nothing (it is the oracle).
4. `cargo xtask clippy`, `cargo xtask static`, then the gate (xtask change escalates to FULL: leave it running and name it).
5. Both package flavours build; `tools/deb/parity.sh` rerun (row 2 gone, no new rows); two sampled Debian binaries
   (e.g. cif2hkl, raster3d `render`; or xfoil with a stdin script) run in the swapped root (`tools/deb/swap-system.sh`
   would need an extra-package hook: not present yet).

## Not to redo
* No glibc/libm source read; the ABI layout comes from GCC's own clones (our build) and the black-box reference run.
* Do not regenerate versions.tsv wholesale (drops the R3 rows).
