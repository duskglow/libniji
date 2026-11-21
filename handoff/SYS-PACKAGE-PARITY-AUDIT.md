# SYS-PACKAGE-PARITY-AUDIT: package parity audit (planning; fixes nothing)

**State:** branch `task/sys-package-parity-audit`, cut from origin/next `28740dd5e`. Result: `plan/package-parity-audit.md`
(26 ranked gaps). Data: `results/package-parity/` (inventories, gaps, consumer counts). Script: `tools/deb/parity.py`, driver
`tools/deb/parity.sh WORK OURS-DEBS OUT [SAMPLE]`. Proposed briefs: `handoff/SYS-PACKAGE-PARITY-AUDIT/*.md` (6).

## Counts

* Reference: Debian trixie glibc 2.41-12+deb13u4, 16 binary packages: 10563 files, 860 ELF objects, 5148 x86-64 dynamic
  symbol rows. Ours (system flavour from next 28740dd5e, 2.39+niji20261116.1-1): 6 packages, 93 files, 22 ELF, 4216 rows.
* Gaps (`gaps.tsv`, 3028 rows): (a) files 1606; (b) sonames 579 (10 x86-64 libraries, 6 gconv helpers, rest multilib);
  (c) pairs 627 (308 GLIBC_PRIVATE, 96 after 2.39, 58 multi-version, 125 other); (d) packages 10; (e) control 268.
* Consumer scan: 4000 of 23193 trixie amd64 packages that depend on libc6 (random, seed 20261116), 4000 scanned, 0 failed.
  Users: libm multi-version pairs (fmod@GLIBC_2.2.5 25), libmvec.so.1 18, libresolv.so.2 13, libthread_db.so.1 1; every other
  soname or pair gap 0. Exact relations: locales-all Build-Depends 108 sources / Depends 98; locales Depends 114 / B-D 65;
  libc6-dbg Depends 1 / B-D 4; libc-dev-bin B-D 5; multilib libc6-i386 Depends 32, libc6-x32 19.
* Run twice (jobs `-ref-inv`/`-rank` and `-run2`): every inventory and gaps.tsv byte-identical; only the import counts moved
  (the first run lost one package, docker.io, to a decoding error that is now fixed).

## Jobs (keke)

`sys-package-parity-audit-ours-build` (build.sh --flavour system, rc 0), `sys-package-parity-audit-run2` (the committed
driver, fresh work dir, rc 0: the data committed here). Earlier: `-ref-inv`, `-rank`, `-ours-inv` (rc 0, superseded by run2).

## Findings for the coordinator

* SL-SONAMES-2 landed, but the system flavour ships no `libBrokenLocale.so.1` (folded into SYS-DEV-LINKS).
* No Debian keyring on the build host: indices are SHA256-checked against InRelease, the signature is not checked.
* Not counted: maintainer-script use of libc-bin's programs (bodies not read); transitive users beyond one Depends level.

## Next step

Cut, in order: SYS-CONTROL-PARITY (S), SYS-LIBMVEC (M; or re-prioritise ledger M-0/M-1), SYS-DEV-LINKS (S),
SYS-DBG-PACKAGE (S), SYS-THREAD-DB (M), SYS-LOCALES-ALL (M, after SL-LOCALEDEF and R5). Rerun the audit after landings
(`tools/deb/parity.sh`, about 10 minutes, network to deb.debian.org).
