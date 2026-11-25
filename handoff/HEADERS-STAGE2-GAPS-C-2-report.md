# HEADERS-STAGE2-GAPS-C-2: report

Code head 0dcb8fb0b (then the plants results, this report and the report json). Branch `task/headers-stage2-gaps-c-2`.

## Commits (mine)
* Merge of origin/next (conflicts: syslog.h.in, layouts/macros/visibility tables, abi_extract.py, header_probe.py;
  generated files taken from next) and the regeneration commit (headers-gen, abi-gen) after it.
* header_probe `thin_names()` candidates; rows spliced into visibility.tsv (71 new keys before pruning) and macros.tsv
  (58 + 1 by hand); `headers/utmp.h.in` member aliases; `headers/arpa/nameser.h.in` byte-order and accessor macros;
  S13 `THIN` units; plants.

## Jobs (all on the Zen 1 host, role tree synced from this branch)
* `headers-stage2-gaps-c-2-hc5`: clippy `--all-targets --all-features -D warnings`, `cargo xtask static`, headers-gen
  --check, abi-gen --check, FULL header_check **0 failed of 21580**, `cargo test -p xtask` (665 + the other xtask
  binaries, all ok), `cargo test -p niji-abi` ok, gmp.h compile proof (C gnu17, C89, C++17; the gmp.h extracted in leg 1,
  no new download): **rc 0**. Before the nameser accessors the same job (`-hc4`) was green too.
* `headers-stage2-gaps-c-2-plants2`: 5 plants, **5 CAUGHT**, 0 survived, 0 WARN (handoff/HEADERS-STAGE2-GAPS-C-2-plants.txt).
  A first run (`-plants`, on the head before the PRIb unit compared characters) had one SURVIVED plant (PRIb64 "ld" for
  "lb": the unit compared only sizes); the unit was fixed (strcmp) and the run repeated: SUPERSEDED.
* Not re-run on the merged head: the package proofs of legs 1-3 (bzip2, coreutils expr, elfutils libelf, net-tools):
  they download sources, and the coordinator's note of 15:50Z allows the local mirror only; the merge changed no header
  they use. The gmp.h proof above was repeated.
* Time: the xtask tests take ~15 min of each hc job (the long pole); queue waits for a slot 5-10 min.

## Probe-settled behaviour (black-box, reference system)
* ut_name/ut_time/ut_xtime/ut_addr are aliases of ut_user / ut_tv.tv_sec / ut_tv.tv_sec / ut_addr_v6[0] (same object, same
  size). UTMP_FILENAME "/var/run/utmp", WTMP_FILENAME "/var/log/wtmp".
* GETSHORT/GETLONG (= NS_GET16/32) read big-endian and advance by 2/4; PUTSHORT/PUTLONG (= NS_PUT16/32) write and advance.
  ns_msg_id: int; ns_msg_base/end: const unsigned char *; ns_msg_size: long; ns_msg_count: int; ns_rr_name: char *, "." for
  an empty name; ns_rr_type/class: ns_type/ns_class; ns_rr_ttl: unsigned int; ns_rr_rdlen: int; ns_rr_rdata: const unsigned char *.
* The values of the new PRIb*/UDP_*/SOL_UDP rows are type_probe's.

## Tests: what they prove and do not
The THIN units are compile-time (form: sizes, types, visibility per configuration; the PRIb strings by strcmp). They do
not run the byte-order macros: a wrong shift or advance would not be caught by them (the gmp/C conformance programs do
not use them either). A runtime check is the next addition if these macros matter beyond compiling.

## RULING NEEDED / INFORMATIONAL
* Ruling #8587 stays provisional: see the guard-set note in HEADERS-STAGE2-GAPS-C-2.md (one place to narrow).
* INFORMATIONAL: `header_probe.py thin_names()` has no test of its own (a probe tool; its output is covered by the rows
  and units). Candidate lists for queue.h/timex.h/pidfd.h/telnet.h found nothing new: the remaining counts of those headers
  are names outside the lists.
* INFORMATIONAL: coreutils needs a complete `struct __locale_struct` (leg 3).
* Slips: I printed a diff of the generated `libc-include/utmp.h` (a view of my own change, not a full read); a first
  `jobwait` over ssh blocked a Bash call while the job sat in the slot queue.

## IMPROVEMENT
header_probe/type_probe fail with "the unit failed outside its own test lines" without printing the compiler's error: print
the first error line (it cost me a probe round and a guess).
