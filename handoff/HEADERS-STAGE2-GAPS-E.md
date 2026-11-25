# HEADERS-STAGE2-GAPS-E: IPPROTO_MAX, tftphdr names, ns_msg/ns_rr accessors, fd_mask/NFDBITS, __u_char family, ftw.h S_IS*

**State (end of leg 4, run 1's last leg).** Branch `task/headers-stage2-gaps-e` from origin/next ecc8c95e3; role
keke:~/w/r1. Commits: 0253841b2 (probe + templates), 0cad32bf6 (values, includes, reserved emits, regen), 8302e1914
(@@needs-if fix + header_check rows + plants file), 603e40de3 (one fd_set macro text), 0b078aedc (its plants), + this
note and the report. Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-E/`; keke `~/work/headers-stage2-gaps-e/`
(stock/ reference probe table, niji2/ + cmp2.out, pkg/ package sources, pkgproof3.sh, regen.sh). Decisions in force:
D-11.4 (FD_* index through `__fdelt_chk`); no new rows; black-box probes only; no differences row.

## Done
* Legs 1-3: header_probe stage2e_names, visibility +1274 rows, templates, macros/typedefs/enumerators rows, reserved
  emits (R-H7), `<sys/time.h>` includes in utmp.h utmpx.h sys/procfs.h sys/profil.h sys/timex.h, `@@needs-if` counts
  a header's names only in the configurations where the include happens; header_check units for every premise name
  (s05, s07, s10, s12). Packages: keyutils, libcap-ng, libev, libnftnl, inetutils sources compile with libniji's
  headers (job pkgproof3, on 8302e1914; the later commits change only FD_* macro text).
* Leg 4 (603e40de3): the full header_check red (360 failed: "FD_SET redefined" when `<sys/time.h>` precedes
  `<sys/types.h>`; pre-existing on next for a program's own include order) is fixed at the cause: the seven templates
  that define FD_SET/FD_CLR/FD_ISSET/FD_ZERO (stdlib.h, utmp.h, utmpx.h, sys/time.h, sys/procfs.h, sys/select.h,
  sys/types.h) carry one text, `<sys/select.h>`'s (index via `__fdelt_chk`, D-11.4), each guarded. New s07 unit
  "fd_set macros A then B": each defining header before and after `<sys/types.h>`, every configuration, macros expanded.
  Two plants (fd-isset-time-division, fd-clr-stdlib-division).

## Jobs (keke)
* `headers-stage2-gaps-e-hc4` (603e40de3 tree): header_check --only new units + P5/P1 of the touched headers: 0 failed of 1053.
* `headers-stage2-gaps-e-xtask4`: `cargo test -p xtask` (includes the full header_check) on 0b078aedc.
* `headers-stage2-gaps-e-plants4`: 12 plants on 0b078aedc. Results: see the hand-back / plants file.
SUPERSEDED: regen1, values, values2, tpdbg, pkgproof, pkgproof2, xtask (red, fixed), plants (8302e1914), hc3.

## Not done: `<netinet/ip6.h>` (coordinator note 13:43Z) -> NEXT STORY
Larger than the note says: besides the extension-header structs (ip6_ext, ip6_hbh, ip6_dest, ip6_rthdr, ip6_rthdr0,
ip6_frag, ip6_opt, ip6_opt_jumbo/nsap/tunnel/router) and the IP6F_*/IP6OPT_*/IP6_ALERT_* constants, libniji's ip6.h
also lacks the `struct ip6_hdr` member-name macros (ip6_vfc, ip6_flow, ip6_plen, ip6_nxt, ip6_hlim, ip6_hops):
visibility.tsv lists only struct ip6_hdr, ip6_hdrctl, ip6_mtuinfo for it. The name list is RFC 3542 section 2 (an
allowed spec). Steps: add the names to header_probe (stage2e_names or a new set), probe the reference for visibility,
layouts (all structs) and macro values; rows in visibility/layouts/macros.tsv; template headers/netinet/ip6.h.in
(member macros like arpa/tftp.h's th_* ones; member_names in header_checks/s12.py handles any nesting depth); regen;
header_check rows (P1 if ip6.h has p1 1); plants; an nftables source file compiled with libniji's headers as proof.
Estimate: one full run (about 80-100 calls).

## Exact next step
If xtask4 and plants4 are green: dispatcher verifies; coordinator lands. If red: fix what it names, rerun only that suite.
