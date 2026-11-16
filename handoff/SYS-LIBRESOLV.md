# SYS-LIBRESOLV (issue #5): libresolv.so.2 in the system packages

State: branch `task/sys-libresolv-2`, head 84dc17f94 (WIP), SYS-LIBRESOLV-2 stopped at the ~150k context
checkpoint. Build role keke ~/w/n2; job scripts in keke ~/work/SYS-LIBRESOLV-2 (deb.sh, lintbase.sh,
probe.sh + rrneg.c, fulldiff.sh).

## Decisions in force

* D-16.6 (ruling #6096): libresolv.a stays empty, libc.a defines every libresolv function;
  check-system.sh proves a static `-lresolv` link runs (done, green in deb1).
* No new decision record: every behaviour change below matches the reference (probe-settled).

## Done (do not redo)

* Run 1: system.sh ships libresolv.so.2 (libc6) and libresolv.so (libc6-dev); check-system.sh,
  check-symbols.sh (sample sym-resolv), smoke/resolv.c, smoke/sym-resolv.c; differential probe
  `crates/core/tests/oracle/ns_parse_diff.c` + `tests/ns_parse_products.rs`.
* Run 2:
  - ns_parse_products ran the host `timeout` with LD_LIBRARY_PATH = products (host loader aborted on
    libniji's libc.so.6); now `env` sets it for the probe only.
  - Probe-settled reference behaviour, now in `crates/boundary/src/resolv.rs`:
    ns_parserr rrnum -1 = the handle's next record in that section; a call naming a valid section other
    than the handle's current one moves the handle to that section's record 0 even when it fails;
    failed calls in the same section or with an invalid section leave the handle; other negative rrnum
    ENODEV. Every failure of dn_expand / dn_skipname / ns_name_unpack / ns_name_uncompress sets errno
    EMSGSIZE. ns_name_uncompress renders the root as "." (dn_expand gives "").
  - Corpus: rrnum -1 twice, -2, qd -1 twice per message.
  - root-system.sh: `resolv` (plain -lresolv) runs in the empty root; `krb5` (smoke/krb5.c, linked
    -lgssapi_krb5 -lkrb5 against the build system's Kerberos libs copied into the root) - ld.so --list
    finds the root's libresolv.so.2 and it runs with LD_BIND_NOW (PASS in deb1).
  - deb_symbols_products.rs: libc6's symbols file must have a libresolv.so.2 block with
    ns_initparse/ns_parserr@GLIBC_2.9 (gate-time guard for issue #5).
  - Plants file `handoff/SYS-LIBRESOLV-2-plants.txt` (6 plants), NOT run yet.

## Results so far (keke)

* lint0 (clippy + static, inherited head): rc 0.
* deb1 (head 5e55fd75f; killed during the coexist build after the system steps): build-system 0,
  check-system 0, check-symbols 0 (sym-resolv -> libc6 (>= 2.9)), root-system 17 pass / 1 fail (the
  ld.so --list pattern, fixed in 84dc17f94: the loader prints /lib/... through merged /usr),
  swap trixie same=54 differ=7 (ldd-1..7: libc.so.6/libm.so.6 shown as /usr/lib/... vs the stock
  root's /lib/...; ldd-6 -v version lines; no libresolv line in any diff), lint errors 7 / warnings 51.
* lintbase (origin/next 28740dd5e system packages): lint errors 7 / warnings 50. The one new warning:
  `W: libc6: undeclared-elf-prerequisites (libc.so.6) [usr/lib/x86_64-linux-gnu/libresolv.so.2]`.
* ns3: the differential test still differed (errno, root ".", handle move) -> fixed in 84dc17f94.
* ns4 (head 84dc17f94: clippy, static, so, ns_parse_products + deb_symbols_products, fulldiff.sh):
  started detached at hand-back, result unknown.

## Exact next steps

1. `~/bin/jobwait sys-libresolv-2-ns4` on keke; read ~/work/SYS-LIBRESOLV-2/diff.txt if it differs.
2. Lintian warning: find why lintian flags libresolv.so.2 but not libpthread.so.0 etc. (all have
   DT_NEEDED libc.so.6 per xtask/src/placeholders.rs check). Compare `readelf -dW` of libresolv.so.2
   and libpthread.so.0 in the packages; fix in xtask/src/placeholders.rs (FORWARDED link) or the
   packaging so lint is no worse (7/50).
3. Swap ldd-1..7: run swap-system.sh on the base packages (keke ~/work/SYS-LIBRESOLV-2/lintbase/sys
   is gone? lintbase.sh deleted only sys-work; the .debs are in lintbase/sys) to show the 7 ldd
   differences exist on origin/next too (expected: the branch changes no loader/ldconfig code).
   If they do, INFORMATIONAL, not this story.
4. Rerun deb.sh (new label, head NIJI_DEB_COMMIT/SOURCE_DATE_EPOCH updated) for both flavours.
5. Plants: `cargo xtask plants handoff/SYS-LIBRESOLV-2-plants.txt --dir /dev/shm/claude/SYS-LIBRESOLV-3/plants-1 --timeout 600`
   (keke is Zen 1: /dev/shm). Gate: `cargo xtask affected --run --base origin/next`.
6. Drop the WIP subjects only by new commits (no rewrite); handoff, report json, handback-check.

## Open questions

* INFORMATIONAL: the system flavour lacks libBrokenLocale.so.1/.so/.a, libg.a, libmcheck.a,
  libpthread_nonshared.a, which the coexist flavour ships (not this story).
