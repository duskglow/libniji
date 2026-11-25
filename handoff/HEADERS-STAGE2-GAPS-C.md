# HEADERS-STAGE2-GAPS-C: stock include guards, utime.h, ar.h, paths.h, dirent.h DTTOIF, radio-protocol structures

**State (end of leg 3, 2026-11-25).** Branch `task/headers-stage2-gaps-c` from origin/next ecc8c95e3 (not merged with
next since; next has changed shared tables: the train merges, regenerating generated files). Role keke:~/w/n4 (also the
reference system: Ubuntu GLIBC 2.39, GCC 13.3). Code head: 146de4e37 (then this note, plants results and the report).
Scratch: `~/w/.scratch/HEADERS-STAGE2-GAPS-C/` (local), `~/work/headers-stage2-gaps-c/` on keke. Decisions: none new;
every row follows black-box probes; no differences row. Do not redo: anything below.

## What the branch does
* **Guards:** each conventional include-guard name (`guard_candidates`, from the header path: `sys/types.h` ->
  `_SYS_TYPES_H`, `__SYS_TYPES_H`, ...) the reference defines is defined by the generator beside `__NIJI_H_`, with the
  reference's value: **213 of 242 shipped headers** (29 have none: wrappers, limits.h, stdc-predef.h).
* Rows: complete `struct utimbuf`; ar.h `struct ar_hdr`, ARMAG, SARMAG, ARFMAG; the BSD `_PATH_*` set of paths.h;
  DTTOIF/IFTODT; `static_assert`; LOG_PRI/LOG_MAKEPRI/LOG_FAC; MNTOPT_*/MNTTYPE_*; sys/pci.h and sys/soundcard.h include
  the kernel's headers. Legs 1-2 found and fixed two defects (conditional guards before <features.h>; static_assert level).
* **Leg 3 (coordinator note 13:43Z):** netax25/ax25.h, netrom/netrom.h, netrose/rose.h, netipx/ipx.h, netash/ash.h,
  neteconet/ec.h now declare their structures and constants: `struct sockaddr_ax25`, `full_sockaddr_ax25`,
  `ax25_address`, the AX.25 route/ctl/info/fwd structs, `nr_route_struct`, `sockaddr_rose`, `full_sockaddr_rose`,
  `rose_address`, the ROSE route/cause/facilities structs, `sockaddr_ipx` and the IPX interface/route/config structs,
  `sockaddr_ec`/`ec_addr`, `sockaddr_ash`; AX25_*, NETROM_*, ROSE_* (limits, options, cause codes), IPX_* (frame types,
  flags), ECTYPE_*, SOL_AX25/NETROM/ROSE/IPX. Candidates from the kernel UAPI headers (header_probe.py `radio_names`),
  probed: 146 visibility, 73 macro, 171 layout, 23 type rows (new keys only). The reference's headers do not compile
  together with linux/ax25.h, netrom.h or rose.h either (probed), so libniji defines its own (as the reference does).
* New `spec/tools/header_checks/s13.py`: member offsets of every structure these headers own (P1 checks size/align
  only) and a net-tools-shaped use unit (C, C++), which also compiles on the reference system (checked by hand: gnu17,
  c99, c++98, c++17).

## Commits
77063f55b, df1671e60, b86b9bb45, 9275c0042, ccde9d613 (legs 1-2); 0603568c9 radio tables + probe + s13 + plants3;
146de4e37 regenerated libc-include + crates/abi.

## Jobs (keke) for the final code head 146de4e37
* `headers-stage2-gaps-c-hc3`: clippy -D warnings, static, headers-gen --check, abi-gen --check, FULL header_check
  **0 failed of 21049** (FORM-DIFF notes 1557 as before, none in the six headers), `cargo test -p xtask`, `cargo test -p
  niji-abi`. Result: see hand-back.
* `headers-stage2-gaps-c-plants3`: **3 plants, 3 CAUGHT, 0 WARN** (handoff/HEADERS-STAGE2-GAPS-C-plants3.txt).
* `headers-stage2-gaps-c-plants4`: the 13 leg-2 plants re-run on this head (handoff/HEADERS-STAGE2-GAPS-C-plants.txt;
  13/13 CAUGHT on 9275c0042 before).
* Proofs: `-proof8` net-tools 2.10 lib/ax25.c, ax25_gr.c, netrom*.c, rose*.c, ipx*.c, econet.c, ash.c, af.c with every
  family on, against libniji's headers (`-proof7` SUPERSEDED: version.h, a make product, was missing). `-proof6`
  coreutils: `src/expr.c` compiles with gmp.h (0 warnings, 10 `__gmpz` references); the link of expr fails on
  `rpl_mbrtoc32`: gnulib's localename.c needs a complete `struct __locale_struct` (`error: invalid use of undefined type`),
  so mbrtoc32's replacement was not built. Not this story's headers: INFORMATIONAL, next story (locale_t's layout key,
  start-up rules §9). bzip2, elfutils libelf, gmp.h (C, C++): ok in legs 1-2 (`-proof2`).
* SUPERSEDED: hc1, hc2, gen3, gen4, lint2, plants (older heads), proof, proof3, proof4, proof5, proof7.

## Not done / next (completeness triage, counts only: non-reserved macros reference/libniji, gnu17)
* Radio headers, left: the protocol-private ioctls (SIOCAX25*, SIOCNRDECOBS, SIOCRS*, SIOCAIPXITFCRT, SIOCAIPXPRISLT,
  SIOCIPXCFGDATA, SIOCIPXNCPCONN) and the member alias `sax25_uid`: shown by the reference, but no integer constant in a
  unit including the header alone (expressions on a name the header does not show) - they need a template form.
* scsi/scsi.h 131/2; scsi/sg.h 56/2; netinet/icmp6.h -102; net/if_ppp.h -96; netinet/tcp.h -82; netinet/ip6.h -79;
  resolv.h -64; netinet/ip_icmp.h -53; inttypes.h -42; netinet/if_fddi.h -40; netinet/igmp.h -36; link.h -34; elf.h -31
  (GAPS-B's); arpa/nameser.h -21; sys/queue.h -13; lastlog.h/utmp.h -13; arpa/telnet.h -12. Families: SCSI (SCSI specs,
  sg(4)); ICMPv6, IPv6, TCP options, IGMP, FDDI (RFC 4443, 8200, 793/7323, 2236, IEEE 802); resolver/nameser (RFC 1035);
  PPP (RFC 1661). Full table: keke ~/work/headers-stage2-gaps-c/triage.tsv.

## Rulings / notes
* RULING NEEDED: R-H2 says "exactly the names external headers and programs test"; this story defines every probed
  guard of every header (213), not only tested ones. Recommend keep (same meaning, cheap; programs test unknown subsets).
* INFORMATIONAL: coreutils needs a complete `struct __locale_struct` (gnulib localename.c), see Jobs.
* INFORMATIONAL: form probe compiles C++ without a C++ library, so complex.h/tgmath.h C names show one-sided FORM-DIFF
  notes in cxx17 (pre-existing).
* Rule slips: leg 1: one remote here-string, one `sed -i`, one cat of a generated header; leg 3: one remote
  `cat > file` to install probe4.sh (later scripts by scp), one `find /` on keke (stopped by PID). None refused.
