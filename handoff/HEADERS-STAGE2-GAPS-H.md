# HEADERS-STAGE2-GAPS-H: handoff (run 1, end of leg 3)

Branch task/headers-stage2-gaps-h from origin/next 0f7aa0180; role keke:~/w/n1. No new decision rows (black-box
probes only; the sched declarations rest on D-24.12 / owner ruling R3). Scratch: local
`~/w/.scratch/HEADERS-STAGE2-GAPS-H/`; keke `~/work/headers-stage2-gaps-h/` (probe4.py, test2.sh, test3.sh, pkg.sh,
mkroot.sh; `excal/` = Devuan excalibur root with libc6-dev 2.41, headers only).

## Done (pushed)
* Item 1 (730f215ea): `<arpa/ftp.h>` FTP_NAMES tables `typenames`, `strunames`, `modenames`, `formnames`
  (fragment, header_check s13.d/30, conformance program `s13_arpa_ftp_names` with `/* NIJI-HEADERS: yes */`).
* Item 2: sweep below.
* Item 3 (util-linux chrt/uclampset, found on the way): `struct sched_attr` (kernel UAPI `<linux/sched/types.h>`),
  `sched_setattr`, `sched_getattr` and the SCHED_* / SCHED_FLAG_* names in `<sched.h>` under _GNU_SOURCE
  (`headers/sched.h.d/010-sched-attr.frag`, s08 slice cell, header_check s08.d/30, spec section 24.3 text).
* d5454f918: `spec/tools/attr_probe.py check` prints a line of a `prototypes-additions.tsv` function that only the
  reference cannot build as a known difference naming the row's decision (was: counted, so the s19 test
  `diagnostic_attributes_match_the_reference` failed once <sched.h> declared the two R3 functions).
* 986fb4c2f: regenerated `libc-include/` (arpa/ftp.h, sched.h) committed; leg 1 had not, which is why pkg2 stopped.
* 8e0b14e84: plant `s2h-attr-additions` (11 plants in handoff/HEADERS-STAGE2-GAPS-H-plants.txt).

## Results on the code head 8e0b14e84 (keke)
* test3: headers-gen --check ok (242 headers); attr_probe check base: 2827 functions, 17157 tests, 0 differ;
  `cargo test -p xtask s19`: 2 passed.
* test2 (code identical but for attr_probe): header_check 0 failed of 26137; probe4 libniji vs 2.41 diff 0 lines,
  no SCHED name missing or extra in gnu, cxx17, c11, cxx11s; conformance s13_arpa_ftp_names passed.
* clippy (workspace, all targets, all features, -D warnings) rc 0; `cargo xtask static` rc 0.
* plants2: 11 of 11 CAUGHT. pkg3: inetutils gets past ftpd.c and stops on `<protocols/talkd.h>` (talk/*.c:
  CTL_MSG, CTL_RESPONSE, ANNOUNCE, ...); util-linux compiles everything and stops at dpkg-gensymbols
  (`uuid_time64@UUID_2.40`: needs `__USE_TIME_BITS64`, which 2.41 defines on x86-64 and 2.39 does not; RULING
  NEEDED in the report).

## 2.41 probe results (settled, do not redo)
struct sched_attr and both functions only under _GNU_SOURCE (C gnu*, C++ default); under GNU <sched.h> includes
<linux/sched/types.h>; sizeof 56 align 8, kernel VER1 members; SCHED_NORMAL 0, SCHED_EXT 7, SCHED_FLAG_* 1..64,
KEEP_ALL 24, UTIL_CLAMP 96, SCHED_FLAG_ALL undefined; prototypes non-const, -Wnonnull on a null attr; own struct +
<sched.h> under GNU = error; linux/sched/types.h or linux/sched.h with <sched.h> either order = ok.

## Sweep (item 2)
* `<arpa/ftp.h>`: only FTP_NAMES switches anything. `<arpa/telnet.h>` *_NAMES: 0 DIFF (no regression).
  tftp/inet/nameser: no switch. `<protocols/routed.h>` RIPCMDS -> `ripcmds` (5), `<protocols/timed.h>` TSPTYPES ->
  `tsptype` (25): both lacking with the whole header (below).
* GAP (reported, not chased): libniji's `<protocols/routed.h>`, `<protocols/timed.h>`, `<protocols/rwhod.h>`,
  `<protocols/talkd.h>` lack their protocol names (RIPVERSION, struct rip; TSPVERSION, struct tsp; WHODVERSION,
  struct whod; TALK_VERSION, NAME_SIZE, struct osockaddr ...). Recommend one follow-up story for the four.

## Not to redo
The FTP table probes; the arpa/protocols sweep; the telnet regression; the 2.41 probes; the excalibur root.

## Next step
Land (coordinator). Follow-ups: one story for the four `<protocols/*.h>` headers (talkd.h first: inetutils' next
stop); a ruling on `__USE_TIME_BITS64` for util-linux's libuuid.
Report: handoff/HEADERS-STAGE2-GAPS-H-report.md, handoff/reports/HEADERS-STAGE2-GAPS-H.json.
