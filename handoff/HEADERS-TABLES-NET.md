# HEADERS-TABLES-NET: handoff (leg 1 ended by REFRESH, context)

**State.** Branch `task/headers-tables-net` from origin/next 0495c4684; role keke:~/w/n2 (keke is the reference
system: Ubuntu GLIBC 2.39-0ubuntu8.9, GCC 13.3). Scratch: `~/w/.scratch/HEADERS-TABLES-NET/` locally (scripts
`visdiff.py`, `splice_vis.py`, `s13headers.txt` = the 35 slice 13 headers comma-separated, `vis1.tsv`), and
`~/work/headers-tables-net/` on keke.

**Done (commit 4ba6c8181, WIP).**
* `spec/tools/header_probe.py`: `net_names()` (names + tags from RFCs / kernel UAPI, docstring cites each group),
  added to `candidates()`. Function-like and member-access macros (IPTOS_TOS(), ICMP6_FILTER_SETPASS(),
  icmp6_id, nd_rs_type, ...) were deliberately NOT made candidates: the generator needs a template definition for
  each; report them as a follow-up gap (see below).
* Job `htn-vis1` (keke, kind test, rc 0, 167 s): `header_probe.py --jobs 16 --headers <s13 list> --out
  ~/work/headers-tables-net/vis1`. Reproduced all 13356 committed rows of those headers (0 gone, 0 changed),
  626 added, spliced into `spec/abi/visibility.tsv` (+626 / -0).
* Added names: arpa/ftp.h 26 macros; arpa/telnet.h 151; arpa/tftp.h 13 + struct tftphdr; net/if_slip.h 11;
  net/ppp-comp.h 27; net/route.h 38 RTF_/RTCF_/RTMSG_ + struct rtentry, in6_rtmsg; net/if_arp.h struct arpreq,
  arpreq_old, arpd_request; netinet/if_ether.h (and netinet/ether.h, BSD set) struct ether_arp + the arp structs +
  ETHER_MAP_IP_MULTICAST (an `#ifdef` hit: probably FUNCTION-LIKE, needs a template definition or drop it from the
  candidates); netinet/ip.h 93 macros + struct ip_timestamp; netinet/ip_icmp.h 172 (ip.h names in the BSD set:
  ip_icmp.h includes ip.h only there; check the template's include) ; netinet/icmp6.h 60 + 17 structs.
* `spec/tools/abi_extract.py`: TYPES lists the new structs; HEADERS += net/route.h netinet/if_ether.h arpa/tftp.h.

**Exact next steps.**
1. `type_probe.py --headers <s13 list> --out ~/work/headers-tables-net/tp1` on keke (jobrun, after sync; it reads
   the spliced visibility.tsv). Splice only the NEW names' rows into `macros.tsv` and `typedefs.tsv` (struct rows of
   the new tags); a name with no value = function-like: drop it from net_names() and rerun the vis probe or delete
   its vis rows by the same splice. Add integer rows to `constants.tsv` too (S09 precedent: both tables).
2. Layouts: copy spec/ to a scratch dir on keke, `python3 spec/tools/abi_extract.py --layouts-only` there, check the
   old rows are unchanged, splice new layouts.tsv / enumerators.tsv rows. Update `spec/abi/PROVENANCE` (one
   paragraph, like the S09/S16 ones).
3. `cargo xtask headers-gen` (also runs abi-gen), check netinet/ip_icmp.h template include of netinet/ip.h
   (conditional?), header_check.py for s13 (see handoff/SL-HEADERS-S13.md for the command), clippy, static.
4. Plants (`handoff/HEADERS-TABLES-NET-plants.txt`: e.g. a changed macros.tsv value, a removed layouts row; command
   must run header_check after headers-gen, see SL-HEADERS-S13), gate (`cargo xtask affected`; generated headers
   select nothing: FULL is the train's), precheck, report json, handback-check.
5. Report (brief step 3): names the specs list the reference lacks (probe negatives: e.g. IPTOS_CLASS_* present but
   IPTOS_DSCP_CS*, TELOPT_TN3270E.., ICMP_UNREACH_* BSD names present?: compute from net_names() minus vis rows) and
   function-like/member macros not covered (follow-up story).

**Not to redo.** The visibility probe and splice (verified additions only).
