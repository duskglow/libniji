# NL-LOCALE-NAME-ABI: _NL_LOCALE_NAME in <langinfo.h>, static_assert in <assert.h>, layout-macro sweep

**State:** branch `task/nl-locale-name-abi` (from origin/next ecc8c95e3), role keke:~/w/n3. Head: see
`handoff/reports/NL-LOCALE-NAME-ABI.json`. Decisions: none new (both changes follow reference probes and ISO C 7.2;
no differences row, no compat option).

## Commits
* `93027447b` Define _NL_LOCALE_NAME in <langinfo.h> (template, visibility.tsv row, header_probe candidate, s10 form
  rows, sym_nl_langinfo / sym_nl_langinfo_l cases, regenerated libc-include/langinfo.h).
* `3770f6ab5` Define static_assert in <assert.h> (template with a hand-written condition, visibility.tsv row,
  regenerated libc-include/assert.h).
* Plants file `handoff/NL-LOCALE-NAME-ABI-plants.txt`; this page and the report json.

## Probe-settled behaviour (keke, black-box, both libraries)
* `_NL_LOCALE_NAME(c)` = `_NL_ITEM(c, 0xffff)`; type follows the argument (int, long, unsigned); not usable in `#if`;
  shown by langinfo.h in every configuration but time64only (header_probe --names).
* nl_langinfo / nl_langinfo_l of it: the category's name as setlocale(c, NULL) names it, "POSIX" read back as "C",
  alias spellings kept (en_US.utf8), "" for LC_ALL. libniji already served this: identical output for C, POSIX,
  en_US.UTF-8, en_US.utf8, the environment, mixed newlocale, duplocale, uselocale, categories 0..12. libniji gives
  "" for category numbers -1, 13, 100 and answers LC_GLOBAL_LOCALE with the global locale (now tested).
* `static_assert`: assert.h, every C configuration except c89, c99, isoc99, c2x, gnu2x (time64only refused), no C++.

## Proof (perl)
* trixie perl 5.40.1 CORE headers (aqours pool libperl5.40), perl's ccflags: reference HS_KEY 0xeb80487,
  sizeof(PerlInterpreter) 3768, USE_QUERYLOCALE. libniji before: 0xf280487, 3880, USE_PL_CURLOCALES (Yohane's
  "got 0xf280080"). libniji after: 0xeb80487, 3768, USE_QUERYLOCALE. Of 12,919 macro names perl's headers define,
  before: 5 differ (USE_PL_CURLOCALES, USE_QUERYLOCALE, STATIC_ASSERT_1/2, _STDIO_H); after the first commit 3
  (STATIC_ASSERT_1/2 from static_assert, fixed by the second commit; _STDIO_H, below).
* XS load: a minimal XS module built against libniji's headers, loaded by trixie's own perl binary running on
  libniji's loader (job `nl-locale-name-abi-xsload3`): loads (handshake accepted) and answers under LC_ALL=C,
  en_US.UTF-8 and C.UTF-8 ("v5.40.1 setlocale(LC_CTYPE)=en_US.UTF-8 nl_langinfo(_NL_LOCALE_NAME(LC_TIME))=en_US.UTF-8").
  Its negative control (macro removed) is not valid: the probe module itself names the macro, so it failed on an
  undefined symbol before the handshake; the HS_KEY numbers above are the before/after evidence.
* liblocale-gettext-perl build against the system flavour: NOT RUN (budget); the HS_KEY probe is the brief's key proof.

## Sweep (scratch tool, not committed): names tested in #if lines of trixie's perl 5.40, python3.13, openssl 3.5
and glib/gio 2.84 headers (961 upper-case + 256 lower-case), per public header (243) and 7 configurations (gnu17,
c11, gnu, perl's flags, xopen700, t64, gnu++17), exit status and own-file error lines only.
* Layout-changing: only `_NL_LOCALE_NAME` (perl). Fixed.
* Reference defines, libniji's same header does not (not layout): `static_assert` (fixed); `_STDIO_H` (stdio.h and
  every header including it: argp.h, malloc.h, resolv.h, stdio_ext.h; complex.h and tgmath.h in C++); `va_copy`,
  `__va_copy` (printf.h); S_ISBLK/CHR/DIR/FIFO/LNK/REG/SOCK (ftw.h); `CHARBITS` (values.h); `_IOC_SIZE`
  (sys/soundcard.h); `_PATH_RESCONF` (resolv.h); `SPACE` (scsi/scsi.h). All configurations unless named.
* libniji defines, reference does not: `__GLIBC__`, `__GLIBC_MINOR__`, `__GLIBC_PREREQ`, `__THROW` and the feature
  macros `_DEFAULT_SOURCE`, `_POSIX_SOURCE`, `_XOPEN_SOURCE(_EXTENDED)` in arpa/ftp.h, arpa/telnet.h, arpa/tftp.h,
  net/if_slip.h, netpacket/packet.h, net/ppp-comp.h, sys/ttychars.h, sys/ttydefaults.h (they include features.h);
  INT*/UINT*/SIZE_MAX limits in 25 (header, configuration) pairs; `CMSG_LEN` (net/if_ppp.h, net/if_shaper.h,
  netinet/ether.h, netinet/if_ether.h in c11/xopen700; protocols/timed.h everywhere); `S_ISSOCK` in sys/stat.h c11.
* Include differences: a.out.h, stab.h (reference compiles, libniji refuses: legacy R-H7 rows); sys/vm86.h (reverse).

## Open
* gnulib localename (coordinator note 13:20Z): grep's stage-2 log fails at localename.c on the incomplete
  `struct __locale_struct`. Whether _NL_LOCALE_NAME moves gnulib to the nl_langinfo_l path is NOT VERIFIED: the job
  that fetched grep's source to rebuild that object was refused (hand-back REFUSED line). Next: rebuild grep,
  findutils or gettext against this branch's headers in Yohane's stage 2 and read the build log.
* Next step: the train; then the `_STDIO_H` and printf.h `va_copy` rows (cheap, source compatibility only).
