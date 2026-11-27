# HEADERS-STAGE2-GAPS-H: report (run 1)

## Commits (task/headers-stage2-gaps-h, base origin/next 0f7aa0180)
* 730f215ea Define the FTP_NAMES name tables of <arpa/ftp.h>
* c11d191fc, eec75957b (WIP subjects) declare struct sched_attr, sched_setattr, sched_getattr in <sched.h>; spec
  section 24.3 text; sched plants; NIJI-HEADERS directive on the ftp conformance program
* d5454f918 Compare no attr_probe line of a prototype beyond the reference
* 986fb4c2f Regenerate libc-include
* 8e0b14e84 Plant the additions table out of attr_probe check
* then handoff/report commits only

## Tests and jobs (keke, role n1, code head 8e0b14e84)
* headers-stage2-gaps-h-test3: headers-gen --check ok; attr_probe check base 0 differ of 17157 tests (sched_getattr,
  sched_setattr listed as known: D-24.12); `cargo test -p xtask s19` 2 passed.
* headers-stage2-gaps-h-test2 (same headers): header_check 0 failed of 26137; probe4 vs the 2.41 root: 0 diff lines;
  conformance s13_arpa_ftp_names passed. Its s19 failure is the one d5454f918 fixes (test3).
* clippy -D warnings rc 0; cargo xtask static rc 0 (codemap: 1 file pending regeneration by the train).
* headers-stage2-gaps-h-plants2: 11 plants, 11 CAUGHT, 0 SURVIVED, 0 WARN (results in
  handoff/HEADERS-STAGE2-GAPS-H-plants.txt).
* headers-stage2-gaps-h-pkg3 (rc 1: both further stops below; debs from code head 8e0b14e84, system flavour, trixie
  chroot, stock built first): mmdebstrap rc 0, swap dpkg rc 0 (libc6, libc6-dev 2.39+niji20261127.1-1).
  inetutils: stock rc 0, libniji rc 2. ftpd.c now builds (the brief's stop is gone); the NEXT STOP is
  `<protocols/talkd.h>`: `CTL_MSG`, `CTL_RESPONSE`, `ANNOUNCE`, `LEAVE_INVITE`, `RING_WAIT` and the struct members
  (`vers`, `type`, `id_num`, `addr`, `l_name`, `r_name`, `r_tty`, `answer`) undeclared; units talk/invite.c,
  get_names.c, ctl.c, ctl_transact.c, get_addrs.c (the item-2 sweep's GAP).
  util-linux: stock rc 0, libniji rc 2. Every unit compiles (chrt and uclampset included); the build stops at
  dpkg-gensymbols: `uuid_time64@UUID_2.40` missing from libuuid1. Cause (util-linux 2.41.5's own
  libuuid/src/uuid.h and uuid_time.c): the symbol exists only `#if defined(__USE_TIME_BITS64) && defined(__GLIBC__)`.
  Black-box compile probe (`#ifndef __USE_TIME_BITS64 / #error`, after <time.h>): 2.41 headers (excalibur root)
  define it with no flags, with `_FILE_OFFSET_BITS=64` and with `_TIME_BITS=64`; stock 2.39 and libniji define it in
  none of these. So this is the ABI level (trixie's util-linux symbols file is 2.41's), not a 2.39 header gap.
* SUPERSEDED: headers-stage2-gaps-h-plants1 (two survivors: the conformance program read the host's <arpa/ftp.h>;
  fixed by NIJI-HEADERS), headers-stage2-gaps-h-pkg2 (libc-include not committed; fixed by 986fb4c2f),
  headers-stage2-gaps-h-test2 (s19 red, fixed by d5454f918).

## Probe-settled behaviour
See handoff/HEADERS-STAGE2-GAPS-H.md (FTP_NAMES tables: type `char *[]`, extern-less definitions, element counts
and strings as stock 2.39, only with FTP_NAMES defined; sched_attr family as the 2.41 interface).

## INFORMATIONAL
* `<protocols/routed.h>`, `<protocols/timed.h>`, `<protocols/rwhod.h>`, `<protocols/talkd.h>` lack their protocol
  names; recommend one follow-up header story for the four (inetutils talk/talkd, routed-style tools need them).
* attr_probe now skips (prints) lines of prototypes-additions.tsv functions the reference cannot build; their
  attributes (nonnull 2 on both) were settled by the 2.41 probe, not by slice 19.

## RULING NEEDED
* util-linux on trixie needs `__USE_TIME_BITS64` defined as 2.41's headers define it on x86-64 (every
  configuration), or libuuid1 loses `uuid_time64@UUID_2.40`. Recommend: a follow-up header story that takes this one
  2.41 header behaviour early under ruling R3's terms (as D-24.12 took sched_setattr), with a differences row if the
  owner counts it a difference from 2.39.

Refusals: none.
