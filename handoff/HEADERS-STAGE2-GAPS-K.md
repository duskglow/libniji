# HEADERS-STAGE2-GAPS-K: handoff (run 1, end of leg 1: findings and plan, no code yet)

Branch task/headers-stage2-gaps-k, cut from origin/next 16e03637e, no merge. Role shiki:~/w/sh1 (Skylake-SP class,
no clang there; clang-19 and the llvm-19 source are on keke: ~/work/RELEASE-SNAPSHOT-1127A/u2/pkgs/compiler-rt/...,
J's proof script keke ~/work/headers-stage2-gaps-j/pf1.sh builds the system-flavour debs and compiles the unit).
Job scripts and outputs: shiki ~/work/headers-stage2-gaps-k/ (p1.sh probe, j1.sh runner: `j1.sh TAG` runs p1.sh on
stock, libc-include and target/niji/include if built, then form.py; outputs TAG-stock.out, TAG-niji.out, TAG-form.diff).
Local copies of the scripts: the run's scratch dir (.scratch/HEADERS-STAGE2-GAPS-K/).

## Probe-settled (job headers-stage2-gaps-k-probe1, stock 2.39 vs libc-include at 16e03637e; C and C++)
* `<netipx/ipx.h>`: stock defines SIOCAIPXITFCRT, SIOCAIPXPRISLT, SIOCIPXCFGDATA, SIOCIPXNCPCONN as
  SIOCPROTOPRIVATE plus 0, 1, 2, 3, the expansion parenthesised (own `#define SIOCPROTOPRIVATE 1000` gives 1000+n,
  `2*X` gives 2*(1000+n)), type int, values 35296..35299 after `<sys/ioctl.h>` (either order), not usable with the
  header alone (SIOCPROTOPRIVATE not declared: same as libniji's AX.25 ones), defined outright (an earlier program
  definition is replaced). Same form as headers/netax25/ax25.h.in's `@@if SIOCAX25GETUID` block: copy that pattern.
  Stock also defines `sipx_special` (outright); libniji does not. NEXT: probe what it aliases (offsetof/sizeof of
  `.sipx_special` in struct sockaddr_ipx against each member) and emit it like ax25's `sax25_uid` block.
  IPX_MAX_PPROP_HOPS: equal on both sides (not defined). Include closure: stock ipx.h -> sys/types.h, stdint.h,
  bits/sockaddr.h; libniji -> features.h, sys/types.h, stdint.h (equal in effect).
  The rows also need visibility.tsv rows (netipx/ipx.h, macro, !time64only like the ax25 ones) and the
  header_probe.d/radio.py note ("Not candidates: SIOCAIPX*, SIOCIPX*") updated.
* `PPPIOC*` / `<net/if_ppp.h>`: stock's 18 PPPIOC* are not usable in `#if` and an earlier program definition is
  replaced (libniji: literals under `#ifndef`). With the kernel's `<linux/ppp-ioctl.h>` (or `<linux/if_ppp.h>` +
  `<linux/ppp_defs.h>`) in the same unit, either order, `-Wsystem-headers -Werror`: stock warns for SC_MASK only;
  libniji warns for 46 names (PPPIOC*, SC_*, SIOCGPPPSTATS/VER/CSTATS) when net/if_ppp.h comes first, none when the
  kernel's comes first (#ifndef). So stock spells every shared name except SC_MASK token-identically to the kernel's
  UAPI header (stock net/if_ppp.h includes, depth 2: sys/types.h, stdint.h, net/if.h, sys/ioctl.h, net/ppp_defs.h;
  not the kernel ppp header directly).
* FORM-DIFF class (form.py on libc-include, job probe1; `#if` usable in libniji only), the ioctl requests: 18
  PPPIOC* (<net/if_ppp.h>), BLKBSZGET BLKBSZSET BLKGETSIZE64 (<sys/mount.h>), MTIOCGET MTIOCGETCONFIG MTIOCPOS
  MTIOCSETCONFIG MTIOCTOP (<sys/mtio.h>): 26 macros. No other ioctl-like name in the other FORM-DIFF groups.
  Summary line: 9488 macros, 14962 pairs, 274 pairs differ (534 #if, 73 defined one side, 1158 expression one side).
* termios: `<termios.h>` + `<sys/ioctl.h>`, `-Wsystem-headers -Werror`: stock clean both orders; libniji
  termios-first: TIOCSER_TEMT and TTYDEF_SPEED redefined; ioctl-first: TTYDEF_SPEED. Cause: libniji spells
  TTYDEF_SPEED `B9600` in termios.h/pty.h/termio.h (macro-aliases.tsv rows) but `13` in sys/ioctl.h,
  sys/ttydefaults.h, sgtty.h (no alias row: B9600 not visible there); TIOCSER_TEMT `1` (kernel-spelled.tsv name; the
  kernel's asm-generic/ioctls.h, included by sys/ioctl.h, defines it in its own spelling). Stock: TTYDEF_* with
  `<sys/ioctl.h>` alone have no value (their expansion names termios constants: form.py audit group E1), stock
  termios.h includes bits/termios.h and sys/ttydefaults.h. TTYDEF_IFLAG/OFLAG/LFLAG/CFLAG are literal on every
  libniji header (no warning; E1 difference stays report mode).

## Plan (leg 2)
1. Generator: a way to emit a macro in the kernel UAPI header's spelling (kernel docs/UAPI headers are an allowed
   source; libniji ships them at target/niji/uapi). Proposed: new generator input `spec/abi/macro-spellings.tsv`
   (header, name, spelling) read like macro-values.tsv (xtask/src/headers.rs `header_values`, values.rs `macro_row`
   / `value()`), emitted outright (not under #ifndef) where stock replaces an earlier definition; rows for the 26
   request macros (spelled `_IOR('t', 90, int)` etc., the kernel's own text: check each header then has `_IOR`
   visible, i.e. includes sys/ioctl.h -> asm/ioctl.h), the SC_* and SIOCGPPP* shared names of net/if_ppp.h (all but
   SC_MASK), TIOCSER_TEMT (kernel spelling) in termios.h/pty.h/termio.h/sgtty.h/sys/ioctl.h. Check rows.rs (the rows
   test reads header_values) and dup_values.py / kernel_spelled.py interplay. A simpler alternative if it fits: keep
   these under the kernel's own definition by including the kernel header (NOT stock's closure: rejected).
2. TTYDEF_SPEED: one spelling everywhere (`B9600` where stock's has no value alone; macro-aliases.tsv rows for
   sys/ioctl.h, sys/ttydefaults.h, sgtty.h if the generator allows an alias not visible in the header).
3. IPX: template block + sipx_special + visibility rows.
4. header_check module spec/tools/header_checks/s13.d/60-stage2-gaps-k.py (pattern: 50-stage2-gaps-j.py): the
   termios+ioctl pair both orders; net/if_ppp.h with linux/ppp-ioctl.h both orders (no redefinition but SC_MASK?
   check how P5 handles -Wsystem-headers); IPX values after sys/ioctl.h; PPPIOC*/BLK*/MTIOC* not in `#if`.
   Form enforcement: add the 26 pairs to form.py ENFORCED.
5. Regenerate (cargo xtask headers-gen; spec/tools/kernel_spelled.py after `cargo xtask so`), rerun j1.sh -> 0
   DIFF; plants (docs/plants.md format) ; affected gate (expect FULL escalation: changed crates' tests); llvm-19 unit
   on keke (pf1.sh pattern, clang-19 and g++).

## Jobs so far
shiki: headers-stage2-gaps-k-probe1 (findings above; base tree, not a proof of the final head).
