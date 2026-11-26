# HEADERS-STAGE2-GAPS-G: handoff (run 1, end of leg 2: refresh)

Branch task/headers-stage2-gaps-g from origin/next dff0cc5c1; role keke:~/w/r5. Scratch: local
`~/w/.scratch/HEADERS-STAGE2-GAPS-G/` (k.sh = run a command on keke; lp.py line probe with config sets c/all/clang/
clv; cc.sh header compile check; syms.sh nm diff stock vs libniji; probe files f1 f3c f7 f8 f9 f10 cl.c; addfrag.py,
owns.py). keke `~/work/headers-stage2-gaps-g/` (same scripts, `kern/` UAPI links, `niji-inc/` copy of libc-include:
refresh it with `rm -rf niji-inc && cp -r ~/w/r5/libc-include niji-inc` after each headers-gen). No cargo locally:
headers-gen runs on keke, then pull `libc-include` back with
`"${VSSH[@]}" 'cd ~/w/r5 && tar cf - libc-include' | tar xf - -C <worktree>` BEFORE the next sync (a sync copies the
worktree's stale generated headers over the role's). No decision rows; black-box probes only.

## Done (commits 072b0f243 2c2f8cc2f, FTS64 commit, 4997f30fc, 77c3c4b0c; all pushed)
1. strerror_r XSI form: headers/redirects.tsv + functions.rs redirect_line: int, nonnull(2), no wur (probe: matches).
3. resolv.h renames: functions.rs parenthesizes `(__x) (` for `#define x __x` targets (probe: matches).
2. __BYTE_ORDER family, __SIGRTMIN/MAX, __toupper_l/__tolower_l (reserved.tsv emit rows; __toupper_l also a
   self-calling macro under fortify, ctype.h.d frag); __bswap_16/32/64 static inline, one guarded fragment in
   byteswap.h endian.h sys/types.h stdlib.h netinet/in.h (probe: type, values, not a macro: match).
4. sys/fanotify.h -> linux/fanotify.h, net/ethernet.h -> linux/if_ether.h, netinet/if_fddi.h -> linux/if_fddi.h,
   @@owns of the kernel's struct tags (probe: ethhdr, fanotify_event_metadata, fddihdr, FDDI_K_*: match).
5. FTS64/FTSENT64: abi_extract.py TYPES gains them; the 32 rows of its --layouts-only run spliced into layouts.tsv
   (full regen churns curated rows: not taken). Probe: size, align, every offset, member types: match.
6. clang fortify (s18.rs): $V/mode wrappers under `!defined __clang__`; clang C gets `#define f(__p0.., ...)` to the
   _chk route (clang_macro). Probe (clang 19 C/C++, gcc, levels 1-3): macro-ness matches; nm -u symbols identical.
7. obstack 7 macros (obstack.h.d/010-begin.frag), CTRL/CCEQ (frags in 6 tty headers), PTRACE_O_*/EVENT_* (`@@enum TAG
   plain`, new marker arg in enums.rs: stock has no macros of those names), SHADOW, ELF_NOTE_*/NT_GNU_*, IGMP, __*_t
   (already in next via GAPS-F; probe: match).
* header_check fragments s05.d s08.d s09.d s10.d s12.d s13.d s14.d /20-stage2-gaps-g.py: `--only` run 0 failed of 367.
  s05.d fragment also runs alone (`python3 spec/tools/header_checks/s05.d/20-stage2-gaps-g.py`): the clang check.
* Plants: handoff/HEADERS-STAGE2-GAPS-G-plants.txt (16). clippy 0, static 0, headers-gen --check ok (keke).

## Jobs running (started 17:05Z on keke, head 77c3c4b0c)
* `headers-stage2-gaps-g-plants-1` (plants, dir /dev/shm/claude/headers-stage2-gaps-g/plants-1)
* `headers-stage2-gaps-g-test-1` (test: cargo test -p xtask; niji-abi appendix_c_layouts; full header_check)
`cargo xtask affected --list --base origin/next`: FULL escalated (generator inputs) -> "FULL: the train's".

## Exact next steps
1. `~/bin/jobwait headers-stage2-gaps-g-plants-1 headers-stage2-gaps-g-test-1` on keke; read ~/logs/<label>.log
   (summaries, not whole). Fix what fails; rerun only the failing part as a new labelled job.
2. Package proofs (blocking in the brief): logs `/srv/aqours/logs/stage2/<src>_<ver>.log.zst` (read-only) name the
   failing units; compile each against the system flavour (cargo xtask so -> target/niji) as a job: systemd/elogind
   sparse-endian.h, eudev __bswap_16, openssh base64.c, liburing fanotify, iptables ethhdr, fakechroot fts64.c, libssh
   strerror_r, glib2.0 ptrace, llvm-toolchain-19 unit with clang. If sources are not on keke, say so (NOT VERIFIED).
3. Precheck on the synced final head: `cargo xtask precheck --jobs <labels> --head <sha>`; report json
   handoff/reports/HEADERS-STAGE2-GAPS-G.json; `ops handback-check`.

## INFORMATIONAL for the report (found, not fixed here)
* TTYDEF_SPEED is B9600 in termios.h and 13 in sys/ioctl.h / sys/ttydefaults.h: including both families conflicts
  (-Werror). termios.h before the kernel's asm/ioctls.h (termio.h includes it) also conflicts in header_check's UAPI
  tree. Units here avoid both orders.
* Systemic: 147 functions have warn_unused_result on stock only under fortify, on libniji always (diagattrs.tsv from
  the fortify=3 base); nothrow/leaf absent in C. Next story: attr_probe per-config wur column + `__NIJI_WUR`.
* __asprintf (reserved, undeclared on libniji) is a clang macro on stock. obstack_empty_p's C++ type not verified.
  A program's 0-argument `__res_init()` macro: stock errors, libniji compiles.
* Sweep item 1: gnu C getitimer/setitimer/getrlimit/setrlimit(64) take enum types on stock; getumask, bindresvport6,
  _argp_input, _option_is_short undeclared on libniji. Item 4 sweep (kinc.out on keke): stock-only kernel closures
  beyond the three fixed: errno.h -> linux/errno.h (19 headers), linux/limits.h, falloc, close_range, sockios, param,
  ppp, if_slip, stat, quota, prctl, sysinfo, rseq, atalk: listed, not changed.
