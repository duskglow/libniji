INCOMPLETE (checkpoint): validated head 30d07e74b so far: builds, checks, gate, empty root, smoke and three of the four Yohane fixes green; #6 only partly fixed (finding Y6); swap, lint, plants still running; staging and 4(b) not started

# RELEASE-SNAPSHOT-NIJI1: first libniji snapshot, validation of 30d07e74b (checkpoint)

**State:** branch `task/release-snapshot-niji1` (handoff/ only on top of the head). Head to tag: origin/next at start,
`30d07e74b5597c52e67cb4feac112f3d1d2d756e` "Regenerate train-owned views" (2026-11-17 03:05:35 UTC, epoch 1794884735).
`git log --oneline 23b9dfdb5..30d07e74b`: 145 lines, 127 non-merge commits. Role keke `~/w/n2`; job scripts and logs in
keke `~/work/RELEASE-SNAPSHOT-NIJI1/` (RELEASE-SNAPSHOT-CXX's debs.sh, validate.sh, gate.sh, chain-role.sh, smoke.sh,
stage.sh and CXX-DOCS's install.sh/inside.sh with head, epoch, role and the renamed names changed; new: y/ +
yohane.sh, pkgdiff.sh); mei `~/work/RELEASE-SNAPSHOT-NIJI1/tree` (git archive of 30d07e74b + keke's role products,
libc.so.6 sha256 bc64496a...). Checkpoint reason: context reached ~150k (thread_common budget rule).

## Step 2: reruns on 30d07e74b vs snapshot-2026-11-16 (keke unless noted)

| check | job | result | vs 11-16 |
|---|---|---|---|
| system build | debs-1 | 6 packages, `2.39+niji20261117.1-1`, names libniji's | rename |
| check-system | debs-1 | `check: PASS (6 packages, version 2.39+niji20261117.1-1)` | F1 fixed (11-16 failed) |
| check-symbols | debs-1 | `PASS (4217 symbol lines, 12 libraries)`; dpkg-shlibdeps samples as before plus `sym-resolv: libc6 (>= 2.9)` | +82 lines, +2 libraries (libresolv.so.2, libBrokenLocale.so.1) |
| coexist build + check | debs-1 | rc 0 (`niji6`, `niji6-dev` `0.0~snapshot20261117+30d07e74b559`) | rename |
| package contents | pkgdiff-1 | libc-bin +`/usr/bin/ldd` (269672 -> 372180 B); libc6 +libresolv.so.2, +libBrokenLocale.so.1, +lintian override `usr/share/lintian/overrides/libc6`, Priority required -> optional; libc6-dev +libanl.so, libBrokenLocale.a/.so, libc_malloc_debug.so, libg.a, libmcheck.a, libpthread_nonshared.a, libresolv.so, Depends + linux-libc-dev, libcrypt-dev, rpcsvc-proto, `Provides: libc-dev (= <version>)`; locales, libc-l10n Priority standard, libc-l10n Multi-Arch foreign; niji6-dev +libanl.so, libc_malloc_debug.so; niji6 unchanged (already had libresolv.so.2, libBrokenLocale.so.1) | every change from SYS-LIBRESOLV, SYS-DEV-LINKS, SYS-CONTROL-PARITY, UTIL-LDD; **Maintainer still `Liella Maintainers <liella@koiren.ai>` in all 8 (finding N1)** |
| empty root | roots-1 | `root: RESULT pass=19 fail=0` | 15 -> 19 |
| swapped trixie root | roots-1 (rc 1 from the ldd cases only) | `swap: RESULT same=56 differ=7`; the 7 are ldd-1..ldd-7 (`/lib/x86_64-linux-gnu` vs `/usr/lib/x86_64-linux-gnu` spelling, the interpreter line, `ldd -v` blocks: the DL-TRACE-INTERP difference the brief expects); every other command same (11-16's 52 minus none, plus the gawk/copy_exec cases added since; per-case list not yet compared) | as expected; detailed case-by-case comparison with 11-16 still to do |
| lintian, proofs | lint-1 | QUEUED behind roots-1 (chain) | - |
| lua_reload, cxx_dlclose | smoke-keke-1, mei smoke-mei-1 | ok 1/1 both sides, both hosts | same |
| gate | gate-1 | clippy --all-features -D warnings, xtask clippy, static, records check, affected --base origin/next: all rc 0 | - |
| plants | plants-1 | RUNNING (6 release-check plants, handoff/RELEASE-SNAPSHOT-NIJI1-plants.txt) | - |

## Step 3: the four Yohane fixes, swapped trixie root (yohane-2; stock root first, then libc6, libc-bin, libc6-dev, libc-dev-bin swapped in)

* #8: netns with loopback up only, `nameserver 192.0.2.53` and `nameserver 127.0.0.1`: `getaddrinfo: -3 EAI_AGAIN:
  Temporary failure in name resolution` on stock and libniji (both resolv.conf cases); `getent ahosts example.org`:
  no output, status 2 on both. Same.
* #7: Debian gawk 1:5.2.1-2+b1 binds `fmod@GLIBC_2.2.5`; `gawk 'BEGIN { print 3 % 1.5 }'` prints `0`; also
  `1.5 -1.5 1` for 7.5%2, -7.5%2, 1e300%7. Same as stock.
* #5: msmtp 1.8.28 (NEEDED libresolv.so.2) `--version` runs. res.c (res_mkquery, ns_initparse, ns_parserr, dn_expand)
  linked `-lresolv` against the installed libniji libc6-dev, dynamic (NEEDED libresolv.so.2) and static: output
  identical to stock. Objects compiled before the swap: libniji's libc6-dev ships no headers and replaces Debian's,
  so the swapped root has no `/usr/include/stdio.h` (finding N2).
* #6 (finding Y6): EUC-JP round trip through iconv(3) (C program, and Debian's Text::Iconv) identical, its EUC-JP
  bytes identical to stock (sha256 f1ac0dac..., 100 B). ISO-8859-15 does NOT open (`Invalid argument`); 58 of the
  stock root's 1180 `iconv -l` names open both ways (stock 1180), as SYS-ICONV-CHARMAPS reported (target not
  reached). `iconv -l` cannot run: libniji ships no iconv program, and Debian's `/usr/bin/iconv` does not start
  (`version GLIBC_ABI_DT_RELR not found`, the known gap in validation/results/bincompat/README.md item 3).

## Step 4: documentation currency (partial)

(a) README.md + docs/ (53 files): 238 links, 0 broken; 197 `cargo xtask` mentions, 0 unknown; 388 backticked paths,
17 missing: 2 not references (a proposed path, a generated dir); still stale from CXX-DOCS: compat-flags.md s29 test,
saturation/07 hot-text.list, 11 removed handoffs in saturation/01-05; new stale: differences-from-glibc.md:307
`records/pending/SL-HEADERS-S05-3-1.toml` (now an assigned record). (b) NOT STARTED (needs staging).
(c) drafted: no RELEASE-NOTES section for this snapshot; no old-to-new name table where a package user looks
(README.md:3 one sentence only; install-packages.md none); install-packages.md:105 states the old Maintainer;
install-packages.md:124 "plain gcc compiles with the system's glibc headers" is false in a root where libniji's
libc6-dev replaced Debian's (N2); no sentence for #8 (EAI_AGAIN), #7 (old libm versions), #6 (EUC-JP); ldd: true
(README.md:72, toolchain.md:72, install-packages.md:123); libresolv/BrokenLocale/dev links: true (install-packages.md:13,14,124).

## Next step (continuation, ~35 calls)

1. `~/bin/jobwait release-snapshot-niji1-roots-1 release-snapshot-niji1-lint-1 release-snapshot-niji1-plants-1` on keke;
   swap: compare with 11-16 (52 same; ldd/gawk/copy_exec cases are new since), lintian vs 7 E / 48 W.
2. Stage: `jobrun release-snapshot-niji1-stage-1 test -- sh ~/work/RELEASE-SNAPSHOT-NIJI1/stage.sh` (serial 1), then
   `install.sh` (4b, inside.sh already renamed) as `-install-1`.
3. Finish the report: first line, asset table, rulings Y6, N1, N2.
