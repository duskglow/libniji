# HEADERS-NET-SYS-NAMES: the header names a static busybox still misses (leg 1 handoff)

**State (leg 1, 2026-11-25).** Branch `task/headers-net-sys-names` from origin/next 855efa170 (HEADERS-BUSYBOX-GAPS
landed). Role keke:~/w/n2 (the reference system). Scratch: `~/w/.scratch/HEADERS-NET-SYS-NAMES/` (local: nsprobe.py,
probe1.sh, merge_vis.py), `~/work/headers-net-sys-names/` on keke (same scripts, outputs in `p1/`). No decisions yet;
everything follows black-box probes. HEADER (arpa/nameser.h) is HEADERS-APT-GAPS': left alone.

## Commits
* leg 1: header_probe.py `net_sys_names()` (candidates, wired into candidates()); abi_extract.py HEADERS +scsi/scsi.h,
  scsi/sg.h, TYPES + the new tags.

## Probe results so far (job headers-net-sys-names-probe-1, outputs keke `~/work/headers-net-sys-names/p1/`)
* header_probe --names: 461 new visibility rows, 0 changed (p1/new-vis.tsv; already merged into the copy
  p1/t/spec/abi/visibility.tsv). Per header: scsi/scsi.h 126, netinet/tcp.h 107, scsi/sg.h 64, netinet/icmp6.h 39,
  netinet/ip6.h 34, sys/personality.h 33 (PER_* and flags are enumerators), netinet/ip_icmp.h 17, mntent.h 11,
  scsi/scsi_ioctl.h 8, netinet/if_ether.h 5 and netinet/ether.h 5 (arp_*), sys/syslog.h 3 + syslog.h 3 (LOG_FAC,
  LOG_PRI, LOG_MAKEPRI: function-like, need template definitions), sys/shm.h 2 (shminfo, shm_info), sys/msg.h 2
  (msginfo, msg_cbytes), sys/sem.h 1 (seminfo), net/route.h 1 (rt_mss). Member macros are visible in
  gnu17,gnu2x,default,bsd,svid,gnu,lfs64,fob64,time64,dynstack,reentrant,fortify1-3,cxx* (check = where the tag is).
* abi_extract --layouts-only in p1/t: ok; `struct _code` (CODE: only under SYSLOG_NAMES) and tcp_diag_md5sig skipped.
* type_probe on the first-showing headers: p1/tp/macros.tsv, typedefs.tsv (752 macro rows incl. old ones: merge only
  names not yet in spec/abi/macros.tsv).
* Member-name form (nsprobe v1, p1/member-form.txt): every name is a macro; offsets/sizes known for all but struct icmp
  (its offset program failed). v1 missed inner tokens (GCC reports the error at the poison `#define` line): FIXED in
  nsprobe v2 (installed on keke), rerun needed. Known already: rt_mss -> rt_mtu; msg_cbytes -> __msg_cbytes;
  icmp6_pptr/mtu via icmp6_data32, icmp6_id/seq/maxdelay via icmp6_data16 (id/maxdelay index 0 at off 4, seq index 1
  at off 6); mld_*/nd_*_*/rr_* via the *_hdr member (type via icmp6_type, the rest presumably via icmp6_data*).
* SYSLOG_NAMES: v1 dump did not build (stdio after syslog.h); fixed in v2 (dump, terminating rows, the two-unit link,
  whether syslog.h alone with SYSLOG_NAMES compiles).
* Process note: one v1 compile did not capture stderr, so its diagnostic reached the job log; nothing from it is used
  (v2 captures every compile; facts come from exit statuses and the probe programs' own output).

## Exact next steps
1. On keke: `cd ~/w/n2 && python3 ~/work/headers-net-sys-names/nsprobe.py ~/work/headers-net-sys-names/p1/t/spec/abi/layouts.tsv ~/work/headers-net-sys-names/p1`
   (as a jobrun job); read member-form.txt (via= now lists every token) and syslog-names.txt. For struct icmp, offsets
   per name (the program builds per M if needed).
2. Splice into spec/abi: visibility rows (p1/new-vis.tsv, merge_vis.py), layouts (diff p1/t/spec/abi/layouts.tsv vs
   committed: additions only), typedefs and macros (new names only). Then `cargo xtask headers-gen`.
3. Templates: `@@owns` + `@@when <tag>` member macros in netinet/ip_icmp.h.in, icmp6.h.in, ip6.h.in, if_ether.h.in,
   net/route.h.in, sys/msg.h.in (as net/if.h.in); sys/syslog.h.in LOG_PRI/LOG_FAC/LOG_MAKEPRI and the SYSLOG_NAMES
   block (CODE, prioritynames, facilitynames, INTERNAL_*), per the probe.
4. Tests: generalize s12.py MEMBER_MACROS/member_names to every header (multi-level paths, indexed forms); SYSLOG_NAMES
   via a module `problems(ctx)` that builds and runs a program against ctx.inc (header_check.py hook) or compile units.
   Do NOT change header_check.py or xtask/src/headers.rs (precheck: shared generator = RULING NEEDED).
5. Rerun bbsweep (copy ~/work/headers-busybox-gaps/bbsweep.sh, R=~/w/n2, new RAM dir), static busybox build + applets,
   plants file handoff/HEADERS-NET-SYS-NAMES-plants.txt, clippy/static/precheck, report json, handback-check.
