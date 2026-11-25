# HEADERS-STAGE2-GAPS-C-2: merge with next, thin-header completeness pass 2

Continues `handoff/HEADERS-STAGE2-GAPS-C.md` (legs 1-3: guards, utimbuf, ar.h, paths.h, DTTOIF, radio structures; do not redo).

**State.** Branch `task/headers-stage2-gaps-c-2` (from task/headers-stage2-gaps-c 65e599cf3, merged with origin/next of
2026-11-25 15:5x). Build role: the Zen 1 host (60 vCPU), also the reference system (GLIBC 2.39, GCC 13.3). Scratch: `~/work/headers-stage2-gaps-c-2/`.

## Done this leg
1. **Merge with next** (the CI conflict): `headers/syslog.h.in` takes next's LOG_PRI/LOG_FAC/LOG_MAKEPRI (same macros, my copy dropped);
   the spec tables (layouts, macros, visibility) and `abi_extract.py`/`header_probe.py` keep both sides; generated files
   (libc-include/*, layout.rs, appendix_c_layouts.rs) taken from next and regenerated with headers-gen/abi-gen, never hand-merged.
2. **Thin-header completeness pass** (`header_probe.py thin_names()`, candidates from C23, utmp(5), queue(3), adjtimex(2), udp(7),
   the IANA TELNET and DNS registries; a candidate is only a name to probe). New rows: **inttypes.h** PRIb*/PRIB*/SCNb* (C23
   7.8.1, 42 names: the whole `inttypes.h` gap of the triage), **netinet/udp.h** UDP_CORK/ENCAP/NO_CHECK6_TX/RX/SEGMENT/GRO,
   UDP_ENCAP_* and SOL_UDP (12), **utmp.h/lastlog.h/utmpx.h** UTMP_FILENAME, WTMP_FILENAME, UTMPX_FILENAME, WTMPX_FILENAME and
   the member aliases ut_name/ut_time/ut_xtime/ut_addr (template forms, probed: same object, same size),
   **arpa/nameser.h and resolv.h** the byte-order macros GETSHORT/GETLONG/PUTSHORT/PUTLONG/NS_GET16/32/NS_PUT16/32, the
   `ns_msg_*` and `ns_rr_*` accessors (types probed: `+ 0` promotion, name "." when empty) and NS_TSIG_ALG_HMAC_MD5.
3. Unit `THIN` in `spec/tools/header_checks/s13.py` (C and C++ where the name is visible); plants in
   `handoff/HEADERS-STAGE2-GAPS-C-2-plants.txt`.

## Guard set (ruling #8587, provisional keep)
The guard names are chosen in one place: `xtask/src/headers.rs` `Gen::guards()` takes each name of `guard_candidates(header)`
(the seven conventional shapes of the header path) that `visibility.tsv` shows as a macro of that header. Narrowing to
the tested set is one change there: filter the loop by a list of names, or drop the guard rows from visibility.tsv.

## Open / next
* Left from the leg-3 triage (counts of non-reserved macros reference/libniji, gnu17): net/if_ppp.h, resolv.h (RES_* are
  all present: the rest is nameser), netinet/icmp6.h, ip6.h, tcp.h, igmp.h, ip_icmp.h, if_fddi.h, if_tr.h, netatalk/at.h,
  protocols/talkd.h and routed.h, sys/quota.h, net/ppp_defs.h, ppp-comp.h, link.h; the radio protocol-private ioctls
  (SIOCAX25*, SIOCRS*, SIOCIPX*) need template forms. queue.h, sys/timex.h, sys/pidfd.h, arpa/telnet.h, ftw.h still show
  counts above libniji's although every candidate name tried was already present: the missing names are outside the
  candidate lists (do not guess more lists: take the family from the owning spec section first).
* arpa/nameser.h: `struct ns_tsig_key` (2072 bytes) and `struct ns_tcp_tsig_state` (544) are shown by the reference and
  absent in libniji (need abi_extract tags and layouts); arpa/telnet.h's TELCMD/TELCMD_OK macros and `telcmds` table
  (the table is a definition under a program-set macro: a template form).
* INFORMATIONAL (leg 3, still open): coreutils' gnulib locale code needs a complete `struct __locale_struct` (locale_t layout key).

## Jobs (Zen 1 host)
See handoff/HEADERS-STAGE2-GAPS-C-2-report.md (written with the hand-back).
