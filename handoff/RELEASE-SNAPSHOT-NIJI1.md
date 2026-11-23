COMPLETE: snapshot-ready at 30d07e74b (RELEASE-SNAPSHOT-NIJI1 + -2): every check green; #6 partly fixed (ruled not blocking, #6199)

# RELEASE-SNAPSHOT-NIJI1: first libniji snapshot, validation of 30d07e74b

**State:** head to tag `30d07e74b5597c52e67cb4feac112f3d1d2d756e` "Regenerate train-owned views" (origin/next at the
start of run 1, 2026-11-17 03:05:35 UTC, epoch 1794884735). Branch `task/release-snapshot-niji1-2` = that head + handoff/
only (no product or packaging change). `git log --oneline 23b9dfdb5..30d07e74b`: 145 lines, 127 non-merge commits.
Run 1 (keke role n2, jobs `release-snapshot-niji1-*`) built and checked; run 2 (mei role s3 + keke `~/work/
RELEASE-SNAPSHOT-NIJI1-2/`, jobs `release-snapshot-niji1-2-*`) fixed the red, finished swap, lint, plants, staging, 4(b).
Scripts: keke `~/work/RELEASE-SNAPSHOT-NIJI1{,-2}/` (run 2: roots.sh, swapverdict.sh, stage.sh, install.sh, inside.sh,
tree/ = `git archive 30d07e74b`, tools/deb identical to n2's).

**Run 1's red (verify: roots-1 rc 1) and its cause:** validate.sh took swap-system.sh's exit status as the verdict;
swap-system.sh exits 1 whenever any command differs, including ldd-1..7, the expected DL-TRACE-INTERP loader-trace
difference. Plants-1's baseline-3 failed the same way (rc 2, no plant applied). Fix (job scripts and the plants file
only): swapverdict.sh, UTIL-LDD-3's verdict (all 63 commands ran; every one except ldd-1..7 identical; ldd-1..7 identical
after normalising exactly (a) `/usr/lib` vs `/lib` spelling, (b) bare interpreter name, (c) the libc/libm `-v` blocks).

## Results vs snapshot-2026-11-16 (all on 30d07e74b's products; libc.so.6 sha256 bc64496a...)

| check | job (host) | result | vs 11-16 |
|---|---|---|---|
| system + coexist builds, check-system, check-symbols | debs-1 (keke) | 6 + 2 packages; `check: PASS (6 packages, version 2.39+niji20261117.1-1)`; `PASS (4217 symbol lines, 12 libraries)` | rename; F1 fixed; +82 lines, +libresolv.so.2, +libBrokenLocale.so.1 |
| package contents | pkgdiff-1 (keke) | as run 1 listed: SYS-LIBRESOLV, SYS-DEV-LINKS, SYS-CONTROL-PARITY, UTIL-LDD files/fields only | each change traced; Maintainer kept (owner 11-16, not a finding) |
| empty root | -2-roots-1 (keke) | `root: RESULT pass=19 fail=0` | 15 -> 19 |
| swapped trixie root | -2-roots-1 (keke) | `swap: RESULT same=56 differ=7`; `verdict: commands=63 same=56 known=7 bad=0`; cache 63 libs (stock 71) | 11-16's 52 commands all still identical; new since: ldd-1..8, copyexec-1, gawk-1/2; ldd-1..7 differ only by (a)-(c) as expected |
| lintian trixie / excalibur | lint-1 (keke) | 10 E / 40 W each | 7 E / 48 W: see below |
| coexist proofs | lint-1 (keke) | trixie and excalibur `pass=21 fail=0` each | same |
| lua_reload, cxx_dlclose | smoke-keke-1, smoke-mei-1 | ok 1/1 both sides | same |
| gate | gate-1 (keke), -2-lint-0 (mei) | clippy -D warnings (xtask + workspace --all-features), static, records, affected --base origin/next: rc 0 | - |
| plants | -2-plants-1 (mei) | 6 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY/NO-MSG (handoff/RELEASE-SNAPSHOT-NIJI1-2-plants.txt, with results) | CXX: 4 of 6 (F1 set two aside); now all 6 |

**Lintian changes, each explained:** +3 E `no-code-sections` libg.a, libmcheck.a, libpthread_nonshared.a (the empty
archives SYS-DEV-LINKS added; same tag as the 6 existing empty archives); -2 W `lacks-unversioned-link-to-shared-library`
libanl.so, libc_malloc_debug.so (links added by SYS-DEV-LINKS); -8 W `undeclared-elf-prerequisites (libc.so.6)`: all
gone, libresolv.so.2's and libBrokenLocale.so.1's included, by libc6's new override (df340a23b,
`usr/share/lintian/overrides/libc6`, only the self-provided libc.so.6 prerequisite); +2 W libc-bin `/usr/bin/ldd`:
`no-manual-page` and `shared-library-lacks-prerequisites` (UTIL-LDD's ldd is a static-pie ELF, no NEEDED, which lintian
classes as a shared library).

## The four Yohane fixes (run 1, yohane-2, swapped trixie root, Debian's own binaries)

* #8: netns with loopback only: `getaddrinfo: -3 EAI_AGAIN: Temporary failure in name resolution` on stock and libniji
  (`nameserver 192.0.2.53`, `nameserver 127.0.0.1`); `getent ahosts example.org` status 2 on both. Fixed.
* #7: Debian gawk 1:5.2.1-2+b1 binds `fmod@GLIBC_2.2.5`; `gawk 'BEGIN { print 3 % 1.5 }'` prints `0`. Fixed.
* #5: msmtp 1.8.28 (NEEDED libresolv.so.2) `--version` runs; a `-lresolv` program (res_mkquery, ns_initparse, ns_parserr,
  dn_expand) links against libniji's libc6-dev, dynamic and static, output identical to stock. Fixed.
* #6 (ruling #6199: partly fixed, not blocking): EUC-JP round trip through iconv(3) (C program and Text::Iconv) byte
  identical, EUC-JP bytes identical to stock. 58 of the stock root's 1180 `iconv -l` names open both ways against UTF-8:
  8859_1 10646-1:1993 ANSI_X3.4-1968 ANSI_X3.4-1986 ASCII CP367 CP819 CSASCII CSEUCPKDFMTJAPANESE CSISOLATIN1 CSUCS4
  CSUNICODE EUC-JP EUCJP IBM367 IBM819 ISO-8859-1 ISO-10646 ISO-IR-6 ISO-IR-100 ISO-IR-193 ISO646-US ISO8859-1
  ISO_646.IRV:1991 ISO_8859-1 ISO_8859-1:1987 L1 LATIN1 OSF00010001 OSF00030010 UCS-2 UCS-2BE UCS-2LE UCS-4 UCS-4BE
  UCS-4LE UCS2 UCS4 UJIS UNICODEBIG UNICODELITTLE US-ASCII US UTF-8 UTF-16 UTF-16BE UTF-16LE UTF-32 UTF-32BE UTF-32LE UTF8
  UTF16 UTF16BE UTF16LE UTF32 UTF32BE UTF32LE WCHAR_T. ISO-8859-15 does not open (`Invalid argument`). `iconv -l` cannot
  run: libniji ships no iconv program, and Debian's `/usr/bin/iconv` stops with `version GLIBC_ABI_DT_RELR not found`
  (libc.so.6 has no GLIBC_ABI_DT_RELR version node).

## Documentation currency

(a) README.md + docs/ (53 files): 238 links, 0 broken; 197 `cargo xtask` mentions, 0 unknown; 17 of 388 backticked paths
missing: 2 not references; stale since CXX-DOCS: compat-flags.md s29 test, saturation/07 hot-text.list, 11 removed
handoffs in saturation/01-05; new: differences-from-glibc.md:307 `records/pending/SL-HEADERS-S05-3-1.toml` (now assigned).
(b) **pass** (-2-install-1): docs/install-packages.md followed literally in a fresh trixie root against the staged
assets: `sha256sum -c SHA256SUMS`, published names (`.` for `~`), `apt install ./niji6_... ./niji6-dev_...`, the four
niji-gcc/niji-g++ builds run, purge leaves the package set and /etc, /usr as before: 24 ok, 0 FAIL; the 6 staged system
.debs byte-identical to debs-1's.
(c) changes since 11-16 and their sentence: rename -> README.md:3 "libniji was called Liella until November 2026." only,
no old-to-new name table where a package user looks (install-packages.md has none): INFORMATIONAL; #8 EAI_AGAIN: none;
#7 libm/libc GLIBC_2.2.5 versions (fmod, fmodf, exp, exp10, pow, hypot, hypotf, log, log2, pthread_sigmask): none; #6
EUC-JP: none; #5 libresolv.so.2: install-packages.md:13 (true); libBrokenLocale and dev links/archives: :14, :124 (true);
control fields: :124-125 (true); static ldd and its interpreter line: install-packages.md:123, README.md:72,
toolchain.md:72 (true); the ldd trace difference: docs/RELEASE-NOTES.md:111 (true). No docs/RELEASE-NOTES.md section for
this snapshot yet (the coordinator writes the notes). **Wrong sentence (#6201, for the docs story):**
install-packages.md:124, libc6-dev row: "No headers yet: plain `gcc` compiles with the system's glibc headers" is false
on a system-flavour root: libniji's libc6-dev replaces Debian's, so there are no glibc headers (no `/usr/include/stdio.h`).

## Staged (keke `~/release-staging/snapshot-20261117/`, serial 1; job -2-stage-1; `sha256sum -c` 8 OK)

| asset | bytes | sha256 (first 20) |
|---|---|---|
| libc6_2.39+niji20261117.1-1_amd64.deb | 2693420 | e945e7b4a65ae6785077 |
| libc6-dev_2.39+niji20261117.1-1_amd64.deb | 3001884 | 903aff2f07d509bce435 |
| libc-bin_2.39+niji20261117.1-1_amd64.deb | 372180 | ab2bd2108939eb8ca00c |
| libc-dev-bin_2.39+niji20261117.1-1_amd64.deb | 1952 | 13f3e38ce9e1e6cd2c85 |
| libc-l10n_2.39+niji20261117.1-1_all.deb | 1928 | ecc03a6fce0d186fde92 |
| locales_2.39+niji20261117.1-1_all.deb | 1936 | 9ae06a00d8344a54aa29 |
| niji6_0.0.snapshot20261117+30d07e74b559_amd64.deb | 2677140 | ec3799e9803132634901 |
| niji6-dev_0.0.snapshot20261117+30d07e74b559_amd64.deb | 2999564 | 66e43561b2195aa4b917 |

Plus SHA256SUMS and MANIFEST (private: names the build host). Nothing published, no tag.

## Open / next

Rulings already given (11-10 notes): #6199 #6 partly fixed, not blocking (follow-ups briefed); #6200 Maintainer kept;
#6201 headers gap (SL-HEADERS), the sentence above goes to the docs story. Next: the coordinator tags 30d07e74b and
publishes the staged assets. Do not redo: builds, checks, roots, lint, staging, 4(b).
