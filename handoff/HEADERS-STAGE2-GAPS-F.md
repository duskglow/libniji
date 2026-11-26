# HEADERS-STAGE2-GAPS-F: handoff (leg 1 of run 1, REFRESH)

Branch task/headers-stage2-gaps-f from origin/next 05995ca18; role shiki:~/w/sh1 (synced at 4d268813a).
Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-F/` (probe.py, probe2.py, probe3.py, splice.py, reserved_rows.py,
vis2.tsv); shiki `~/work/headers-stage2-gaps-f/` (stock/probe.tsv, niji1/probe.tsv = libniji BEFORE the change,
stock2.txt, vis2/visibility.tsv). No decision rows; black-box probes only.

## Done (commit 4d268813a, pushed)
Step 1 (a, b, c), probe-settled facts (stock, every configuration C/C++ unless said):
* `<sys/io.h>`: inb inw inl, `_p` forms, outb outw outl, `_p` forms, insb insw insl outsb outsw outsl: NOT exported
  by stock libc (only ioperm/iopl); static inline (nm of own object: local `t` at -O0, nothing at -O2), declared in
  every config incl. c89 strict and C++98, types as outb(2), not noexcept, not macros. Fragment
  headers/sys/io.h.d/010-port-io.frag (`@@owns` all 18), `@@fragments` added to headers/sys/io.h.in.
* `<arpa/telnet.h>`: authtype_names encrypt_names enctype_names telcmds slc_names: `extern char *x[]` always;
  defined (D) under AUTH_NAMES / ENCRYPT_NAMES (encrypt+enctype) / TELCMDS / SLC_NAMES. telopts, TELOPT_FIRST (0),
  TELOPT_LAST (39), TELOPT_OK, TELOPT only with TELOPTS. *_OK: (unsigned int) conversion, int in C / bool in C++, not
  #if-usable; TELCMD_OK evaluates twice; TELCMD(x) = telcmds[x - 236]; SLC_NAME_OK range 0..NSLC; SLC_NAMELIST is
  the 20-entry list; SLC_NAMES is defined by the header (the list) with or without -DSLC_NAMES. Strings in
  stock2.txt. Fragment headers/arpa/telnet.h.d/010-name-tables.frag, `@@fragments` in telnet.h.in.
* `<sys/types.h>` family: 19 new typedefs.tsv rows (__int8_t.. least types, __intptr_t, __loff_t, __rlim_t,
  __rlim64_t, __timer_t void*, __sig_atomic_t, __suseconds64_t; exact types probed), 54 reserved.tsv emit rows
  (every __ typedef sys/types.h shows, except __atomic_wide_counter, __pthread_list_t, __sigset_t: aggregates of
  the pthread/signal internals, left out). visibility.tsv: rows of the 59 new names from
  `header_probe.py --names` on stock (vis2), spliced in sorted place (-557 +3233).
* shiki job headers-stage2-gaps-f-gen1: headers-gen + headers-gen --check ok (242 headers); libc-include committed.

## Next steps (exact)
1. Rerun `python3 ~/work/headers-stage2-gaps-f/probe.py niji2 --include ~/w/sh1/libc-include` (jobrun, label
   headers-stage2-gaps-f-niji2) and probe2.py; diff against stock/probe.tsv and stock2.txt (expect 0 except nm).
2. header_check units: new file spec/tools/header_checks/<slice>.d/<NN>-stage2-gaps-f.py (`units(ctx)`), see
   s12.d/40-stage2-gaps-d.py for the shape: port I/O type/inline units, telnet tables per switch, __intN_t types.
3. Step 2 (forms): include guards. guards() in xtask/src/headers.rs (~line 613) emits `#define _X_H` outright;
   stock keeps a program's earlier definition (macro-forms.tsv `kept`). Cheap fix: wrap in `#ifndef` when
   `!self.t.redefined(header, n)`. FD_SET (stock replaces; templates guard with `#ifndef FD_SET`, GAPS-E D-11.4) and
   ifa_broadaddr/ifa_dstaddr/_PATH_RESCONF: do only if cheap, else list why left.
4. Package proof (step 3): util-linux sys-utils/hwclock-cmos.c, inetutils libtelnet/auth.c + encrypt.c compile
   against the system flavour (find sources on keke: GAPS-E used ~/work/headers-stage2-gaps-e/pkg/).
5. Plants file handoff/HEADERS-STAGE2-GAPS-F-plants.txt via `cargo xtask plants`; clippy, static, precheck,
   report json, handback-check.
