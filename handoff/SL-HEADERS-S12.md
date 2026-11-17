# SL-HEADERS-S12: slice 12, network core and name services (INCOMPLETE, checkpoint at 150k context)

**State.** Branch `task/sl-headers-s12`, code head 27457b733 (cut from origin/next 241236c4e, which holds slice 6).
Say "slice 12" in commits and handoffs. Report: `handoff/reports/SL-HEADERS-S12.json`.

**Done.**
* 16 templates (`headers/{sys/socket,sys/un,netinet/in,arpa/inet,netdb,net/if,ifaddrs,netinet/tcp,netinet/udp,resolv,arpa/nameser,arpa/nameser_compat,nss,aliases,rpc/netdb,sys/socketvar}.h.in`)
  and their output in `libc-include/` (43 headers now). `CMSG_*` (cmsg(3); `CMSG_NXTHDR` is C.9), `IN6_IS_ADDR_*` (RFC 3493),
  `htonl`/`ntohl` family (byte swaps, spec 8.8), `h_errno`, `_res` are template text; every ABI fact comes from the tables.
* Generator (`xtask/src/headers.rs`): nested and anonymous unions/structures, bit-fields (`bits W@O`), arrays of anonymous
  structures, structures only described as a member (`struct ifmap`; skipped for `struct __pthread_*`); object types spell
  the struct keyword; markers `@@needs H...` (names supplied by a header of a slice not in this tree; include is
  `__has_include`-guarded until its template exists) and `@@needs-if NAME H`. `reserved.tsv`: `__cmsg_nxthdr`,
  `__h_errno_location`, `__res_state` emit; `bindresvport6` omit (declared by the reference header but not exported).
* `spec/abi/layouts.tsv`: `struct res_sym` appended by hand from a probe (size 24, align 8; number 0 int, name 8 char *,
  humanname 16 char *); `abi_extract.py` lists it, so the next regeneration must reproduce the rows (check order).
* Proof so far: `headers-gen --check`, clippy, static green; every header compiles alone and twice in gnu17, c11 +
  `_DEFAULT_SOURCE`, gnu++17 (`sl-headers-s12-compile3`). Plants: `handoff/SL-HEADERS-S12/plants.txt` (7).

**Open (not done).** P1 `_Static_assert` TUs for the slice; P2/P3 compare (`header_probe.py`/`callsite_probe.py --include
libc-include --headers <slice>`, `probe_compare.py`); P5 `-std=c89 -pedantic-errors` and `-Wsystem-headers`; P8 (both orders
with `linux/in.h`, `in6.h`, `if.h`, with and without `_GNU_SOURCE`; the `__UAPI_DEF_*` wrappers are NOT written: the structs
`sockaddr_in`, `in6_addr`, `ifreq`... are emitted unconditionally, so including the kernel header first will redefine them);
P10 (CMSG_*, IN6_IS_ADDR_* against a reference-built program); the pending record for the generator additions
(`records/pending/SL-HEADERS-S12-1.toml`, D-NEW, series D-X); a differences row for `bindresvport6` and `__has_include`.
Known gaps: `arpa/nameser.h`, `resolv.h`, `netdb.h` need `sys/param.h`/`signal.h` (slices 7, 8): until they land P2
differs and those two names resolve through the guarded include. `htonl` macros are defined whenever the table says; their
-O2 call-site form is unchecked (P3). The `IN6_IS_ADDR_*`, `CMSG_ALIGN/DATA/FIRSTHDR/SPACE/LEN` macros have no table row, so P2 does not see them.

**Next step.** Run P2/P3 on the slice, fix template conditions; write the `__UAPI_DEF_*` wrappers for P8; P1 TUs; the record.
Plan rows: plan/headers.md 2.4, 3, 4 (slice 12). Do not redo: the generator extensions above.
