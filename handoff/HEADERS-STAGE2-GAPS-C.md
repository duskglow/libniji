# HEADERS-STAGE2-GAPS-C: stock include guards, utime.h, ar.h, paths.h, dirent.h DTTOIF, completeness pass

**State (leg 1, 2026-11-25).** Branch `task/headers-stage2-gaps-c` from origin/next ecc8c95e3; role keke:~/w/n4 (also the
reference system: Ubuntu GLIBC 2.39, GCC 13.3). Scratch: `~/w/.scratch/HEADERS-STAGE2-GAPS-C/` (local),
`~/work/headers-stage2-gaps-c/` on keke (probe scripts, outputs). Decisions: none new; every row follows black-box
probes; no differences row. HEAD: see `git log -1` (leg 1 ends with the regenerated tree committed).

## Done (leg 1)
* header_probe.py: `guard_candidates(header)` (conventional guard shapes generated from the header path) and
  `stage2_names()` (guards of every public header; ARMAG/SARMAG/ARFMAG, `struct ar_hdr`, `struct utimbuf`, BSD
  `_PATH_*` set, DTTOIF/IFTODT; coordinator note: static_assert, LOG_PRI/LOG_MAKEPRI/LOG_FAC, MNTOPT_*/MNTTYPE_*).
  abi_extract.py: `struct ar_hdr`, `struct utimbuf` in the type list, ar.h and utime.h in HEADERS.
* Tables spliced (new keys only, 0 conflicts): visibility.tsv +310, macros.tsv +103, typedefs.tsv +2, layouts.tsv +11
  (ar_hdr 60/1, utimbuf 16/8). Two compiler-predefine rows (`-` __ELF__/__TIME__, from a `__X__` candidate shape) were
  dropped with the shape.
* Generator (xtask/src/headers.rs): `guard_candidates` (same list as the probe) and `Gen::guards`: each candidate
  visibility.tsv shows in the header as a macro is `#define`d right after libniji's `__NIJI_H_` guard (assert.h: after
  the banner), with its macros.tsv value, empty when the reference's is empty, under its condition when not universal
  (complex.h, tgmath.h: not C++11+). Guard names are owned (not re-emitted).
* Templates: dirent.h.in IFTODT/DTTOIF; syslog.h.in LOG_PRI/LOG_MAKEPRI/LOG_FAC; assert.h.in static_assert;
  sys/pci.h.in and sys/soundcard.h.in now include the kernel's <linux/pci.h> / <linux/soundcard.h> (sys/vt.h pattern).
* ar.h, utime.h, paths.h, mntent.h need no template change: their rows come from the tables via @@emit.

## Probe-settled (keke, black box)
* Guards: 213 of 242 shipped headers define one of the path-derived candidates (`!time64only` 191, `*` 20,
  complex.h/tgmath.h not in C++11+). Values 1, except 12 defined empty (_ARGP_H, _ARPA_NAMESER_H_, _NET_PPP_COMP_H,
  _PATHS_H_, _RESOLV_H_, _SIGNAL_H, _SYS_MSG_H, _SYS_PLATFORM_X86_H, _SYS_QUEUE_H_, _SYS_SINGLE_THREADED_H,
  _SYS_TERMIOS_H, _SYS_TTYDEFAULTS_H_; stringification probe). 29 without a path-derived guard (wrappers such as poll.h,
  wait.h, sys/fcntl.h; limits.h; stdc-predef.h is implicitly included).
* IFTODT(m) = ((m) & 0170000) >> 12, DTTOIF(t) = (t) << 12; argument parenthesised; type of the promoted argument;
  usable in #if; visible in dirent.h and sys/dir.h (MISC configurations).
* LOG_MAKEPRI(LOG_USER, LOG_ERR) = 11 (or'ed, not shifted), LOG_PRI(0x1b) = 3, LOG_FAC(LOG_USER|LOG_ERR) = 1; types follow
  the argument; usable in #if. static_assert: a macro (`_Static_assert`) in c11, c17, gnu11, gnu17 only.
* sys/pci.h shows PCI_VENDOR_ID, sys/soundcard.h shows SNDCTL_DSP_RESET (kernel names).

## Completeness triage (counts only: non-reserved macros reference/libniji, gnu17)
sys/pci.h 975/2 and sys/soundcard.h 497/23 (fixed above, kernel includes); scsi/scsi.h 131/2; netrom/netrom.h 67/2;
scsi/sg.h 56/2; netax25/ax25.h 52/2; neteconet/ec.h 10/2; mntent.h 13/2 (MNTOPT_*/MNTTYPE_* rows added); netinet/icmp6.h
-102, net/if_ppp.h -96, netinet/tcp.h -82, netinet/ip6.h -79, netrose/rose.h -78, resolv.h -64, netinet/ip_icmp.h -53,
inttypes.h -42, netinet/if_fddi.h -40, netinet/igmp.h -36, link.h -34, elf.h -31 (GAPS-B's), netipx/ipx.h -26,
arpa/nameser.h -21, sys/queue.h -13, lastlog.h/utmp.h -13, arpa/telnet.h -12. Errors (no count): regexp.h, sys/elf.h,
sys/vm86.h (shipped to fail). Full table: scratch triage.tsv.

## Exact next step (leg 2)
1. Fetch the regenerated tree from job `headers-stage2-gaps-c-gen1` (~/work/headers-stage2-gaps-c/libc-include.tar) if
   not yet committed; read its header_check --only result in ~/logs/headers-stage2-gaps-c-gen1.log.
2. header_check rows: a guards unit (each header's guards defined with the probed value) and DTTOIF/IFTODT, LOG_*,
   static_assert, ar_hdr form units (header_checks/s*.py of the owning slice); plants file
   handoff/HEADERS-STAGE2-GAPS-C-plants.txt (guard dropped, IFTODT unparenthesised, LOG_MAKEPRI shifted, static_assert
   in c2x, ar_hdr).
3. Jobs on keke: clippy + static; header_check full; `cargo test -p xtask`; plants; proof builds (coreutils expr.c with
   gmp.h mpz_out_str, bzip2, elfutils libelf; cheap extras vim bufwrite.c / e2fsprogs dump.c / dhcpcd).
4. Report json, precheck, handback-check.
