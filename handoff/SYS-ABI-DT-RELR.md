# SYS-ABI-DT-RELR: libc.so.6 defines GLIBC_ABI_DT_RELR; the ABI tables record symbol-less nodes

**State:** branch `task/sys-abi-dt-relr-2` (continues `task/sys-abi-dt-relr`), cut from origin/next 752955498.
Code head 61c9a7ad3 (+ plants file 71afa1e11 + this handoff). Role keke:~/w/n3 (SYS-ABI-DT-RELR ran on mei:~/w/s4).
Report json: `handoff/reports/SYS-ABI-DT-RELR-2.json`; plants: `handoff/SYS-ABI-DT-RELR-2-plants.txt`.

## What the branch does

1. **Count** (`tools/deb/parity.py`, SYS-ABI-DT-RELR): `rank` writes `consumers-verneeds.tsv`, the GLIBC-family
   version requirements a package's ELF files record with no symbol imported at them; `parity.py verneeds DEB...`
   does the same for local .debs. Fresh trixie indices (2026-11-17), 4000 of 23193 packages sampled, 0 failed:
   `libc.so.6 GLIBC_ABI_DT_RELR` **1** package (thunderbird); the reference's own packages: libc-bin 9 programs
   (getconf, getent, iconv, locale, localedef, pldd, zdump, iconvconfig, zic), libc-dev-bin gencat, libc-devtools 2,
   libc6-dev sotruss-lib.so, nscd, libc6's 270 gconv modules. Numbered nodes required with no symbol:
   `libc.so.6 GLIBC_2.32` 88 packages, `ld-linux-x86-64.so.2 GLIBC_2.2.5` 9.
2. **Tables:** `spec/tools/abi_extract.py --empty-versions-only` writes `spec/abi/empty-versions.tsv` (library,
   version, parent): every reference version definition no exported symbol carries: `libc.so.6 GLIBC_ABI_DT_RELR`
   (parent GLIBC_2.39), `libm.so.6 GLIBC_2.4` (GLIBC_2.2.5), `ld-linux-x86-64.so.2 GLIBC_2.4` (GLIBC_2.3).
   (libnss_files/libnss_dns' symbol-less GLIBC_PRIVATE is outside the extractor's set; not a libniji product.)
3. **Product:** `xtask/src/empty_versions.rs` lists the honoured nodes (`HONOURED`, with why); a table row not listed
   stops the build. `so.rs` writes each node into the libc.so.6, libm.so.6 and ld.so version scripts with
   `local: *` and the table's parent. `cargo xtask so` checks each linked product's definition against the
   reference's: present, `Flags: none`, same parent (`so: 3 symbol-less version nodes defined`).
4. **Packaging:** `tools/deb/symbols.py` writes `GLIBC_ABI_DT_RELR@GLIBC_ABI_DT_RELR <upstream>` (dpkg-gensymbols
   sees the node's ABS symbol); `check-symbols.sh` accepts exactly that line and links sample `sym-relr`
   (`smoke/sym-relr.c`, `-z pack-relative-relocs`): it must have DT_RELR, record the GLIBC_ABI_DT_RELR requirement
   against the packaged libc.so.6, and get `libc6 (>= 2.2.5)`. `swap-system.sh` keeps Debian's own locale and zdump
   before the swap; `swap-cmds.sh` runs `--version` of each in both roots (`relr-locale`, `relr-zdump`).

## SYS-ABI-DT-RELR-2 (this run)

* The red verify (`sys-abi-dt-relr-proof`, mei, rc 2) was setup: `tools/deb/build.sh` in a role directory needs
  `NIJI_DEB_COMMIT` and `SOURCE_DATE_EPOCH`. The new proof sets both (61c9a7ad3, its commit time).
* **Verdef flags (coordinator's check):** the reference's three symbol-less nodes are `Flags: none`. libniji's were
  `WEAK` (libc's GLIBC_ABI_DT_RELR, ld.so's GLIBC_2.4; libm's GLIBC_2.4 happened to hold the script's `local: *`):
  GNU ld 2.42 flags a node that names no symbol and no pattern WEAK (probe: `E { } A;` WEAK; `E { local: *; } A;`
  none, exports unchanged, also beside an earlier `local: *`). Parents differed too: GLIBC_ABI_DT_RELR took the
  script's previous node GLIBC_2.41 (libniji's sched_getattr node), the reference's is GLIBC_2.39; ld.so's GLIBC_2.4
  had none (reference GLIBC_2.3). Fixed to match the reference; the check now compares flags and parent.
  Index numbers still differ (libc GLIBC_ABI_DT_RELR index 41 vs 40, after GLIBC_2.41): not ABI (a consumer
  records its own vna_other).
* **#6219 dpkg-shlibdeps (measured, keke, host's dpkg-dev and libc6 symbols file, which has
  `GLIBC_ABI_DT_RELR@GLIBC_ABI_DT_RELR 2.36`):** a program whose only tie to the node is the requirement gets no
  dependency from it: a RELR sample importing up to GLIBC_2.34 gets `libc6 (>= 2.34)`, the same as its non-RELR
  twin; Debian's zdump gets `libc6 (>= 2.38)` and iconv `libc6 (>= 2.38), libc6 (>> 2.39), libc6 (<< 2.40)`
  (its GLIBC_PRIVATE imports), both from imported symbols. So the symbols file cannot protect a RELR program
  from a libniji build that predates the node. Today this only concerns earlier libniji snapshot builds
  (2.39+niji... versions sort above every >= 2.36 dependency). Pinned by check-symbols' `sym-relr` sample.
* **Debian's iconv still does not start on libniji, for another reason** (proof-1, swapped trixie root):
  `ld.so: libc.so.6: version 'GLIBC_PRIVATE' not found (required by iconv)`: it imports `__gconv_open`,
  `__gconv_create_spec`, `__gconv_destroy_spec`, `__gconv_get_cache`, `__gconv_get_modules_db`, `__gconv_get_alias_db`
  at GLIBC_PRIVATE. Of libc-bin's nine RELR programs, getent (`__libc_dynarray_resize`) and pldd
  (`__libc_scratch_buffer_grow`) import GLIBC_PRIVATE too; getconf, locale, localedef, zdump, iconvconfig, zic do not.
  The system flavour replaces libc-bin with libniji's own, so these three are never run against libniji there.
  The swap's second RELR program is now Debian's `locale` (zdump passed in proof-1). RULING NEEDED (raised in the
  hand-back): the original brief's "iconv --version starts" is met for RELR, not GLIBC_PRIVATE; recommend no
  GLIBC_PRIVATE (R14-12 stands).
* The swap's ldd-1..7 differences (/usr/lib vs /lib in loader trace output) are the known ones SYS-LIBRESOLV found
  identical on origin/next's packages; this run did not rebuild the base packages to re-confirm them. The proof
  script accepts exactly those seven FAIL lines and requires `relr-locale` and `relr-zdump` to PASS.

## Jobs (keke)

* `sys-abi-dt-relr-2-lint0` (inherited head 972e1b46a: clippy + static) rc 0.
* On 61c9a7ad3 (superseded by the swap change): `-lint1` rc 0 (clippy, static, unit tests); `-plants` 8/8 CAUGHT;
  `-proof` rc 1 (verdefs equal the reference's, check-symbols PASS, relr-zdump PASS; relr-iconv GLIBC_PRIVATE, ldd-1..7);
  `-gate` and `-startup` stopped by me when the swap change made them stale.
* Final head content (31b26287d + prose): `sys-abi-dt-relr-2-gate2` (direct tier), `-plants2` (8 plants),
  `-proof2` (both flavours, verdefs vs reference, check-symbols, swap): rc 0, relr-locale and relr-zdump PASS,
  swap same=58 differ=7 (ldd-1..7 only); `-plants2` 8/8 CAUGHT. The gate's affected run escalated to
  `cargo xtask test` (so.rs feeds every product), so `-startup2` (a second `cargo xtask test`) was stopped and
  `-startup3` runs only the full-profile so + hot_text_products after the gate. Gate verdict: the hand-back.

## Decisions in force / open

* No D-row: the node list, the WEAK/parent rule and the check live in `empty_versions.rs` (local, testable).
* INFORMATIONAL: dpkg-shlibdeps ignores symbol-less requirements (above); no change proposed.
* The proof's second binary is Debian's zdump (libc-bin), not thunderbird (too large for the swap root).

## Next step

None for this story once its jobs are green.
