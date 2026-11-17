# SYS-COMPAT-VERSIONS: libm's old versions (issue #7) and the multi-version audit

**State:** continued by SYS-COMPAT-VERSIONS-2 on branch `task/sys-compat-versions-2` (stacked on
`task/sys-compat-versions`, from origin/next `28740dd5e`); last code commit `da477a66d`
(handoff/reports/SYS-COMPAT-VERSIONS-2.json names the final head). Role keke `~/w/n1`. Plants
`handoff/SYS-COMPAT-VERSIONS-plants.txt` (4). Audit data `handoff/SYS-COMPAT-VERSIONS-audit.tsv`. No new
decision row: the plan rows' own note ("alias if LG-0b shows identical results", D-10.2) decided it; the
probe is the evidence.

## SYS-COMPAT-VERSIONS-2 (fix of the red gate)

* Cause of `sys-compat-versions-gate` rc 1: `xtask compat::tests::the_real_tables_give_the_libm_scope`
  asserted 110 libm aliases and that no alias targets its own name; the 7 new aliases (each jumps to its
  own default) broke both (the second assert was hidden behind the first). Fixed in `da477a66d`: the test
  now lists every (name, version, target) outside `__*_finite` (pow10/pow10f/pow10l renamed to exp10*, and
  exp, exp10, fmod, fmodf, hypot, hypotf, pow at GLIBC_2.2.5 to themselves), so a scope change names its
  pairs; total 107 + 3 + 7 = 117. It also checks the rendered fmod stub and `.symver`.
* New plant `hypotf-old-to-hypot-scope` (retargets hypotf's alias; the scope test must fail, no build).
* `sys-compat-versions-2-lint` rc 0: `cargo xtask clippy`, `cargo xtask static`, the 11 compat tests.
* Final head: full gate `sys-compat-versions-2-gate` (`cargo xtask test`) and plants
  `sys-compat-versions-2-plants` (all 4) on keke; results in the hand-back and the report json.

## Done

* `fmod`, `fmodf`, `exp`, `exp10`, `pow`, `hypot`, `hypotf` at `GLIBC_2.2.5`: `alias` rows in
  `spec/abi/compat-plan.tsv` (were `behaviour`), off `compat-missing-libm.tsv`, stubs regenerated
  (`cargo xtask compat-gen`, `crates/libm-so/compat_aliases.S`). versions.tsv already had the rows.
* Probe record `spec/behaviour/libm-compat.md` + `.glibc.txt`: all 19 old libm versions against
  their defaults on the reference. Identical (bits, errno, invalid/divbyzero/overflow/underflow,
  signgam) for 15. `log`, `log2`, `logf`, `log2f` differ: a negative argument gives +NaN (old) vs -NaN
  (default), same errno and flags. A `matherr` defined in the program is never called.
* Test `old_libm_versions_bind_and_match_the_reference` (crates/core/tests/crt_products.rs, oracle
  `crates/core/tests/oracle/libm_compat_versions.c`): binds each old version with `.symver`, links
  by soname against libniji's products (LD_LIBRARY_PATH set by `env` after `timeout`, so the host's
  `timeout` never loads libniji's libc), compares 2004 cases with the reference.
* `tools/deb/swap-system.sh`/`swap-cmds.sh`: gawk added to the swapped root (gawk-1 `7.5 % 2`, gawk-2).

## Results (keke)

* `sys-compat-versions-verify4` rc 0: test ok; Debian trixie's gawk (debs SHA256-checked against the
  gpgv-verified index) on libniji's ld.so + libc + libm prints `1.5`, as on the reference; `cargo xtask
  clippy` and `cargo xtask static` clean.
* `sys-compat-versions-scan` rc 0: 22834 trixie main amd64 packages depending on libc6, 0 failed.
* Gate `sys-compat-versions-gate` (direct tier) and plants `sys-compat-versions-plants-2` (3 plants:
  a wrong `target=` for fmod, pow, exp10; each regenerates the stubs, rebuilds, runs the test) run
  on the final code and were left running at hand-back (PENDING JOBS). plants-2 reads the plants
  file from `~/work/SYS-COMPAT-VERSIONS/plants.txt` (same text as the committed file): the role was
  busy with the gate, so the commit that changed only this handoff file was not synced.
  `sys-compat-versions-plants` (rc 3) is superseded: the tool refuses .S plants not anchored on a
  `niji_machine_` label, so the plants moved to the plan rows.
* NOT VERIFIED: the full swapped-root run (`swap-system.sh trixie`, now 54 commands) and the
  empty-root form (needs `tools/deb/build.sh --flavour system` from a git checkout); the direct run of
  Debian's gawk above is the evidence instead.
* Superseded: probe-1 (scratch probe), gen, so, t1-t4 (test iterations), verify (C guard bug),
  verify2/verify3 (cross-library differences, then fixed).

## Audit (ranked; user counts = trixie packages / ELF files binding the pair)

Reference multi-version symbols: libc 224 (467 pairs), libm 35 (70), libpthread 1 (12), libdl 1 (3),
librt 1 (5); ld.so, libresolv, libutil, libanl, libnsl none. libniji lacked 23 libc + 33 libm pairs;
libpthread/libdl/librt none. With users, before this branch: fmod 101/142 and fmodf 26/31 (now
exported), pow 8/12, exp 7/13, exp10 4/10, hypot 3/3, hypotf 2/2 (now exported), and:

1. `log@GLIBC_2.2.5` 6 pkgs / 12 files, `log2@GLIBC_2.2.5` 1/2: old behaviour = default except
   +NaN for negative input. Story: libm generator support for `behaviour` rows (a
   `niji_compat_<name>` like libc's) + the two wrappers (plus logf/log2f, no users).
2. `pthread_sigmask@GLIBC_2.2.5` (needed from libpthread.so.0) 3 pkgs: btscanner, pbzip2, wavemon.
   Plan row `behaviour` LG-2 "LG-0b probe": probe old vs new, alias if identical.
3. `dlmopen@GLIBC_2.34` (a DEFAULT version) 3 pkgs: libdlrestrictions1, libfcitx5utils2, libioth0.
   libniji has no dlmopen (compat.rs DEFERRED).
4. No users: the other 21 libc pairs (timer_*, nftw, posix_spawn, regexec, quick_exit,
   sched_/pthread_*affinity, lio_listio, pthread_getattr_np) and libm exp2/exp2f/expf/exp10f/powf/
   lgamma*/logf/log2f (all probe-identical: aliases when a program needs them), totalorder* (2.25-2.27,
   value arguments).

Beyond multi-version pairs (audit tsv part 2): **libmvec.so.1 is not shipped** (49 packages bind
`_ZGVbN2v_sin@GLIBC_2.22`, ~40 symbols with users), and `_r_debug@GLIBC_2.2.5` from
ld-linux-x86-64.so.2 has 5 packages. Both are bigger than this story: recommend their own stories.

## Found: libniji's default pow differs from the reference (not from this change)

The new test compares libniji's pow with the reference's on 484 cases; 8 differ, each also in the
default version (both libraries' old pow = own default). (a) **pow(-1, +-1e308) = 0 with ERANGE in
libniji; the correct result is 1** (1e308 is an even integer). (b) pow(sNaN, +-0) and pow(1, sNaN):
libniji 1 + FE_INVALID, reference qNaN + FE_INVALID. (c) exact subnormal results: the reference
also raises FE_UNDERFLOW. The test lists the 8 cases (DEFAULT_POW) and still requires old = default
there. Recommend a pow story for (a), a ruling for (b); (c) libniji follows IEEE 754 7.5. When (a)
or (b) is fixed, drop the entries from DEFAULT_POW.

## Next step

Coordinator: cut stories for audit items 1-3, libmvec and `_r_debug`. The pow defects (a) and (b) are
ruled (#6108: sNaN -> qNaN + FE_INVALID; #6109) and go to MATH-POW-EDGES, which drops the fixed cases
from DEFAULT_POW in crt_products.rs. Run the swap
smoke with this branch at the next snapshot (expect 54/54). Rule slip: one Bash heredoc used to append
the test (no backslashes in the content; the file is correct).
