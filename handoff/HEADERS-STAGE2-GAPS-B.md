# HEADERS-STAGE2-GAPS-B / -B-2 (Yohane stage 2: openssh, apt, binutils, krb5, systemd, apparmor, babeltrace, avahi; gnu-efi)

**State:** branch `task/headers-stage2-gaps-b-2` (continues `task/headers-stage2-gaps-b`), merged origin/next at e64b49f2a
(HEADERS-NET-SYS-NAMES landed). Role keke:~/w/n3. Scratch `~/w/.scratch/HEADERS-STAGE2-GAPS-B-2/`; on keke
`~/work/headers-stage2-gaps-b-2/`. No new decision rows (local, table-driven; forms settled by black-box probes recorded
in the check modules).

## Done
* Leg 1-4 (HEADERS-STAGE2-GAPS-B): openssh premise corrected (the reference's `<signal.h>` declares the exec family too; the
  break was libniji's `__sentinel__` spelling; libniji declares none); AF_x as PF_x and aliases as aliases
  (macro-aliases.tsv, aliases check: 0 differences for sys/socket.h); `__RES`; RTLD_DI_*/RTLD_DL_* + Dl_serinfo/Dl_serpath;
  IN6ADDR_*_INIT; ELF 32/64 type pairs; UDP_*/SOL_UDP; ELFMAG; the 20 ELF R_/ST_/M_ accessors; format_arg on the gettext
  family. Units: s10 format_arg, s12 initializers, s16 dl_requests/elf_info; 9 plants in
  handoff/HEADERS-STAGE2-GAPS-B-plants.txt.
* Leg 5 (this run, B-2): merged origin/next (conflicts in s12.py and header_probe.py: both sides kept;
  appendix_c_layouts.rs regenerated with `cargo xtask abi-gen`, never hand-merged).
* Ruling #8579: probed stock `dlinfo` on RTLD_DI_CONFIGADDR, RTLD_DI_PROFILENAME and RTLD_DI_PROFILEOUT with the main
  program's handle: all three return -1 with dlerror "unsupported dlinfo request" (pointer untouched). libniji refuses
  them the same way (-1 + dlerror; gate-run dl tests cover the served requests). Equal: nothing more to do, they stay declared.
* Ruling #8568: openssh 10.0p1 `make -k ssh sshd` against this branch's headers merged with next: rc 0, 0 failing files,
  `ssh` and `sshd` linked (misc.c's TCP_NODELAY now comes from HEADERS-NET-SYS-NAMES; getrrsetbyname.c's HEADER from
  HEADERS-APT-GAPS-2). The tarball from leg 2 was reused, no new download. netinet/udp.h UDP_* were already in (leg 4).

## Jobs (keke, final head)
gate (affected direct tier: 18 suites ok, 11811 passed / 0 failed, conformance 103/0, clippy and static clean), plants (9
plants), proof6 (openssh above). Earlier legs' jobs are superseded by these (see the report).

## Not done / next step
* b64_ntop/b64_pton (coordinator 13:45Z): libniji's libresolv exports `__b64_ntop`/`__b64_pton`, visibility.tsv and
  prototypes.tsv list both as resolv.h functions, but libniji's resolv.h declares neither. The reference's resolv.h has
  `#define b64_ntop __b64_ntop` / `#define b64_pton __b64_pton` plus the declarations of `__b64_ntop`
  (const unsigned char *, size_t, char *, size_t) and `__b64_pton` (char const *, unsigned char *, size_t), both nothrow.
  Findings: headers/reserved.tsv has no `emit` row for the two `__b64_*` names (headers-gen omits `__x` names by default,
  as it does for `__res_init` before its emit row); the `b64_ntop`/`b64_pton` macros are in neither visibility.tsv nor macros.tsv
  (add them to header_probe's stage2_names, run `header_probe.py --headers resolv.h --names <file>`, merge the rows,
  add the macros to headers/resolv.h.in, a unit and a plant). Left to a follow-up story: it needs a table regeneration
  and a second gate, and this run's gate would be stale.
