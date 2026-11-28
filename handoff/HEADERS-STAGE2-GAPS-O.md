# HEADERS-STAGE2-GAPS-O: handoff (run 1)

Branch task/headers-stage2-gaps-o, cut from origin/next 051f93be2, no merge. Role mei:~/w/s1. Job scripts in
mei:~/work/headers-stage2-gaps-o/ (godiff.py and ordercls.py from GAPS-N; nameonly.py lists the `nameonly` class with
headers and configurations, never stock's values; libgo.sh + lgmiss.sh; splice/probe/values/layouts/proof scripts).
Plants: handoff/HEADERS-STAGE2-GAPS-O-plants.txt.

## Clean-room method (flag #8873, info)
Stock's -fdump-go-spec output carries macro bodies (header text). Only the NAMES of stock's dump were used (as an
`#ifdef` probe would give them); values, kinds and layouts come from header_probe.py, type_probe.py, enum_probe.py and
abi_extract.py; RTMSG_OVERRUN's form from an identical-redefinition probe (exit status only).

## Classification of the 547 `nameonly` names
* 330 reserved (`__*`, `_BITS_*`, `_*_H` guards, `__HAVE_*`, `__GLIBC_USE_*`, `__*_defined`): not shipped, the
  generator's default `omitted` rule (headers/reserved.tsv); no package named needs them (plan/headers.md section 3).
* 164 public macros and enumerators + the C2X PRIb/SCNb rows: shipped (all but RT_ADD, RT_CONSISTENT, RT_DELETE).
* 54 `sizeof_*` (struct sizes godump prints): 5 public shipped (group_req, group_filter, ip_msfilter, ip_opts,
  timestamp). Not shipped, by group:
  - reserved types (`__FILE`, `__fpos_t`, `__fpos64_t`, `_G_fpos64_t`, `__mbstate_t`, `__once_flag`, `__sigval`,
    `__pthread_*`, `__atomic_wide_counter`, `__gconv_*`, `__prfpregset_t`): stock-internal spellings of types
    libniji declares under its own reserved names; no package named.
  - kernel-shared (`__kernel_fd_set`, `__kernel_fsid_t`, `__kernel_sockaddr_storage`, `__ptrace_*`, `prctl_mm_map`):
    stock's header pulls the kernel UAPI header that declares them; libniji's does not. libgo copies the sizes but
    names no constant of them (no libgo error).
  - sigcontext internals `_fpreg`, `_fpx_sw_bytes`, `_xsave_hdr`, `_xstate`, `_ymmh_state` (reserved spellings).
  - public, NOT YET (next story): Elf32_RegInfo, Elf32_gptab, Elf_MIPS_ABIFlags_v0, Elf_Options, Elf_Options_Hw
    (elf.h), atalk_addr, atalk_netrange, sockaddr_at (netatalk/at.h), fddi_header, tr_statistics, trh_hdr, trllc,
    trn_hdr, shaperconf, sockaddr_iucv, sockaddr_pkt, r_debug_extended, ns_tsig_key, ns_tcp_tsig_state,
    _ns_flagdata, gaih_addrtuple. Method as here: add to abi_extract.py TYPES (and HEADERS for netinet/if_tr.h,
    netatalk/at.h, net/if_shaper.h, netiucv/iucv.h, net/if_packet.h, netinet/if_fddi.h), run --layouts-only, take
    only their rows (a fresh run differs from the committed table elsewhere), typedefs.tsv rows, splice the hp1
    struct/typedef rows (mei:~/work/headers-stage2-gaps-o/hp1/visibility.tsv), add to TYPES_SHIPPED.
* RT_ADD/RT_CONSISTENT/RT_DELETE: stock declares them in an enumeration inside struct r_debug's r_state member (C:
  file scope; C++: r_debug's scope, so not global). libniji emits r_state as unsigned int. Needs a generator construct
  (an enumeration declared in a member, enumerators.tsv tag for it); NOT_YET in the fragment.

## Form-only class (item 2)
Decimal against stock's hex or expression spellings: a program sees the value, the type (macros.tsv type column,
checked by the generated P1 units), `#if` usability (form.py) and, for strings, the bytes; the spelling itself only
through `#` stringizing of an expansion, which no package in the Yohane corpus does. Casts godump cannot read: the
dump drops the name in stock too, so no package of this kind sees a difference. Recorded, not changed.

## Proof (report json for the labels)
Sweep nameonly 547 -> 382, shipped group 0 (both trees); libgo sysinfo.c missing only reserved/kernel sizes;
header_probe both trees vs stock for the shipped names, C and C++: 560 rows 0 DIFF; fragment 0 libniji and
--reference; dup.py 0 (3261 dup-listed rows removed).

## Next step
Land through the train if the jobs are green. Follow-up story: the 20 public struct types and the r_debug member
enumeration above.
