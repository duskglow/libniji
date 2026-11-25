# HEADERS-STAGE2-GAPS-C: stock include guards, utime.h, ar.h, paths.h, dirent.h DTTOIF, completeness pass

**State (end of leg 2, 2026-11-25).** Branch `task/headers-stage2-gaps-c` from origin/next ecc8c95e3; role keke:~/w/n4
(also the reference system: Ubuntu GLIBC 2.39, GCC 13.3). Code head: 9275c0042 (then this note and the report).
Scratch: `~/w/.scratch/HEADERS-STAGE2-GAPS-C/` (local), `~/work/headers-stage2-gaps-c/` on keke. Decisions: none new;
every row follows black-box probes; no differences row.

## What the branch does
* **Guards:** for every shipped header, each conventional include-guard name (`guard_candidates`: shapes derived from
  the header path, `sys/types.h` -> `_SYS_TYPES_H`, `__SYS_TYPES_H`, ...) that the reference defines is defined by the
  generator beside libniji's own `__NIJI_H_` guard, with the reference's value (`1`, or empty for 12 headers).
  **213 of 242 shipped headers** define one (191 in every accepted configuration, 20 `*`, complex.h and tgmath.h not in
  C++11 and later); 29 have none (wrappers such as poll.h, wait.h, sys/fcntl.h; limits.h; stdc-predef.h).
* Rows: `struct utimbuf` complete; ar.h `struct ar_hdr` (60 bytes, align 1), ARMAG, SARMAG, ARFMAG; the BSD `_PATH_*`
  set the reference defines in paths.h, with values; DTTOIF/IFTODT (dirent.h, sys/dir.h); coordinator extras:
  `static_assert` (assert.h), LOG_PRI/LOG_MAKEPRI/LOG_FAC (syslog.h), MNTOPT_*/MNTTYPE_* (mntent.h); sys/pci.h and
  sys/soundcard.h now include the kernel's <linux/pci.h> / <linux/soundcard.h> (975/2 and 497/23 macros before).
* Leg 2: header_check units (s05 guards per header and pattern of shown guards + static_assert; s07 DTTOIF/IFTODT,
  BSD paths, _PATH_DEVNULL, utimbuf; s14 syslog priority macros; s16 ar(5) format), which found two defects, fixed:
  conditional guards (complex.h, tgmath.h) were defined before <features.h> and so showed in C++11+ (now include
  <features.h> first); `static_assert` follows the ISOC level outside C23/C++ (shown under `-std=c99 -D_ISOC11_SOURCE`
  and `-std=c17 -D_ISOC2X_SOURCE`), not the language version alone (0 mismatches against visibility.tsv over all
  configurations).

## Commits
77063f55b sources/tables/generator/templates; df1671e60 regenerated libc-include + crates/abi tables; b86b9bb45
header_check units + the two fixes + plants file; 9275c0042 regenerated libc-include (assert.h, complex.h, tgmath.h);
ccde9d613 typedefs.tsv ar_hdr/utimbuf rows moved to sorted order (static abi-types was red: leg-1 splice appended them).

## Jobs (keke)
* `headers-stage2-gaps-c-hc2` (head 9275c0042): clippy -D warnings ok; FULL header_check **0 failed of 20811** (FORM-DIFF
  notes 1557, the complex.h/tgmath.h guard notes gone); `cargo test -p xtask` 6 suites green; `cargo xtask static` RED
  (abi-types ordering, fixed in ccde9d613) -> rc 1, so hc2 is SUPERSEDED by lint2 for the lint part.
* `headers-stage2-gaps-c-lint2` (head ccde9d613): clippy, static, headers-gen --check, P1 ar.h/utime.h + S07/S16 units.
* `headers-stage2-gaps-c-plants`: **13 plants, 13 CAUGHT, 0 WARN** (results in handoff/HEADERS-STAGE2-GAPS-C-plants.txt;
  ran on 9275c0042, the planted code is unchanged by ccde9d613).
* `headers-stage2-gaps-c-proof2` + `-proof4` (coreutils): against libniji's generated headers (`-nostdinc`, kernel UAPI
  and zlib/gmp headers from packages beside): gmp.h `mpz_out_str` after <stdio.h> in C (gnu17, c89) and C++ (gnu++17):
  ok; bzip2 1.0.8 libbz2.a + bzip2 + bzip2recover: ok; elfutils 0.190 configure + lib + libelf.a: ok; coreutils 9.4
  configure --with-libgmp (HAVE_GMP_H 1, LIB_GMP -lgmp) + src/expr: proof4.
* lint2 RESULT: rc 0 (clippy clean, static clean, headers-gen --check ok, 0 failed of 87). plants and lint2 are the
  JOBS for the code head ccde9d613 (+ hc2 for the full header_check / xtask result on the same generated tree).
* coreutils: RUNNING at leg-2 end: `headers-stage2-gaps-c-proof6` (script ~/work/headers-stage2-gaps-c/proof3.sh,
  `make -k -j16 V=1` of the whole configured tree, judges src/expr: built + `-lgmp` on its link line; prints the count
  and first lines of other errors, which are INFORMATIONAL unless they touch this story's headers). Read with
  `~/bin/jobwait headers-stage2-gaps-c-proof6` and the tail of ~/logs/headers-stage2-gaps-c-proof6.log.
* SUPERSEDED: `-hc1` (0/20166 + xtask green, on df1671e60, before the leg-2 fixes), `-gen3` (found the two defects),
  `-gen4` (new units 0/723, pre-commit tree), `-proof` (script set-up: gmp.h path, missing libgmp10/zlib1g-dev),
  `-proof3`/`-proof4` (my script's gmp check named the wrong config.h / Makefile variable; configure was fine: HAVE_GMP_H
  1), `-proof5` (`make src/expr` alone skips BUILT_SOURCES: gnulib's replacement stdckdint.h was not generated).

## NEXT (leg 3): coordinator note 13:43Z (wins over the brief)
"in your completeness pass, the amateur-radio and legacy protocol headers first: struct sockaddr_ax25
(<netax25/ax25.h>; net-tools, finish line) and the same for <netrom/netrom.h>, <netrose/rose.h>, <netipx/ipx.h>,
<netash/ash.h>, <neteconet/ec.h>." Method as leg 1: candidate names from the kernel UAPI / socket(7)-family man pages
and net-tools' use (never reference header text); probe visibility + layouts with spec/tools/header_probe.py /
abi_extract.py (add the struct tags to its type list and the headers to HEADERS), splice new keys only into
visibility/layouts/typedefs/macros.tsv (**keep typedefs.tsv sorted by (name, KINDS index)**, xtask/src/abi_types.rs),
headers-gen, s16-like member-offset units (or rely on the slice's member-offset unit if the headers' slice has one),
plants, then: full header_check + `cargo test -p xtask` + clippy + static job, plants job, precheck on the final head,
report json update (head, jobs), handback-check. Build a net-tools-like probe (sockaddr_ax25 with ax25_call) as proof.
* Gate: generated tables and the generator change: `cargo xtask affected` is FULL (the train's); per the brief,
  precheck + the readers' tests (xtask) + header_check instead.

## Not done / next (completeness triage, counts only: non-reserved macros reference/libniji, gnu17)
scsi/scsi.h 131/2; netrom/netrom.h 67/2; scsi/sg.h 56/2; netax25/ax25.h 52/2; neteconet/ec.h 10/2; netinet/icmp6.h
-102; net/if_ppp.h -96; netinet/tcp.h -82; netinet/ip6.h -79; netrose/rose.h -78; resolv.h -64; netinet/ip_icmp.h -53;
inttypes.h -42; netinet/if_fddi.h -40; netinet/igmp.h -36; link.h -34; elf.h -31 (GAPS-B's); netipx/ipx.h -26;
arpa/nameser.h -21; sys/queue.h -13; lastlog.h/utmp.h -13; arpa/telnet.h -12. Families: SCSI command/status codes
(scsi/scsi.h, scsi/sg.h: SCSI specs, sg(4)); AX.25/NET/ROM/ROSE/Econet/IPX socket options (kernel UAPI); ICMPv6, IPv6
extension headers, TCP options, IGMP, FDDI (RFC 4443, RFC 8200, RFC 793/7323, RFC 2236, IEEE 802 docs); resolver and
nameser (RFC 1035); PPP (RFC 1661). Full table: keke ~/work/headers-stage2-gaps-c/triage.tsv. Next story: start from
the 2-macro network-family headers.

## Rulings / notes
* RULING NEEDED: R-H2 says "exactly the names external headers and programs test"; this story defines every probed
  guard of every header (213), not only tested ones. Recommend keep (same meaning, cheap, programs test unknown subsets).
* INFORMATIONAL: the form probe compiles C++ with `-nostdinc` and no C++ library, so complex.h/tgmath.h's C names
  (`I`, `_Complex_I`, `complex`, and now the guards) show as one-sided FORM-DIFF notes in cxx17, which a real C++ build
  does not reach (the C++ library's <complex.h> stands in). Pre-existing for the other names.
* Leg-1 slips (rules): one remote here-string (`<<<`), one `sed -i` (abi_extract.py HEADERS line), one read of a
  generated header with cat. Leg 2: none known.
