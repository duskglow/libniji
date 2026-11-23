# LIBRESOLV-OWN-CODE: libresolv.so.2's own code in libresolv.so.2 (leg 2 handoff)

State: branch `task/libresolv-own-code`, base origin/next d4cbe1667. Code final at 956eb65b6 (clippy and
`cargo xtask static` clean on the role). Role keke ~/w/n4; scratch keke ~/work/libresolv-own-code/ (`lrcount.sh`
counts, `sets/` product sets, `base/` = d4cbe1667 release build, `head/` = this head's release build), mei
~/work/libresolv-own-code/. Plants file: handoff/LIBRESOLV-OWN-CODE-plants.txt.

## What it does

libresolv.so.2 is now built from its own compilation (`crates/libresolv-so`, boundary feature `libresolv`) and no
longer forwards its own code to libc.so.6. libc.so.6 keeps NIJI_PRIVATE aliases only for the 13
`placeholders::STILL_FORWARDED` names; the libresolv object reaches libc state only through libc.so.6 exports
(`malloc` family, `fwrite`, `stdout`, `strlen`/`strnlen`, `__errno_location`, `memcpy`/`memset`) and the two private
entries `__niji_resolv_block` / `__niji_resolv_pfcode`. Exports and versions of both libraries are identical to
spec/abi/versions.tsv (product check `placeholders::product_errors`; `deb_symbols_products`, `versions_products` green).

Classification (step 1). libc.so.6 itself calls none of the 67 (leg 1, exp1: relinked without the 67 aliases, none left).
* Move (54): `__b64_ntop __b64_pton __dn_count_labels __fp_nquery __fp_query __loc_aton __loc_ntoa __p_cdname
  __p_cdnname __p_class __p_class_syms __p_fqname __p_fqnname __p_option __p_query __p_rcode __p_secstodate __p_time
  __p_type __p_type_syms __putlong __putshort __res_nameinquery __res_queriesmatch __sym_ntop __sym_ntos __sym_ston
  _getlong _getshort _res_opcodes inet_net_ntop inet_net_pton inet_neta ns_datetosecs ns_format_ttl ns_get16 ns_get32
  ns_initparse ns_makecanon ns_msg_getflag ns_name_ntol ns_name_rollback ns_parse_ttl ns_parserr ns_put16 ns_put32
  ns_samedomain ns_samename ns_skiprr ns_sprintrr ns_sprintrrf ns_subdomain res_send_setqhook res_send_setrhook`.
* Still forwarded (13, need libc's resolver/NSS/thread state; follow-up): `__fp_resstat __hostalias __res_hostalias
  __res_close __res_isourserver _gethtbyaddr _gethtbyname _gethtbyname2 _gethtent _sethtent res_gethostbyaddr
  res_gethostbyname res_gethostbyname2`.

Leg 2 fixes (leg 1's object pulled libc state): (a) the `#[used]` pthread_once link-edge static (D-22.16) is
SHF_GNU_RETAIN, so `--gc-sections` kept it and the once/thread machinery: not compiled in the libresolv product;
(b) `strz::byte_len` decided CPU paths (`cpu::ready`) before the C layer's scanners: the libresolv product calls
libc.so.6's `strlen`/`strnlen` (same length). The undefined `_init`/`niji_clone7`... in leg 1's note were leftovers of
`ld -r` that `objcopy --strip-unneeded` drops (the final object needs 10 libc symbols). The xtask check now also
refuses any `.init_array`/`.fini_array`/`.ctors`/`.init`/`.fini` in the object (no constructor in libresolv.so.2).
(c) Compat-only own names (`__p_secstodate`, `res_send_set{q,r}hook`: `@GLIBC_2.2.5` only) are defined as local
`niji_own_<name>` and exported by a generated `niji_ph_<name>` stub (`jmp`) under `.symver`. (d) With `local: *` the
version script now lists every export at its node (else the `.symver` names were hidden); unit test
`own_definitions_keep_every_export_global`. (e) Unsafe sites moved where the audit allows them (strz, posix/s16,
heap.rs `LibresolvAlloc`); crates/Cargo.lock refreshed.

## Measurements (release products, d4cbe1667 vs this head at 4537139; libc.so.6 code unchanged since)

libc.so.6: RELATIVE 1778 -> 1496 (-282), all relocations 2066 -> 1784, .text 3,019,657 -> 2,963,593 (-56,064 B),
file -81,144 B. libresolv.so.2: relocations 71 -> 333, .text 1,301 -> 67,532 B, file 29,968 -> 110,504 B.

User instructions per start (Zen 1: execcount median of 21, ASLR off; Skylake-SP: perf stat through runenv, so +~121k
on every row; both deterministic), page faults, kernel instructions (median of 31 interleaved rounds, noisy), syscalls:

| program | Zen 1 user base -> head | Skylake-SP user base -> head | faults | kernel (Zen 1 / SKX) | syscalls | glibc 2.39 user (Zen 1) |
|---|---|---|---|---|---|---|
| empty main | 745,899 -> 740,320 (-5,579) | 867,269 -> 861,689 (-5,580) | 56 -> 52 | -19.5k / -22.2k | 43 = 43 | 100,133 |
| 3 libraries | 1,001,230 -> 995,649 (-5,581) | 1,122,592 -> 1,117,011 (-5,581) | 72 -> 67 | -36.4k / -29.2k | 88 = 88 | 135,133 |
| clock_gettime | 779,930 -> 774,350 (-5,580) | 901,270 -> 895,690 (-5,580) | 61 -> 57 | -20.0k / -28.4k | 43 = 43 | 102,272 |
| resq (-lresolv: res_mkquery, ns_initparse, ns_parserr, ns_get16, p_type) | 1,050,594 -> 975,857 (-74,737) | 1,171,736 -> 1,096,999 (-74,737) | 91 -> 94 | +60.1k / +60.7k | 85 -> 87 | 189,102 |
| resl (needs libresolv.so.2, calls nothing) | 985,746 -> 912,797 (-72,949) | 1,107,318 -> 1,034,370 (-72,948) | 60 -> 57 | +3.4k / +15.4k | 61 -> 60 | 160,503 |

No row is worse in user instructions; every non-libresolv row is better on both CPUs. The libresolv program pays +3
faults and ~+60k kernel instructions (its 67 KB of code is now faulted in from libresolv.so.2) and 74.7k fewer user
instructions (67 bound forwards gone). The brief's ~13.3k per start predates DYN-START-LINK-SHAPE's cheaper RELATIVE
loop; measured now: 5.58k.

Cost of loading libresolv vs glibc (Zen 1 user): libniji resl - empty main = 172.5k (base 239.8k); glibc 60.4k. But
libniji's ld.so costs ~85k per tiny library in general (3 libraries - empty main = 255.3k, glibc 35.0k): the loader's
per-library cost is ~7x glibc's, a loader finding for a follow-up story, not libresolv's. libresolv's own share above a
tiny library: ~87k (333 relocations, 23 imports, 71 exports).

## Audit (step 3)

Only libresolv.so.2 imports `@NIJI_PRIVATE` names. libanl, libdl, libpthread, librt, libutil are version-placeholder
shells (text 5 B; their functions are libc.so.6's own ABI). libm, libmvec, libnsl, libBrokenLocale,
libc_malloc_debug have their own code and import only public libc names. libc.so.6's other `niji_*` exports are
the public NIJI_NATIVE_1 API. No other case found.

## Tests and plants

`sonames_resolv_dlopen_own_code` expects the output of the same program on glibc 2.39 (probe settled: `p_query`
parses a 512-byte message, so it prints `;; ns_initparse: Message too long` for a short question; the test passes a
512-byte buffer and uses `fp_nquery` for the question). Plants (5): link edge back in, CPU dispatch back in,
`__res_close` not forwarded, fwrite seam dropping text, block seam failing: all CAUGHT on 4537139 (run 1).
Package flavours (liella6 / libc6 debs): NOT VERIFIED here (needs tools/deb/build.sh); both ship this libresolv.so.2.

## Next step

Collect the final-head jobs (keke): `libresolv-own-code-tests2` (role build + changed crates' tests),
`libresolv-own-code-plants2`, `libresolv-own-code-fullhot2` (release build + hot_text_products + Zen 1 counts).
Follow-ups: move the 13 STILL_FORWARDED names (needs seams for `_res`, hosts-file cursor, h_errno); the ld.so
per-library load cost.

Rows: D-16.6, D-27.9, D-27.22, D-22.16, D-4.21.
