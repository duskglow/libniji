# HEADERS-STAGE2-GAPS-F: handoff (run 1, leg 3: hand-back)

Branch task/headers-stage2-gaps-f from origin/next 05995ca18; role shiki:~/w/sh1. No decision rows (black-box probes
only, no ruling needed). Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-F/`; shiki and keke
`~/work/headers-stage2-gaps-f/` (probe tables, pkgproof.sh, pkgproof2.sh, Debian orig trees under pkg/).

## What the branch does (all pushed)
* 4d268813a: `<sys/io.h>` port I/O (inb .. outsl, `_p` and string forms: static inline, every configuration and
  optimisation level, no library symbol, not noexcept; x86 only, fragment headers/sys/io.h.d/010-port-io.frag);
  `<arpa/telnet.h>` name tables and macros (AUTHTYPE_NAME[_OK], ENCTYPE_NAME[_OK], ENCRYPT_NAME[_OK], TELCMD[_OK],
  SLC_NAME[_OK], SLC_NAMELIST, tables `extern char *[]`, defined by AUTH_NAMES / ENCRYPT_NAMES / TELCMDS / TELOPTS /
  SLC_NAMES; fragment headers/arpa/telnet.h.d/010-name-tables.frag); `<sys/types.h>` `__int8_t` .. `__uint64_t`
  and the reserved `__` family stock shows there (19 typedefs.tsv rows, 54 reserved.tsv emit rows, visibility).
* cb45747a5: header_check fragments s15.d/10 (port I/O), s13.d/10 (telnet), s05.d/10 (exact sys/types.h).
* 6adf24eb2: FEATURES-FORMS items where libniji moved away from stock: include guards keep a program's earlier
  definition where macro-forms.tsv says `kept` (generator guards(), xtask/src/headers.rs); FD_SET blocks guard on
  `__NIJI_FD_BITS` (replaced, as stock); ifa_broadaddr / ifa_dstaddr / _PATH_RESCONF kept. Check s05.d/20.
  None of the listed form items is left.
* 1f88580f8, 26c0557da, f25fbca87, 022f7ba7c (leg 3): typedefs.tsv function/object rows and providers.tsv `none`
  rows for the fragment-owned names (the xtask rows test needs a row per visibility name; providers.tsv names who
  provides a function no library exports, as alloca's row does); typedefs.tsv re-sorted (static abi-types); plants.

## Proof
* header_check (full) 0 failed; features.py forms 0 differ; probes vs stock 0 diff (leg 2, gen3; final: test1).
* Packages (keke, -nostdinc + libniji headers vs stock): pkg1: hwclock-cmos.c, libtelnet auth.c and encrypt.c
  with and without -DAUTHENTICATION -DENCRYPTION: all compile. pkg4: all 31 C units of inetutils libtelnet/,
  telnet/, telnetd/ and util-linux hwclock(.c, -cmos, -rtc), lib/monotonic.c: all compile as with stock (pkg2/pkg3
  only lacked ncurses' termcap.h / ncurses_dll.h, outside libc, under -nostdinc). No further header gap found.
  Not done: a full link of either package (header gaps only).
* Plants (handoff/HEADERS-STAGE2-GAPS-F-plants.txt): plants1 7/9 caught; its 2 survivors were plants-file errors
  (wrong --only unit; a type spelling the rows test only compares with itself), fixed; plants2 on the final code: 9 of 9 caught, no WARN.
* `cargo xtask affected` escalates to FULL (header generator inputs): FULL is the train's; gated with precheck plus
  test1 (headers-gen --check, cargo test -p xtask, header_check, features.py).

## INFORMATIONAL
* Stock skips a whole header whose include guard a program defined first; libniji keeps the guard's value but still
  declares the header's contents (its own guards are `__NIJI_` names).
* The typedefs.tsv `cxx` of a fragment-owned function is consistency-only in the rows test (it declares from the
  row); the real types are the header_check units' job.

## Next step
None for this story: land through the train (FULL gate there).
