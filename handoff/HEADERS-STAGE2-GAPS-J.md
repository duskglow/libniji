# HEADERS-STAGE2-GAPS-J: handoff (run 1, end of leg 1; REFRESH)

Branch task/headers-stage2-gaps-j, cut from origin/next dbb8c346f, no merge. HEAD 073ff35ea (items 1, 2, 3, 5).
Role keke:~/w/n1 (stock Ubuntu 24.04 glibc 2.39 in /usr/include; no cargo on the agent host: run every cargo on
keke through jobrun, pull libc-include back with `"${VSSH[@]}" 'cd ~/w/n1 && tar cf - libc-include' | tar xf -`).
Decision D-X.179 (records/decisions/D-X/D-X.179.toml). Job scripts keke `~/work/headers-stage2-gaps-j/`: p1.sh
(stock probe; stock output p1-stock.out), lay.py (DWARF layouts via abi_extract), gj1.sh (p1 vs libniji tree +
dup_values + header_check --only on the touched headers). Scratch copies in `~/w/.scratch/HEADERS-STAGE2-GAPS-J/`.

## Done (073ff35ea)
* 1 per-header values: spec/abi/macro-values.tsv (talkd DELETE 2, scsi ABORT 6, NOP 8), read by Gen::macro_row
  (values.rs), header_check.py P1 (ctx.header_values) and rows.rs (`macro-values` rows). Audit: spec/tools/dup_values.py
  [--include DIR] (1127A's dupval.py as a repo tool): keke probe1 `dupval: RESULT headers=56 same=6583 diff=0`.
* 2 headers/nested-tags.tsv (`struct whod` / `struct whoent`): aggregates.rs nests it, tag() emits the outer;
  header_check.py `nested_spelling` (P1 and GAPS-I offsets spell it through `wd_we[0]`).
* 3 ANYADDR `# define ANYADDR NULL` (timed.h.in). 5 TSPTYPES/tsptype, RIPCMDS/ripcmds tables in the templates.
* p1.sh on the generated tree: every item 1/2/3/5 line equal to stock.
* Fragment spec/tools/header_checks/s13.d/50-stage2-gaps-j.py (value, talkd switch, whod::whoent C++, ANYADDR,
  tables). Its first version had "replaced" units that cannot pass under -Werror (a redefinition warning); replaced
  by plain value units in 073ff35ea, NOT yet rerun.

## Probe-settled facts for item 4 (stock, keke probe p1.sh / lay.py)
* sizeof: ax25_parms_struct 48 align 2 (netax25/ax25.h and netrom/netrom.h), nr_parms_struct 36/4, ppp_stats 76/4.
  Layout rows printed by lay.py (`python3 lay.py ~/w/n1` on keke): ax25_parms_struct {port_addr ax25_address @0
  (+ port_addr.ax25_call char[7]), values short unsigned int[20] @8}; nr_parms_struct nine `unsigned int` @0..32
  (quality obs_count ttl timeout ack_delay busy_delay tries window paclen).
* Include closure: stock net/ppp_defs.h includes bits/types/time_t.h, <asm/types.h>, <linux/ppp_defs.h> (struct
  ppp_stats, pppstat, vjstat come from the kernel); net/if_ppp.h includes sys/types.h, stdint.h, net/if.h,
  sys/ioctl.h, net/ppp_defs.h and defines PPPIOC* itself (PPPIOCGASYNCMAP 2147775576, PPPIOCSXASYNCMAP 1075868751,
  PPPIOCSMAXCID 1074033745). netax25/ax25.h includes features.h and bits/sockaddr.h only; netrom/netrom.h includes
  netax25/ax25.h. SIOCAX25GETPARMS etc. need SIOCPROTOPRIVATE from elsewhere on stock too (not constant alone).
* libniji today: those three structs incomplete, no PPPIOC*, SIOCAX25*, SIOCNR* names at all (visibility.tsv has
  none: the name probe never had them as candidates).

## Exact next step (leg 2)
1. Rerun gj1.sh on keke (`jobrun headers-stage2-gaps-j-probe2`), expect header_check 0 failed.
2. Item 4: add header_probe.d/stage2-gaps-j.py candidates (PPPIOC* of the kernel's linux/ppp-ioctl.h names,
   SIOCAX25*/SIOCNR*/AX25_VALUES_*/NETROM_* names, tags ax25_parms_struct nr_parms_struct ppp_stats); run
   header_probe.py --headers net/if_ppp.h,netax25/ax25.h,netrom/netrom.h --names FILE --out DIR on keke; splice the
   new visibility rows; layouts.tsv rows from lay.py; macros.tsv rows (values by a black-box print, type int /
   unsigned); net/ppp_defs.h.in: add `#include <asm/types.h>` and `#include <linux/ppp_defs.h>`; SIOC* names
   whose stock form needs SIOCPROTOPRIVATE: define as `(SIOCPROTOPRIVATE + k)` in the template or leave out with a
   reason. Then kernel_spelled.py after `cargo xtask so`, regenerate, p1.sh again.
3. Proof jobs: units2-style llvm-19 sanitizer compile (keke `~/work/RELEASE-SNAPSHOT-1127A/units2.sh` shows the
   flags; source under its u2/pkgs) and an inetutils build against the system flavour (1127A ybuild on mei), the
   direct-tier gate (`cargo xtask affected --run`; if FULL escalated: precheck + changed crates' tests), plants
   (handoff/HEADERS-STAGE2-GAPS-J-plants.txt via `cargo xtask plants`), clippy + static, precheck, report json.

## Informational so far
* Leg 1 ran one `sed -i` by mistake on its own new fragment (docstring wording); result checked, .bak removed.
* A stray local `python3 -` waiting on stdin was started by mistake and killed by PID (2101156).
