# SL-HEADERS-S03: type and value tables (plan/headers.md slice 3)

Branch `task/sl-headers-s03` (from origin/next db204dc19). Role keke:~/w/n1 (keke: Ubuntu glibc 2.39-0ubuntu8.9,
GCC 13.3, the same toolchain as the reference). Brief: ~/state/briefs/SL-HEADERS-S03.md. Plan rows: plan/headers.md
§2.1, §2.2, §4 slice 3; pending decision records/pending/SL-HEADERS-PLAN-1.toml.

## State: done (slice 3)
* `spec/tools/type_probe.py` (new; it reuses header_probe.py's Unit and error-line reader) writes
  * `spec/abi/typedefs.tsv` (3804 rows): kind, name, header, size, align, cxx. 233 typedefs, 132 structs, 1 union,
    4 enums, 37 data objects, 3397 functions. Size and alignment come from the probe's own C objects (`gnu`
    configuration); `cxx` is the demangled template argument of the probe's own C++ function
    `tp_xx_N(tp_w<T>*)` (`cxx14`, so `noexcept` stays out of function types: slice 4's table holds it). Objects
    give `decltype(O)`, functions `decltype(F)` with every typedef resolved (`signal` is
    `void (*(int, void (*)(int)))(int)`, `FILE` is `_IO_FILE`).
  * `spec/abi/macros.tsv` (7112 rows): name, header, type, value. 7104 integer constants (`_Generic` type and
    value), 3 floating (`HUGE_VAL` double, `INFINITY`/`NAN` float; values as exact hex, inf, nan), 5 strings
    (`PRId32`, `PRIu64`, `PRIxMAX`, `P_tmpdir`, `_PATH_MOUNTED`).
  * The `header` column is the header the probe used: the empty unit when the compiler shows the name, else the
    first header in sorted order that shows it in that configuration (so `FILE` reads `argp.h`). The generator
    (slice 5) assigns a name's home header itself; this table only needs one header that shows it.
* `spec/tools/abi_extract.py`: 50 types added to the DWARF path (large-file `struct stat64`, `flock`, `flock64`,
  `statfs64`, `statvfs64`, `rlimit64`, `f_owner_ex`, `file_handle`; protocol `ip`, `iphdr`, `ip6_hdr`, `tcphdr`,
  `tcp_info`, `udphdr`, `icmphdr`, `icmp`, `icmp6_hdr`, `ether_header`, `arphdr`, `sockaddr_ll`, `ip_mreqn`,
  `ipv6_mreq`, `in_pktinfo`, `in6_pktinfo`; ELF `Elf32/64_{Ehdr,Phdr,Shdr,Sym,Dyn,Rel,Rela,Nhdr}`, `Elf64_Chdr`,
  `Elf64_auxv_t`, `Elf64_Ver{def,daux,need,naux}`; debug `struct user`, `elf_prstatus`, `elf_prpsinfo`,
  `elf_siginfo`), 10 headers added to its prelude, and `--layouts-only`. layouts.tsv: 211 types, 1908 rows;
  the diff against the old file is 536 added lines, all rows of the new types, no existing row changed.
* `xtask/src/abi_types.rs`: static check `abi-types` (also `cargo xtask abi-types`): shapes, order, every row's
  (header, name, kind) shown by visibility.tsv, size/align vs layouts.tsv, object sizes vs versions.tsv, integer
  values vs constants.tsv (mod 2^64), hex-float and string spellings. 6 unit tests; passes on the real tables.

## Proof (jobs on keke)
* `sl-headers-s03-probe1`: layouts rerun, then type_probe twice, `cmp` of both tables: byte-identical.
* `sl-headers-s03-lint2`: abi_types tests, `cargo xtask abi-types` on the real tables, xtask clippy, static: rc 0.
* `sl-headers-s03-plants`: 17 plants on the checker (handoff/SL-HEADERS-S03/plants.txt).
* `sl-headers-s03-gate`: direct tier, base origin/next.
* Superseded: `-trial1` (argument parse), `-trial2` (trial subset), `-lint1` (static-check array length).

## Open / next
* INFORMATIONAL: macros.tsv's floating and string rows are few because the candidates are slice 2's
  (constants.tsv, prototypes, versions, layouts and the spec's backticked names): `M_PI`, `_PATH_BSHELL`, `PRId64`,
  `FLT_MAX` are not candidates. Next step: add the standards' and manuals' macro names (C 7.8 PRI/SCN set, POSIX
  math.h `M_*`, paths.h names from the manual) to header_probe.py's candidates, rerun slice 2, then rerun
  type_probe.py. Likewise prototypes.tsv has 3778 names against 3397 function rows; the gap includes C++ overload sets
  such as `strchr` (`decltype` of an overload set fails) and names the C++ configuration does not show.
* INFORMATIONAL: layouts-aarch64.tsv was not regenerated with the 50 new types (needs the aarch64 sysroot run,
  `abi_extract.py --arch aarch64`).
* A future full `abi_extract.py` run will add constants and prototypes from the 10 new prelude headers (elf.h
  values for slice 16): expected, not a regression.
