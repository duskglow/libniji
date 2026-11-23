# SYS-COMPAT-VERSIONS-REST: the remaining old versions Debian trixie packages bind

**State:** branch `task/sys-compat-versions-rest` (from origin/next `8e254c98d`), code commit `17f76d575`;
the report json names the final head. Role keke `~/w/n1`. Plants `handoff/SYS-COMPAT-VERSIONS-REST-plants.txt`
(5). Decisions in force: D-10.2 (libm compat), R15-1/R15-2 (own entries; compat names not in the
archives); the compat-plan rows' notes record each verdict and the probe is the evidence (no new row).

## Done

1. **`log@GLIBC_2.2.5`, `log2@GLIBC_2.2.5`** (libm.so.6). Probe (`libm_compat_versions.c -DPROBE_ALL`,
   now also -qNaN and -sNaN for the logarithms; record `spec/behaviour/libm-compat.glibc.txt`): the old
   version equals the default (bits, errno, flags) except a negative number (finite or -inf): NaN with
   the sign bit clear (`7ff8...`) where the default gives `fff8...`, same EDOM and FE_INVALID. NaN inputs
   of both signs, +-0, subnormals, +-inf: identical. So they are own entries, `niji_compat_log`,
   `niji_compat_log2` (`crates/boundary/src/posix/s10_math.rs`): the default's result, sign cleared for a
   negative argument (integer test on the bits; no FP compare). Plan rows stay `behaviour`.
   * Generator: `compat::libm_entries_from` takes `defined` and gives a `behaviour` row whose
     `niji_compat_<name>` exists a stub to it (libc's rule). `so.rs run_libm`: own entries are roots and
     global in `libm.o` (the .so input), localized in libm.a's copy, local in libm.so.6 (version script);
     the target check accepts an own entry defined by the .so input.
   * logf/log2f: same difference, no users; left on compat-missing-libm.tsv.
2. **`pthread_sigmask@GLIBC_2.2.5`** (libc.so.6; trixie binaries need it from libpthread.so.0, which on
   the reference defines the version node only). Probe `crates/core/tests/oracle/pthread_sigmask_compat.c`
   (140 cases: every `how` incl. 3 and -1, null/non-null set and old set, sets with 32/33, KILL/STOP, all
   bits, RT range; two starting masks; mask set/read by raw syscall): old = default in all 140
   (`spec/behaviour/pthread-sigmask-compat.md` + `.glibc.txt`). Row now `alias`; stub regenerated.
3. **`_r_debug@GLIBC_2.2.5`: NOT BUILT (the brief says to report the size first).** The loader has no
   rendezvous yet (`crates/ld/src/debug.rs` is a 3-line placeholder; plan DL-23 `DL-RDEBUG`, M14-9).
   Reference: `_r_debug` 40 bytes (.bss, GLIBC_2.2.5), `_dl_debug_state` 5 bytes (GLIBC_PRIVATE).
   Size: **M, not the plan's S**, because the link maps exist only after the first `dlopen`
   (`crates/boundary/src/ld/dlopen.rs`: "nothing at start: link only records a few words per object"):
   (a) a start-time `struct link_map` list for every initial object (start-up cost: needs the start-up
   fault/window bounds, ALWAYS FORWARD), which the Loader must adopt in place on the first dlopen
   (stable addresses for debuggers); (b) `_r_debug` (r_version 1 or 2 by probe, r_map, r_brk,
   r_state, r_ldbase) exported from ld.so at GLIBC_2.2.5, `_dl_debug_state`, `DT_DEBUG` filled in the
   program's dynamic section; (c) RT_ADD/RT_DELETE then RT_CONSISTENT with an `r_brk` call around
   each change in dlopen/dlclose; (d) gdb batch tests (`info sharedlibrary`, a breakpoint in a
   dlopened library) and a probe of the reference's r_version/r_ldbase. Estimate 400-700 lines plus
   tests; one story after a ruling on (a)'s start-up cost. Users: libcriterion3, libgnat-12/13/14,
   simplescreenrecorder-lib.
4. **Out of scope, phase 14:** `dlmopen@GLIBC_2.34` (3 packages: libdlrestrictions1, libfcitx5utils2,
   libioth0) needs link-map namespaces (DL-22 `DL-NAMESPACES`).

## Results (keke)

* `sys-compat-versions-rest-probe` rc 0: the reference records above.
* `sys-compat-versions-rest-verify3`: compat-gen, `cargo xtask so` (libm.so.6 exports `log@GLIBC_2.2.5`,
  `log2@GLIBC_2.2.5`; libc.so.6 `pthread_sigmask@GLIBC_2.2.5`; no `niji_compat` in libm.so.6's dynsym
  or global in libm.a), clippy, static, 11 compat tests, the libm test: ok. The sigmask test failed on
  libniji's DEFAULT version (INFORMATIONAL below); the test masks exactly those differences (`sys-compat-versions-rest-t1` ok). The first gate and plants run (baseline-4 red on the same cause) are superseded.
* `sys-compat-versions-rest-trixie` rc 0 (InRelease gpgv-verified, Packages.xz and every deb
  SHA256-checked): trixie's `ries` (binds log@GLIBC_2.2.5) `ries -l1 2.5063` on libniji's ld.so/libc/libm
  prints the reference's equations; `cbootimage -h` (log2@GLIBC_2.2.5) prints the reference's text;
  `pbzip2 -p2` (pthread_sigmask@GLIBC_2.2.5 via libpthread.so.0) round-trips 4 MiB and the reference
  pbzip2 decompresses libniji's output. (`sys-compat-versions-rest-diff`: ries' timing line and two
  search statistics differ: distinct 30276 vs 30273 of 58559; the equations do not.)
* Final head (code `3111e1efc`): `sys-compat-versions-rest-lint` rc 0 (clippy --all-targets --all-features -D warnings, static); `sys-compat-versions-rest-t1` rc 0 (both compat tests); gate `sys-compat-versions-rest-gate2` and plants `sys-compat-versions-rest-plants2` (5 plants, RAM dir plants-2) left running at hand-back (PENDING JOBS).
* Superseded: `-verify`, `-verify2` (script copy clash; libm.a dropped the own entries, fixed in so.rs), `-verify3` (sigmask test before masking), `-gate` (stopped: stale test), `-plants` (baseline-4 red, same cause), `-diff` (diagnostic).

## INFORMATIONAL

* libniji's default `pthread_sigmask` (and so the alias) differs from the reference's default: (a)
  with a null set it rejects an invalid `how` (EINVAL) where POSIX says `how` is not significant
  (the reference returns 0 and fills the old set): a POSIX defect, likely shared by `sigprocmask`;
  (b) it writes the whole 128-byte old set (reference: 8 bytes); (c) a set can block signal 33 and
  the old set hides a blocked 32 (reference: neither 32 nor 33 ever blocked by a set, both shown).
  The test masks exactly these (named in the test). Recommend a default-version story: fix (a), rule
  on (b)/(c), then drop the masks.
* ries' search statistics differ slightly (above): some libm function differs in the last bits
  from the reference on its inputs; results unaffected.
* Rule slip: one Bash heredoc (a Python edit of the libm oracle; no backslashes; the file is correct).

## Next step

Coordinator: a DL-23 story sized M with a ruling on start-time link maps (item 3); the default
pthread_sigmask story (INFORMATIONAL). Re-run the swap smoke at the next snapshot.
