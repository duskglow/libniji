# HEADERS-STAGE2-GAPS-E: IPPROTO_MAX, tftphdr names, ns_msg/ns_rr accessors, fd_mask/NFDBITS, __u_char family, ftw.h S_IS*

**State (end of leg 1, REFRESH).** Branch `task/headers-stage2-gaps-e` from origin/next ecc8c95e3; role keke:~/w/r1 (the
reference system too). Head: the WIP commit 0253841b2 (+ this note). Scratch (local): `~/w/.scratch/HEADERS-STAGE2-GAPS-E/`
(incsweep.py, cmp.py, splice.py, names.py, s2e.names, stock/ and niji/ probe tables, cmp.out, form1.cc, stmt.c,
runform.sh, guard.c). keke: `~/work/headers-stage2-gaps-e/` (same probe files, stock.txt, form1-stock.out).
Decisions: none new; everything follows black-box probes; no differences row.

## Done (commit 0253841b2)
* header_probe.py `stage2e_names()` (the six families + names the include sweep found); `error_lines` compiles with
  `-ftrack-macro-expansion=0` (cause of type_probe's abort on arpa/nameser.h: `GETSHORT` is an object-like alias of
  `NS_GET16`, the bare-name test's error was located in the header; reproduced with tpfail.sh).
* visibility.tsv: +1274 rows from `header_probe.py --names s2e.names` on the reference (insert only, 0 changed rows).
* Templates: arpa/tftp.h th_* macros; arpa/nameser.h ns_msg_*/ns_rr_* and NS_GET16/32, NS_PUT16/32;
  arpa/nameser_compat.h GETSHORT/GETLONG/PUTSHORT/PUTLONG aliases; ftw.h includes sys/stat.h; sys/stat.h S_ISSOCK and
  S_TYPEISMQ/SEM/SHM under `@@if`, S_TYPEISTMO removed (reference shows it nowhere).

## Probe-settled (stock, keke, form1.cc/stmt.c/guard.c)
* ns_msg_id int, ns_msg_base/end `const unsigned char *`, ns_msg_size long, ns_msg_count int (index NOT range-checked:
  sentinel bytes read back), ns_rr_name `const char *` ("." for an empty name), ns_rr_type `ns_type`, ns_rr_class
  `ns_class`, ns_rr_ttl unsigned int, ns_rr_rdlen int, ns_rr_rdata `const unsigned char *`: all prvalues.
* NS_GET16 0x1234 adv 2; NS_GET32 0x56789abc adv 4; NS_PUT16(0xa1b2c3) writes b2c3; NS_PUT32(0x1122334455) writes
  22334455; GETSHORT/... the same; every one compiles as a statement before `else`.
* tftphdr: th_block u short @2, th_code short @2, th_stuff char[0] @2, th_data and th_msg char[0] @4.
* fd_mask long (8, signed); NFDBITS int 64; __u_char..__u_long the unsigned types, __quad_t long, __u_quad_t unsigned long.
* libresolv: stock exports ns_initparse, ns_parserr, ns_msg_getflag, ns_skiprr, ...; libniji's versions.tsv has them
  (crates/core/src/names/dns). Nothing to report.
* Include sweep (incsweep.py over visibility.tsv, then probes of the template-only names): stock-only gaps are ftw.h ->
  sys/stat.h (done); sys/time.h timer macros (timeradd...) through lastlog.h, proc_service.h, sys/procfs.h,
  sys/profil.h, sys/timex.h, thread_db.h, utmp.h, utmpx.h (20 configs; stock shows all of sys/time.h there in 39/40);
  st_atime/st_mtime/st_ctime in fcntl.h, mqueue.h, sys/fcntl.h, sys/file.h, sys/mount.h (22 configs). bswap_*, index,
  TCSETS2 were false positives (no difference).

## Exact next steps
1. Add `#include <sys/time.h>` after `#include <features.h>` in the 8 templates above (Edit tool).
2. st_atime family in fcntl.h, mqueue.h, sys/fcntl.h, sys/file.h, sys/mount.h: see how headers-gen handles their new
   macro rows (or copy sys/stat.h's block under `@@when`), match the 22 configs.
3. Values: job `headers-stage2-gaps-e-values2` (type_probe to ~/work/headers-stage2-gaps-e/types, enum_probe of
   netinet/in.h to enum-in.tsv). Splice rows for the new names only into macros.tsv/typedefs.tsv (fd_mask, __u_char..
   __u_quad_t, __NFDBITS, __FD_SETSIZE); enumerators.tsv `anon IPPROTO_AH  IPPROTO_MAX  <value>` (enumerator only, no macro).
4. headers/reserved.tsv `emit` rows: __u_char __u_short __u_int __u_long __quad_t __u_quad_t (libtirpc's rpc/types.h,
   every tirpc user), __fd_mask __NFDBITS __FD_SETSIZE (brief: the `__` forms).
5. `cargo xtask headers-gen` on keke, commit regen; re-run the niji probe (cmp.py) to 0 differences for s2e.names.
6. header_check rows (s12.py beside member_names: th_* member paths with the fake-structure form test; C++ static_assert
   units for the ns_* types/categories and a constexpr NS_GET/PUT evaluation; `if (c) NS_GET16(..); else` form).
7. Package proof on keke (sources: deb.debian.org reachable from keke; none local): libnftnl ct_timeout.c,
   inetutils tftpsubs.c, keyutils dns.afsdb.c (also check ns_t_afsdb), libev ev_select.c, libcap-ng utils/filecap.c,
   libnsl against tirpc; system flavour.
8. Plants file handoff/HEADERS-STAGE2-GAPS-E-plants.txt, plants job, clippy + static + precheck, report json.
