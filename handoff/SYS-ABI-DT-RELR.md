# SYS-ABI-DT-RELR: libc.so.6 defines GLIBC_ABI_DT_RELR; the ABI tables record symbol-less nodes

**State:** branch `task/sys-abi-dt-relr`, cut from origin/next 752955498. Role mei:~/w/s4.

## What changed

1. **Count** (`tools/deb/parity.py`): `rank` writes `consumers-verneeds.tsv`, the GLIBC-family version
   requirements (VERNEED) a package's ELF files record with no symbol imported at them; `parity.py verneeds DEB...`
   reports the same for local .debs. Rerun on fresh trixie indices (2026-11-17; job `sys-abi-dt-relr-scan`, mei,
   ~7 min, network): 4000 of 23193 sampled packages, 0 failed. Committed: `results/package-parity/consumers-verneeds.tsv`
   and `ref-verneeds.tsv` (the other tables there are from the audit's 11-16 indices, unchanged).
   * `libc.so.6 GLIBC_ABI_DT_RELR`: **1** sampled package (thunderbird). Reference's own packages (left out of the
     sample): libc-bin 9 programs (getconf, getent, iconv, locale, localedef, pldd, zdump, iconvconfig, zic),
     libc-dev-bin gencat, libc-devtools 2, libc6-dev sotruss-lib.so, nscd, libc6's 270 gconv modules.
   * Numbered nodes required with no symbol (libniji defines them as ABI levels): `libc.so.6 GLIBC_2.32` 88 packages,
     `ld-linux-x86-64.so.2 GLIBC_2.2.5` 9; GLIBC_2.36 in the reference's own programs.
2. **Tables:** `spec/tools/abi_extract.py` writes `spec/abi/empty-versions.tsv` (library, version, parent): every
   version definition no exported symbol carries, from `readelf -V` and `--dyn-syms` of the reference (Ubuntu 2.39).
   Run with `--empty-versions-only` (appends its PROVENANCE line). All of them: `libc.so.6 GLIBC_ABI_DT_RELR`
   (parent GLIBC_2.39), `libm.so.6 GLIBC_2.4`, `ld-linux-x86-64.so.2 GLIBC_2.4`. (Outside the extractor's library
   set: libnss_files.so.2 and libnss_dns.so.2 define a symbol-less GLIBC_PRIVATE; not a libniji product.)
   SPEC-ABI-ADDITIONS has not landed (no additions rule to follow).
3. **Product:** `xtask/src/empty_versions.rs` lists the honoured nodes (`HONOURED`, with why: the loader applies
   DT_RELR in `crates/ld/src/reloc.rs` and `boot.rs`; GLIBC_2.4 is an ABI level without symbols); a table row not
   listed stops the build (a node for a feature is a ruling). `so.rs` defines each node empty in the libc.so.6,
   libm.so.6 and ld.so version scripts (GLIBC_ABI_DT_RELR after the numbered nodes, parent GLIBC_2.39) and
   `cargo xtask so` checks the linked products' version definitions (`so: 3 symbol-less version nodes defined`).
   `tools/deb/symbols.py`: a named symbol-less node gets the build's upstream version
   (`GLIBC_ABI_DT_RELR@GLIBC_ABI_DT_RELR <upstream>`: libniji builds before this one lack it); `check-symbols.sh`
   accepts exactly that line. `swap-system.sh` keeps Debian's own iconv and zdump before the swap and `swap-cmds.sh`
   runs `--version` of each in both roots (`relr-iconv`, `relr-zdump`).

## Decisions in force / open

* No D-row: the node list and its rule are in `empty_versions.rs` (a local, testable detail).
* INFORMATIONAL: dpkg-shlibdeps derives dependencies from imported symbols; whether it reflects a symbol-less
  requirement (GLIBC_ABI_DT_RELR) in `shlibs:Depends` was not measured here (check-symbols has no RELR sample).
* The second binary in the proof is Debian's zdump (libc-bin), not thunderbird (the one sampled user: too large for
  the swap root's minimal package set).

## Jobs and results

See `handoff/SYS-ABI-DT-RELR-report.md` (proof, plants, gate labels and verdicts).

## Next step

None for this story once its jobs are green. Possible follow-up: a check-symbols sample linked with
`-z pack-relative-relocs` to pin dpkg-shlibdeps' result.
