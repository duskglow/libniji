# HEADERS-STAGE2-GAPS-E: IPPROTO_MAX, tftphdr names, ns_msg/ns_rr accessors, fd_mask/NFDBITS, __u_char family, ftw.h S_IS*

**State (end of leg 3, REFRESH; the last refresh of run 1).** Branch `task/headers-stage2-gaps-e` from origin/next
ecc8c95e3; role keke:~/w/r1. Commits: 0253841b2 (probe + templates), 0cad32bf6 (values, includes, reserved emits,
regen), 8302e1914 (@@needs-if fix + header_check rows + plants file), + this note. Scratch (local):
`~/w/.scratch/HEADERS-STAGE2-GAPS-E/`; keke: `~/work/headers-stage2-gaps-e/` (stock/ reference probe table, niji2/ +
cmp2.out libniji probe, pkg/ package sources, pkgproof3.sh). Decisions: none new; black-box probes only; no differences row.

## Done
* Legs 1-2: see commits (header_probe stage2e_names, visibility +1274 rows, templates, macros/typedefs/enumerators rows,
  reserved emits R-H7, sys/time.h includes in utmp.h utmpx.h sys/procfs.h sys/profil.h sys/timex.h, @@owns-macro).
* Leg 3 (8302e1914): `xtask/src/headers/slices/s12.rs` `@@needs-if NAME H` counts H's names as supplied only in NAME's
  configurations (sys/wait.h/wait.h now show the __u_char family + __FD_SETSIZE in the strict modes, as the reference;
  only libc-include/sys/wait.h changed). header_check rows: s05 (__u_char family, sys/types.h exact), s07 (sys/select.h
  fd_mask family; sys/wait.h, wait.h __u_char family), s10 (ftw.h S_IS* exact), s12 (tftphdr member names: member_names
  generalised to any nesting depth; `nameser_accessor_units`: C and C++ types as prvalues from black-box probes, incl.
  C `char *` vs C++ `const char *` for ns_rr_name; statement form; C++17 constexpr values; `ipproto_max_units`).
* Probe compare after leg 3: only pre-existing rows remain (complex.h 5 configs; sys/vm86.h; time64only include-errors
  of arpa/ftp.h, arpa/telnet.h, arpa/tftp.h, net/if_slip.h, net/ppp-comp.h, netpacket/packet.h (+2), already in leg 1's
  pre-change table; u_char/fd_mask family extra in net/ppp_defs.h, netpacket/packet.h, utmpx.h; __FD_SETSIZE extra in
  sys/kd.h). All INFORMATIONAL, not this story.
* libresolv: ns_initparse, ns_parserr, ns_msg_getflag, ns_name_uncompress, ns_skiprr exported (docs/codemap/symbols.tsv).
* Jobs on 8302e1914 (keke): `headers-stage2-gaps-e-hc3` header_check --only new units 0 failed of 429;
  `headers-stage2-gaps-e-pkgproof3`: keyutils dns.afsdb.c, libcap-ng utils/filecap.c (stated 1-line config.h: no
  autotools on the host), libev ev.c (ev_select.c), libnftnl src/obj/ct_timeout.c, inetutils libinetutils/tftpsubs.c all
  rc 0 with libniji's headers (-nostdinc, libc-include, gcc include, kernel UAPI only) and with stock;
  `headers-stage2-gaps-e-plants`: 10 plants, 9 CAUGHT, 1 WARN-NO-MSG (ftw-no-sys-stat: headers-gen refuses the template
  first, "ftw.h: S_ISSOCK has no macros.tsv value", 13 problems; a real catch one step earlier than header_check).
* `cargo xtask affected` = FULL (template/table inputs): gate = precheck + `cargo test -p xtask` (FULL: the train's).

## RED to fix first (job headers-stage2-gaps-e-xtask, rc 101): full header_check 360 failed of 20388
Cause: leg 2's `#include <sys/time.h>` (line 7) precedes `<sys/types.h>` (line 8) in utmp.h, utmpx.h, sys/procfs.h,
sys/profil.h, sys/timex.h (+ lastlog.h, proc_service.h, thread_db.h through them). sys/time.h defines FD_SET/FD_CLR/
FD_ISSET as `(fd) / __NIJI_FD_BITS (s)` and sys/types.h (unguarded) as `__fdelt_chk (fd)`: "FD_SET redefined"
under -Wsystem-headers -Werror (P5, P1, S16 offsets). PRE-EXISTING on origin/next: `#include <sys/time.h>` then
`<sys/types.h>` gives the same 3 redefinitions (gnu2x, gnu17, c17 -D_DEFAULT_SOURCE; base tree ~/wl/src/ecc8c95e3...).
Fix the cause, not the order: one FD_* definition text in every header that defines them (find which template/generator
spells each: `grep -rn "fdelt_chk\|FD_SET" headers/*.in headers/sys/*.in xtask/src/headers`), so either order is
identical text; add a header_check unit for `<sys/time.h>` then `<sys/types.h>` (and the reverse) in a few configs;
a plant on it. Then rerun `cargo test -p xtask` (job ~15 min).

## Coordinator note (13:43Z, adds to the brief): `<netinet/ip6.h>` extension-header structs
struct ip6_hbh, ip6_dest, ip6_rthdr, ip6_rthdr0, ip6_frag, ip6_opt*, and their constants (nftables). Today
headers/netinet/ip6.h.in is 17 lines (includes only); layouts.tsv has only struct ip6_mtuinfo. Needs: header_probe names
(add to stage2e_names), layout probe of the reference for the structs, values for IP6OPT_*/IP6F_*/IP6_ALERT_* etc.,
visibility/layouts/macros rows, template, regen, header_check rows (P1 covers sizes/offsets/values if ip6.h has p1 1 in
its slice tsv), plants. If it cannot fit, hand back with it as next step.

## Exact next steps
1. FD_* text fix above; regen (keke `sh ~/work/headers-stage2-gaps-e/regen.sh`), rsync -c libc-include back, commit.
2. ip6.h per the note (or report it as the next story).
3. Plants for the new code (append to the plants file; new `cargo xtask plants` job, new --dir), `cargo test -p xtask`
   job, clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) + `cargo xtask static`,
   `cargo xtask precheck --jobs <labels> --head <sha>`, report json handoff/reports/HEADERS-STAGE2-GAPS-E.json,
   `ops handback-check`. JOBS so far valid only for 8302e1914: hc3, pkgproof3, plants (rerun after code changes).
SUPERSEDED: regen1, values, values2, tpdbg, pkgproof, pkgproof2 (config.h missing), xtask (red, above).
