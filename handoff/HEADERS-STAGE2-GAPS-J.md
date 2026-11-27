# HEADERS-STAGE2-GAPS-J: handoff (run 1, end of leg 2; REFRESH)

Branch task/headers-stage2-gaps-j, cut from origin/next dbb8c346f, no merge. HEAD 5b38c470c (items 1-5 coded).
Role keke:~/w/n1 (stock Ubuntu 24.04 glibc 2.39 in /usr/include; the role dir is NOT a git tree; no cargo on the
agent host: run every cargo on keke through jobrun; pull generated files back with
`"${VSSH[@]}" 'cd ~/w/n1 && tar cf - <paths>' | tar xf -` from the worktree). Decision D-X.179.
Job scripts keke `~/work/headers-stage2-gaps-j/`: p1.sh (stock probe; p1-stock.out), lay.py/lay2.py (DWARF layouts),
gj1.sh (p1 vs libniji tree + dup_values + header_check --only), pj2.sh (header_probe --names), pj3.sh (type_probe,
feature_probe forms, s.c/p.c black-box prints), gen2.sh (headers-gen, so, kernel_spelled.py, headers-gen).
Scratch copies and merge scripts (splice_vis.py, merge_tabs.py) in `~/w/.scratch/HEADERS-STAGE2-GAPS-J/`.

## Done
* Leg 1 (073ff35ea): items 1 (macro-values.tsv + dup_values.py), 2 (nested-tags.tsv, whod::whoent), 3 (ANYADDR),
  5 (TSPTYPES/RIPCMDS tables). Leg 2 probe2 (`headers-stage2-gaps-j-probe2`): header_check 0 failed of 1016,
  dupval diff=0, p1 item 1/2/3/5 lines equal stock.
* Leg 2 item 4: da93299eb header_probe.d/stage2-gaps-j.py candidates; 5b38c470c tables + templates:
  visibility.tsv +266 rows (net/if_ppp.h, net/ppp_defs.h, netax25/ax25.h, netrom/netrom.h, netrose/rose.h),
  macros.tsv +107, typedefs.tsv +18, layouts.tsv +222 (16 structs), macro-forms.tsv +21, kernel-spelled.tsv rerun.
  Templates: netrom.h and rose.h include netax25/ax25.h; ppp_defs.h includes asm/types.h + linux/ppp_defs.h
  (@@owns the kernel's tags, ext_accm, NPmode, NPMODE_*, PPP_ADDRESS/CONTROL/PROTOCOL); if_ppp.h includes
  net/ppp_defs.h, owns ifr__name (b.ifr_ifrn.ifrn_name) and stats_ptr (b.ifr_ifru.ifru_data); ax25.h owns
  SIOCAX25* = (SIOCPROTOPRIVATE+0..11) and sax25_uid = sax25_ndigis; netrom.h owns SIOCNR* (+0..4).
  Probe facts: values and parenthesised forms with SIOCPROTOPRIVATE 0 (s.c), offsetof ifr__name 0 / stats_ptr 16.
  headers-gen passes (job headers-stage2-gaps-j-gen3), `cargo xtask so` passed in it.

## Not verified yet / to check
* PPPIOC* are emitted from macros.tsv as literals (unsigned long), as MTIOCTOP is; stock's are not usable in `#if`
  (replaced-form): a report-mode FORM-DIFF, like MTIOCTOP. The generator refuses a template `_IOR` spelling of a
  macros.tsv name. Mention as INFORMATIONAL (a form-of-ioctl-requests follow-up), not a blocker.
* kernel-spelled.tsv rerun added FDDI_* and IGMP_* rows (libc-include/netinet/if_fddi.h, igmp.h changed: #ifndef
  guards). Check that header_check passes on them; they are the tool's output, the committed table lacked them.
* headers/dup-listed.tsv: netrom/rose now include ax25.h, so AX25_* may be defined in fewer files; header_check's
  dup module says whether the list must shrink (regenerate it with its tool if so, never hand-edit).

## Exact next step (leg 3)
1. Update p1.sh's item-4 lines if needed (its C++ `sizeof struct` probe is malformed on both sides: use a variable),
   add the sanitizer's names (11 PPPIOC*, SIOCAX25*/SIOCNR* with `#include <sys/ioctl.h>` first), then run gj1.sh as
   job `headers-stage2-gaps-j-probe3` (needs target/niji/include: `cargo xtask so` already ran on keke). Expect 0
   differing lines and header_check 0 failed; add header_check rows for item 4 to
   spec/tools/header_checks/s13.d/50-stage2-gaps-j.py (sizes, members, ioctl values, closure) if P1 does not cover them.
2. Proof jobs: llvm-19 sanitizer unit (keke ~/work/RELEASE-SNAPSHOT-1127A/units2.sh shows flags; it needs a
   system-flavour libc6-dev .deb: use the tree's target/niji/include with the same -nostdinc flags instead, or build
   the debs if a tool exists) and inetutils against the system flavour (1127A ybuild on mei; read its scripts).
3. Direct-tier gate (`cargo xtask affected --run`; if FULL escalated: precheck + changed crates' tests), plants
   (handoff/HEADERS-STAGE2-GAPS-J-plants.txt via `cargo xtask plants`, RAM dir /dev/shm/claude/headers-stage2-gaps-j/
   plants-<n>), clippy + static, precheck, report json, handback-check.

## Informational so far
* Leg 1 ran one `sed -i` by mistake on its own new fragment (docstring wording); result checked, .bak removed.
* A stray local `python3 -` waiting on stdin was started by mistake in leg 1 and killed by PID.
* Jobs so far (all on keke): headers-stage2-gaps-j-probe2, -pj2, -pj3, -sprobe, -gen2 (red: template `_IOR` refused,
  superseded), -gen3 (green).
