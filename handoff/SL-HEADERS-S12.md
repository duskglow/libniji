# SL-HEADERS-S12: slice 12, network core and name services (run 2: INCOMPLETE, P8 open)

**State.** Branch `task/sl-headers-s12-2` (on `task/sl-headers-s12`'s work). Say "slice 12" in commits and handoffs.
Report: `handoff/reports/SL-HEADERS-S12-2.json`. Plants: `handoff/SL-HEADERS-S12/plants.txt` (7, stale-output) and
`handoff/SL-HEADERS-S12-2-plants.txt` (4, P10 evaluation).

**Done in run 2.**
* Step 0: clippy and static green on the inherited head.
* `spec/tools/header_check.py`: P1 for the 16 headers (sizes, alignments, member offsets, integer constants; names another
  slice's header owns are skipped, P1 runs there); `--kernel DIR` adds P8 units; `--system` prints the reference's P8 verdicts;
  `--p10` builds `spec/tools/p10_net.c` on both header sets and compares 1299 lines (CMSG_*, IN6_IS_ADDR_*, byte swaps).
  Result: P1+P5 0 failed of 2687; P10 identical.
* P2 and P3 (`header_probe.py`/`callsite_probe.py --include libc-include --headers <slice>`, `probe_compare.py`) compare clean with
  `handoff/SL-HEADERS-S12/accept.py` (2616 P2 lines, 20 P3 lines accepted). Accepted classes: (a) names that `sys/param.h` and `signal.h`
  and what they include supply to `arpa/nameser.h`, `resolv.h`, `netdb.h` (the `__has_include` guard, ruling #6406; goes away
  with SL-HEADERS-S07-TAIL); (b) `_SIGSET_NWORDS`, `_STRUCT_TIMESPEC` (reserved names no template defines); (c) `bindresvport6`
  (declared by the reference header, not exported: omitted; needs a differences row, no compat option since it cannot link);
  (d) `recv`/`recvfrom` fortify call forms (slice 18).
* Template fixes the probes found: endian.h, sys/socket.h, stdint.h and string.h now included only where the table shows the
  names; `s6_addr`, `s6_addr16/32`, `IN6_ARE_ADDR_EQUAL` were missing (P10 found them).

**Open.**
* P8: the libc-first order fails in 10 of 15 units only on `IP_RECVRETOPTS`: linux/in.h spells it `IP_RETOPTS`, libniji's table value
  is `7`. Needs a generator way to spell a macro symbolically (the generator refuses a template define of a table-owned macro;
  its message points at `@@value(NAME)`). The kernel header first fails against the reference's headers too (14 of 30 units;
  `--system`), so that order is not asserted. RULING NEEDED: make kernel-first work (extension; structs, enumerators, macros and
  `group_source_req` need `__UAPI_DEF_*` wrappers; I tried struct wrappers, reverted as insufficient alone).
* Not done: the pending record (`records/pending/SL-HEADERS-S12-1.toml`, D-NEW, series D-X: generator additions, accept rules, P10),
  the differences row for `bindresvport6`, P5 c89/-pedantic, `header_check.py` is not a cargo test (direct tier cannot select it).
* Jobs named in the report were started at hand-back on the final head; read their rc.

**Next step.** Fix `IP_RECVRETOPTS` spelling, rerun `header_check.py --kernel DIR` (stage `linux`, `asm-generic`, `asm` from
linux-libc-dev), write the record and the differences row, wire header_check into a cargo test.
