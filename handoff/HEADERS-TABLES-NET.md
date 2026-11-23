# HEADERS-TABLES-NET: handoff and report

**State.** Branch `task/headers-tables-net` from origin/next 0495c4684; code head afe915972 (the report commit follows
it). Role keke:~/w/n2 (the reference system: Ubuntu GLIBC 2.39-0ubuntu8.9, GCC 13.3). Leg 1 probed visibility, leg 2
did the rest.

**What landed on the branch.**
* Candidates: `spec/tools/header_probe.py` `net_names()` (names and tags from RFCs, man pages and kernel UAPI headers,
  the docstring cites each group). `spec/tools/abi_extract.py`: the new tags in TYPES.
* Tables (every value from a black-box probe of the reference; `spec/abi/PROVENANCE` last paragraph):
  visibility.tsv +617 rows (the 13356 committed rows of the 35 slice 13 headers reproduced unchanged); macros.tsv and
  constants.tsv +490 integer macros (type_probe.py; the 418 rows it gave for names already in the tables identical);
  typedefs.tsv +25 struct tags; layouts.tsv +275 rows (the 25 tags, and struct icmp_ra_addr's own rows).
  enumerators.tsv unchanged. Per header: arpa/ftp.h 26, arpa/telnet.h 151, arpa/tftp.h 12 + struct tftphdr,
  net/if_slip.h 5, net/ppp-comp.h 27, net/route.h 38 + struct rtentry, in6_rtmsg; net/if_arp.h struct arpreq,
  arpreq_old, arpd_request; netinet/if_ether.h struct ether_arp; netinet/ip.h 93 + struct ip_timestamp;
  netinet/ip_icmp.h 78; netinet/icmp6.h 60 + 17 structs.
* Generator fixes the new rows needed (no template changed):
  - `xtask/src/headers/aggregates.rs`: a nested aggregate whose natural size would run past the outermost aggregate's
    recorded size is emitted packed (struct tftphdr: 5 bytes, its union at offset 2 holds char[3] and a 2-byte struct).
  - `xtask/src/abi.rs` (abi-gen): the same rule for nested composites (natural size > the bytes the table leaves);
    a bit-field group is placed at the byte of its first bit (`bits W@P`), not its storage unit's offset (struct
    ip_timestamp: two 4-bit fields from bit 24 of the unit at 0, after the uint8_t members at 1 and 2).
  - Neither changes any existing generated file (headers-gen: only the 11 slice 13 headers and the two abi files
    changed; the abi diff's only deletions are struct icmp's inline id_radv type, now struct_icmp_ra_addr).
* Regenerated: 11 headers under libc-include/, crates/abi/src/layout.rs, crates/abi/tests/appendix_c_layouts.rs.

**Proof.** header_check.py on the final tables (job htn-hgen3): 0 failed of 16348 (the first run, htn-hgen1, failed 39:
all P1 sizeof(struct tftphdr), fixed above). appendix_c_layouts 277 passed and `cargo xtask clippy` clean (htn-abi2;
htn-abi1 failed abi_struct_ip_timestamp, fixed above). `cargo xtask static` green (htn-static1). Plants
`handoff/HEADERS-TABLES-NET-plants.txt`, run htn-plants1: 6 CAUGHT (never-packed C and Rust, attribute dropped,
natural size unrounded, bit-field byte order, macros/constants cross-check), 1 SURVIVED: htn-nested-always-packed
(pack every nested aggregate): no recorded offset or size moves and no table records a nested member's alignment, so
it is now `equivalent:` with that reason. htn-plants2 (final tree, keke, rc 0): 6 CAUGHT, 0 SURVIVED, 1 EQUIVALENT,
no WARN-RC-ONLY / WARN-NO-MSG (one WARN-DEV-SHM: the copy is under /dev/shm; no plant here mounts its own /dev).
Direct gate htn-gate1 (keke, rc 0): `affected: ok`, every direct suite ok (niji native and interposed, niji-abi,
niji-boundary, niji-legacy, niji-math, niji-rt, conformance 97 programs, fuzz-check, static); peak 12G, oom_kill=0.
**Check gap (INFORMATIONAL):** header_check/P1 and appendix_c_layouts cannot see the alignment of a nested member;
an over-packed nested aggregate whose offsets happen not to move passes every check.

**Brief step 3: spec names the reference does not define** (probe negatives: in no header in any configuration).
99 of 598 macro candidates: AUTHTYPE_RSA/SRA/SSL, AUTH_ENCRYPT_MASK/OFF, COMP_PROTO_LEN, ENV_VALUE, ENV_VAR, EOPTNEG,
OACK, ICMP6_DST_UNREACH_NOTNEIGHBOR/POLICY/REJECT, ICMP6_MEMBERSHIP_QUERY/REPORT/REDUCTION, ICMP_ALTHOSTADDR,
ICMP_FILTER, ICMP_PARAMPROB_ERRATPTR/LENGTH, ICMP_ROUTERADVERT_NORMAL/NOROUTE_COMMON, IP6_RT_PRIO_*, IPOPT_SECUR_PROG,
IPTOS_CE, IPTOS_ECT, IPTOS_DSCP_CS0..7, ND_OPT_ADV_INTERVAL/ADVINTERVAL/HOMEAGENT_INFO/ROUTE_INFO/RDNSS/DNSSL,
ND_OPT_PI_FLAG_ROUTER, ND_RA_FLAG_PRF_*, RPM_PCO_MAX, RTCF_* (all but MASQ, NAT, DOREDIRECT, DIRECTSRC), RTF_ANYCAST,
EXPIRES, MAC, PCPU, PREFIX_RT, PREF_MASK, ROUTEINFO, SLC_EBOL/ECR/EEOL/EWR/INSRT/MCBOL/MCEOL/MCL/MCR/MCWL/MCWR/OVER,
SL_MODE_ADAPTIVE/AX25/CSLIP6/SLIP6, TELOPT_CHARSET/COMPORT/ENVIRON/FIRST/LAST/FORWARD_X/KERMIT/RSP/SEND_URL/SLE/
STARTTLS/TN3270E/XAUTH. 4 of 30 tags: compressor, nd_opt_route_info, nd_opt_rdnss, nd_opt_dnssl.
**Names the reference defines that no spec lists:** this method cannot find them (only candidates are probed, and
reading the headers' text is ruled out), so none are reported; the candidate sources are the ones cited in net_names().

**Open gaps (follow-up story).** Names the reference shows that are no integer constant, so no row: function-like
or member-access macros (IPTOS_TOS(), IPTOS_PREC(), ICMP_INFOTYPE(), ICMP6_FILTER_SETPASS() and kin, the icmp6_* /
nd_* / mld_* member macros, ETHER_MAP_IP_MULTICAST()), ICMP_TSLEN, and SIOCSKEEPALIVE, SIOCGKEEPALIVE, SIOCSOUTFILL,
SIOCGOUTFILL, SIOCSLEASE, SIOCGLEASE (net/if_slip.h). Each needs a template definition (tables cannot spell them).

**INFORMATIONAL.** `abi_extract.py --layouts-only` on this tree does not reproduce the committed layouts.tsv outside
the new types (members of _fpstate, _libc_fpstate, __sigset_t, La_x86_64_regs, the place of struct res_sym, the
hand-respelled Elf*_Phdr p_type rows): left as they are; only the new types' rows were spliced.

**Not to redo.** The probes and splices (all additions verified). Scratch scripts: `~/w/.scratch/HEADERS-TABLES-NET/`
(tpdiff.py, splice_lay.py, dropvis.py, negatives.py).

**Status (leg 3, done).** Code head afe915972; later commits are handoff/ only. JOBS: keke:htn-gate1,
keke:htn-plants2 (both rc 0). SUPERSEDED: htn-plants1 (the plants file changed after it: one plant marked
equivalent). The plants results are committed in `handoff/HEADERS-TABLES-NET-plants.txt`. Nothing left running.
**Next step:** landing (coordinator); then a follow-up story for the template definitions listed under Open gaps.
