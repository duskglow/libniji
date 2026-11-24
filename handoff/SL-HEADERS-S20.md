# SL-HEADERS-S20: slice 20, switch and package (handoff, run 2 = SL-HEADERS-S20-2, leg 1)

**State.** Branch `task/sl-headers-s20-2` (run 1's `task/sl-headers-s20` + run 2), base origin/next a0d93596e.
Head with the fixes: 6228e4df5 (this note is committed on top). Build role keke:~/w/n1. Say "slice 20" in commits.
`cargo xtask affected --list --base origin/next` escalates to FULL (headers/*.h.in and reserved.tsv are generator
inputs): the gate is the train's; this story proves itself with precheck + the `test` job below (brief, step 4).

**Decisions in force.** D-X.168 (drivers default to libniji's headers; coexist sysroot `/usr/lib/niji/usr/include`
+ `x86_64-linux-gnu`, ruling #8413; plan/headers.md slice 20 row updated). D-X.169 (`__locale_t`,
`__CORRECT_ISO_CPP_STRING_H_PROTO`, `__CORRECT_ISO_CPP_WCHAR_H_PROTO`, ruling #8412; R-H2/R-H3). Run 1's items
(drivers, 236 manifest rows, P4 tool, `TESTS:` reason for niji_cc_headers.rs in commit 90ca3dad3) stand.

**Done in run 2 (6228e4df5).** Cause by cause, P4 header x -std FAIL count, before (run 1, t2) -> after (t3):
* `__locale_t` undeclared (C++ library's c++locale.h, every `<iostream>`/`<locale>`/`<sstream>` user): fixed.
  header_probe.py now takes reserved.tsv `emit` names as candidates; full reference run (243 headers, 40 configs,
  19 min) gave 25 `__locale_t` rows (each = the header's `locale_t` row), spliced into visibility.tsv; typedefs.tsv
  row from type_probe.py --headers argp.h. 11 generated headers gained the typedef.
* `<cwchar>` const-overload collision (wcschr, wcspbrk, wcsrchr, wcsstr, wmemchr): fixed. The C++ library's
  `<cwchar>`/`<cstring>` declare their own std overloads unless `__CORRECT_ISO_CPP_{WCHAR,STRING}_H_PROTO` is defined
  (read from the C++ library's own header, GCC's, under R-H2 like the kernel's libc-compat.h guards; never the
  reference's). Probe: 3 + 7 rows, C++ configs only, wherever wcschr/strchr show. string.h.in/wchar.h.in define them
  in the C++ overload blocks (`@@owns`).
* Totals: 218 FAIL -> 4 FAIL. Per -std after: c++98 104 ok/39 n/a/2 FAIL, c++03 same, c++11 134/11/0, c++14
  133/12/0, c++17 133/12/0, c++20 134/11/0, c++23 134/11/0; c++26: "not supported by this compiler" (tool fixed).
* niji_cc_headers: 4 passed (both C++ driver tests now green).

**Open (next leg, in order).**
1. c++98/c++03 `<complex.h>`, `<tgmath.h>` (4 FAIL): the C++ library's complex.h `#include_next <complex.h>` and
   finds none: `/usr/include/c++/13/complex.h:38:16: fatal error: complex.h: No such file or directory`. Check
   whether libniji's staged headers ship complex.h/tgmath.h (target/niji/include, libc-include/); if they are
   meant to ship (spec has complex.h rows), find why not staged; if deliberately absent, RULING NEEDED.
2. P4 part 2: `python3 spec/tools/callsite_probe.py --include target/niji/include --out <tmp>` stops with
   `RuntimeError: aio.h.c89.addr.c: the unit failed outside its own test lines`. Find the error the unit gives
   (compile that unit by hand: c89 config, -nostdinc + libniji + gcc include + uapi), fix the header or the tool's
   flags; then noexcept.tsv/callsites.tsv must equal spec/abi's.
3. Plants: job `sl-headers-s20-2-plants` (keke, started 16:53Z on 6228e4df5, 9 plants of
   handoff/SL-HEADERS-S20-plants.txt incl. 2 new: s20-cxx-locale_t, s20-cxx-cwchar). Read `~/logs/<label>.log`.
   If later fixes touch libc-include or the drivers, rerun plants on the final head (new --dir plants-<n>).
4. Final head: clippy, static, rerun ~/work/sl-headers-s20-2/t3.sh as a new `test` job (so + niji_cc_headers +
   headers_p4.py), `cargo xtask precheck --jobs <labels> --head <sha>`, report json, handback-check.
5. Not in this brief (hand back without them, as next step): P6 (`sh conformance/run.sh --niji-headers`), P7
   (smoke with `--niji-headers`), a .deb built with the new rows.

**Jobs (keke).** sl-headers-s20-2-so1 (build, rc 0), -hprobe3 (probe, rc 0), -t3 (test: so ok, niji_cc_headers
4/4, P4 rc 1 = the 4 FAIL + callsite part), -plants (running). Stale/killed: -hprobe (old probe script, killed),
-hprobe2 (25-header run, superseded by hprobe3).

**Not to redo.** The reference probe (35 rows are in); the type_probe row; records D-X.168/169; plan row edit.
Scratch tools: ~/work/sl-headers-s20-2/cw.sh `<role> <-std> <header> [N]` prints the errors of one C++ header
compiled on libniji's headers in niji-gcc's order.

**Plan rows to read.** plan/headers.md section 3 (P4), slice 20 row; D-X.107, D-X.113, D-X.168, D-X.169.
