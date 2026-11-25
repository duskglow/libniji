# HEADERS-STAGE2-GAPS-E: IPPROTO_MAX, tftphdr names, ns_msg/ns_rr accessors, fd_mask/NFDBITS, __u_char family, ftw.h S_IS*

**State (end of leg 2, REFRESH).** Branch `task/headers-stage2-gaps-e` from origin/next ecc8c95e3; role keke:~/w/r1 (also
the reference system). Commits: 0253841b2 (probe + templates), 0cad32bf6 (values, includes, reserved emits, regen), + this
note. Scratch (local): `~/w/.scratch/HEADERS-STAGE2-GAPS-E/` (splice_vals.py, owns.py, regen.sh, nijiprobe.sh, values3.sh,
tpdbg.py, cmp.py, s2e.names, form1.cc, stmt.c, runform.sh). keke: `~/work/headers-stage2-gaps-e/` (same scripts; stock/ =
reference probe table; niji2/ + cmp2.out = libniji probe of the regenerated headers; types3/ = type_probe tables).
Decisions: none new; everything follows black-box probes; no differences row.

## Done
* Leg 1 (0253841b2): header_probe `stage2e_names()`; visibility.tsv +1274 rows (insert only); templates for tftp.h th_*,
  nameser.h ns_msg_*/ns_rr_*/NS_GET/PUT, nameser_compat.h GETSHORT..., ftw.h -> sys/stat.h, S_ISSOCK/S_TYPEIS* conditions.
* Leg 2 (0cad32bf6):
  - header_probe.py `Unit.run`: when an error is located outside the unit (GETLONG -> NS_GET32 used as a value is
    reported at the alias's definition line, even with -ftrack-macro-expansion=0), it bisects the tests until each such
    failure is one test; a unit failing with no tests still raises. This unblocked type_probe (job values3, 28 s).
  - macros.tsv + __FD_SETSIZE 1024, __NFDBITS 64; typedefs.tsv + fd_mask, __u_char, __u_short, __u_int, __u_long,
    __quad_t, __u_quad_t (only these: the probe's other new rows are enumerator-only names or _REG_* (not ours));
    enumerators.tsv `anon IPPROTO_AH IPPROTO_MAX 263` (enumerator only, as stock).
  - reserved.tsv emit: the __u_*/__quad_t family, __fd_mask, __NFDBITS, __FD_SETSIZE (reason R-H7).
  - `#include <sys/time.h>` in utmp.h, utmpx.h, sys/procfs.h, sys/profil.h, sys/timex.h (covers lastlog.h,
    proc_service.h, thread_db.h); st_atime/st_mtime/st_ctime blocks (`@@when st_atime`, `#ifndef`) in fcntl.h and
    sys/file.h; sys/stat.h's block gained `&& !defined st_atime`.
  - `@@owns-macro` for hand-written macros that now have visibility rows (headers-gen errored "has no macros.tsv value"):
    byteswap.h bswap_*, fcntl.h/sys/file.h/sys/pidfd.h st_*time, sys/stat.h S_IS*/S_TYPEISSEM/SHM/st_*time, sys/time.h timer*.
  - `cargo xtask headers-gen` (job regen2, 242 headers; 51 changed); libc-include/ committed (fetched with rsync -c).

## Probe compare after regen (keke cmp2.out, s2e.names only; reference vs libniji)
Pre-existing on origin/next too (leg 1 cmp.out; not this story): complex.h(5 configs) fd/u_char family; sys/vm86.h
include-error; u_char/quad_t/fd_mask family extra in net/ppp_defs.h, netpacket/packet.h, utmpx.h(20).
**Still open, look at first (step 1):** `__u_*`/`__quad_t`/`__FD_SETSIZE` missing in sys/wait.h and wait.h (14 configs);
extra in netpacket/packet.h(39) and sys/kd.h(40, __FD_SETSIZE); th_block/th_code/th_data/th_msg/th_stuff missing in
arpa/tftp.h in 1 config, and libniji include-errors in 1 config for arpa/ftp.h, arpa/telnet.h, arpa/tftp.h,
net/if_slip.h, net/ppp-comp.h, netpacket/packet.h (+2): check whether that config's include-error predates this branch
(re-run cmp.py on leg 1's niji table, keke `niji/visibility.tsv`); if it does, it is INFORMATIONAL, not ours.

## Exact next steps
1. Settle the open compare rows above (wait.h probably needs the reserved __ names through its includes in those 14
   configs; read `cmp2.out` in full, ~45 lines).
2. header_check rows (s12.py beside member_names: th_* member paths with the fake-structure form test; C++ static_assert
   units for the ns_* types/categories and a constexpr NS_GET/PUT evaluation; `if (c) NS_GET16(..); else` form;
   IPPROTO_MAX == 263 enumerator; fd_mask/NFDBITS; ftw.h S_ISDIR visible).
3. Package proof on keke (sources from deb.debian.org; none local): libnftnl or libnetfilter-conntrack failing file,
   inetutils tftpsubs.c, keyutils dns.afsdb.c (also ns_t_afsdb), libev ev_select.c, libcap-ng utils/filecap.c; system
   flavour. Plus the leg-1 form probe (runform.sh with libc-include) to confirm the forms.
4. Plants file handoff/HEADERS-STAGE2-GAPS-E-plants.txt (template plants: drop th_data, NS_GET16 advance, IPPROTO_MAX
   value, ftw.h include), `cargo xtask plants` job, gate (`cargo xtask affected --run`), clippy + static + precheck,
   report json handoff/reports/HEADERS-STAGE2-GAPS-E.json.

## Jobs so far (keke): headers-stage2-gaps-e-values3, -regen2, -nijiprobe2 (probes, not proof). SUPERSEDED: -values,
-values2 (type_probe abort, fixed), -regen1 (owns-macro errors, fixed), -tpdbg (diagnostic).
