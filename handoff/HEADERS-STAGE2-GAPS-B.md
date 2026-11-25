# HEADERS-STAGE2-GAPS-B / -B-2 (Yohane stage 2: openssh, apt, binutils, krb5, systemd, apparmor, babeltrace, avahi; gnu-efi)

**State:** branch `task/headers-stage2-gaps-b-3` (continues `-b-2`), merged origin/next at 4acd0def6 (leg 6). Role
keke:~/w/n3. Scratch `~/w/.scratch/HEADERS-STAGE2-GAPS-B-3/`; on keke `~/work/headers-stage2-gaps-b-3/`. No new decision
rows (local, table-driven; forms settled by black-box probes recorded in the check modules).

## Leg 6 (HEADERS-STAGE2-GAPS-B-3)
* CI conflict from the -b-2 hand-back resolved by merging origin/next: `spec/tools/attr_probe.py` keeps next's
  configurations and nonnull unit (`-nn`) and this branch's sentinel/format_arg unit (`-s`, `-fno-builtin`: tests `s` and
  `fa`), plus the `format_arg` column (TABLE_COLUMNS); `spec/abi/diagattrs.tsv` is next's rows plus the `format_arg`
  column (gettext family) and the exec* rows without a sentinel (this branch's -fno-builtin result), merged by a row-wise
  three-way (not regenerated: ruling #8623 says a full run drifts on pure/const); `libc-include/libintl.h` regenerated.
* b64_ntop/b64_pton (the -b-2 follow-up): `b64_ntop`, `b64_pton` in header_probe's stage2_gaps_b_names (renamed from stage2_names, which the -C story also defines); the two probed rows
  (`macro`, `!time64only`) spliced into visibility.tsv; `__b64_ntop`/`__b64_pton` `emit` rows in headers/reserved.tsv;
  `#define b64_ntop __b64_ntop` (and pton) in headers/resolv.h.in; unit `initializers resolv.h b64` (s12.py: both function
  pointer types exact, a call); the two macros in form.py ENFORCED. Plants: handoff/HEADERS-STAGE2-GAPS-B-3-plants.txt.

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
* Jobs (keke, final head c232d5162): clippy3, gate3, plants3 (3 CAUGHT), stale. gate3's one red is xtask `so::tests::a_product_build_leaves_no_earlier_product_for_the_manifest_lint`: the gate runs it beside the products build and it lists target/niji while the build still writes it (not this story's code; passes alone on finished products: job stale; passed in gate2). Recommend the test or the gate order be fixed by its owner.
* Nothing open from this chain after leg 6 (the item below is DONE in leg 6; kept for the record).
* b64_ntop/b64_pton (coordinator 13:45Z): libniji's libresolv exports `__b64_ntop`/`__b64_pton`, visibility.tsv and
  prototypes.tsv list both as resolv.h functions, but libniji's resolv.h declares neither. The reference's resolv.h has
  `#define b64_ntop __b64_ntop` / `#define b64_pton __b64_pton` plus the declarations of `__b64_ntop`
  (const unsigned char *, size_t, char *, size_t) and `__b64_pton` (char const *, unsigned char *, size_t), both nothrow.
  Findings: headers/reserved.tsv has no `emit` row for the two `__b64_*` names (headers-gen omits `__x` names by default,
  as it does for `__res_init` before its emit row); the `b64_ntop`/`b64_pton` macros are in neither visibility.tsv nor macros.tsv
  (add them to header_probe's stage2_gaps_b_names, run `header_probe.py --headers resolv.h --names <file>`, merge the rows,
  add the macros to headers/resolv.h.in, a unit and a plant). Left to a follow-up story: it needs a table regeneration
  and a second gate, and this run's gate would be stale.
