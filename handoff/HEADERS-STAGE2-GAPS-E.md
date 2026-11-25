# HEADERS-STAGE2-GAPS-E: IPPROTO_MAX, tftphdr names, ns_msg/ns_rr accessors, fd_mask/NFDBITS, __u_char family, ftw.h S_IS*

**State (end of leg 4, run 1's last leg).** Branch `task/headers-stage2-gaps-e` from origin/next ecc8c95e3; role
keke:~/w/r1. Commits: 0253841b2 (probe + templates), 0cad32bf6 (values, includes, reserved emits, regen), 8302e1914
(@@needs-if fix + header_check rows + plants file), 603e40de3 (one fd_set macro text), 0b078aedc (its plants), 0a602098c
(the __fdelt_chk assert), + this
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
  Two plants (fd-isset-time-division, fd-clr-stdlib-division) SURVIVED that unit (plants4): with every block guarded
  a drifted text causes no redefinition. 0a602098c: in C++11..C++23 the unit also asserts (constexpr search of the
  stringified expansion) that FD_SET, FD_CLR and FD_ISSET name `__fdelt_chk`; a hand-mutated copy of sys/time.h fails it.

## Jobs (keke)
* `headers-stage2-gaps-e-hc5` (0a602098c): header_check --only "fd_set macros": 0 failed of 468.
* `headers-stage2-gaps-e-xtask5`: `cargo test -p xtask` (includes the full header_check) on 5b4f8a757 (final code).
* `headers-stage2-gaps-e-plants5`: 12 plants on 5b4f8a757. Results: plants file / hand-back.
SUPERSEDED: regen1, values, values2, tpdbg, pkgproof, pkgproof2, xtask (red, fixed), plants (8302e1914), hc3,
hc4 (603e40de3), xtask4 (green, 0b078aedc, before the assert), plants4 (2 SURVIVED, fixed by 0a602098c).

## Leg 5 (HEADERS-STAGE2-GAPS-E-2): merge with next
Merged origin/next (621e2c0c7): the conflict in header_checks/s12.py kept next's per-header MEMBER_MACROS table and
member_names (HEADERS-NET-SYS-NAMES), with the tftphdr th_* names as its "arpa/tftp.h" entry; header_probe.py keeps
stage2e_names and net_sys_names. headers-gen after the merge rewrote nothing (idempotent). next already landed the
netinet/ip6.h work (member macros, ext-header structs), so the "Not done: ip6.h" item below is closed by next.
Jobs on 621e2c0c7: clippy and static clean; headers-stage2-gaps-e-2-xtask (cargo test -p xtask): 665 + others pass,
only xtask/tests/transcripts.rs the_committed_gencat_transcripts_hold fails (no built target/niji-test/gencat on the
role: an unbuilt product, no header involvement); headers-stage2-gaps-e-2-plants: see hand-back.

## Leg 6 (HEADERS-STAGE2-GAPS-E-3)
No code change. On 04e96f476: clippy, static clean; plants job headers-stage2-gaps-e-3-plants green; xtask2 and gate2 green after `cargo xtask so` (the gencat transcript test needs the built product). SUPERSEDED: headers-stage2-gaps-e-3-xtask, headers-stage2-gaps-e-3-gate (red on that test, product unbuilt).
