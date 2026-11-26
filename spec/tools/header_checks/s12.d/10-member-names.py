"""Slice 12 checks: member-names (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py; header_checks/README.md)."""
from header_check import expand


# Member-name macros: a name for a member nested in a structure's union or inner structure, each a macro for exactly
# that nested member (black-box form probes: HEADERS-BUSYBOX-GAPS for <net/if.h>, poison-after-header; HEADERS-NET-SYS-
# NAMES for the others, a unit whose `struct niji_f` has only the candidate path compiles a use of the name with the
# reference's headers exactly for its expansion). header: {name: (tag, the nested member it names; `[k]` an element)}.
# The names of one table come from the man page or RFC named in the header's template: netdevice(7) (<net/if.h>),
# RFC 792 (<netinet/ip_icmp.h>), RFC 4443, 2710, 4861 and 2894 (<netinet/icmp6.h>), RFC 8200 (<netinet/ip6.h>),
# RFC 826 (<netinet/if_ether.h>, also shown by <netinet/ether.h>), route(8) (<net/route.h>), msgctl(2) (<sys/msg.h>),
# getifaddrs(3) (<ifaddrs.h>, and <net/if.h>'s struct ifaddr), utmp(5) (<utmp.h>; HEADERS-GETTEXT-CLANG, the same
# probe for each name and header).
_ICMP6 = {"type": "icmp6_type", "code": "icmp6_code", "cksum": "icmp6_cksum"}
_ICMP6_DATA = "icmp6_dataun.icmp6_un_data"
_ARP = {f"arp_{m}": ("ether_arp", f"ea_hdr.ar_{m}") for m in ("hrd", "pro", "hln", "pln", "op")}
MEMBER_MACROS = {
    "net/if.h": {
        "ifr_name": ("ifreq", "ifr_ifrn.ifrn_name"),
        **{f"ifr_{m}": ("ifreq", f"ifr_ifru.ifru_{m}") for m in
           ("hwaddr", "addr", "dstaddr", "broadaddr", "netmask", "flags", "mtu", "map", "slave", "data", "newname")},
        **{f"ifr_{m}": ("ifreq", "ifr_ifru.ifru_ivalue") for m in ("metric", "ifindex", "bandwidth", "qlen")},
        "ifc_buf": ("ifconf", "ifc_ifcu.ifcu_buf"),
        "ifc_req": ("ifconf", "ifc_ifcu.ifcu_req"),
        **{f"ifa_{m}": ("ifaddr", f"ifa_ifu.ifu_{m}") for m in ("broadaddr", "dstaddr")},
    },
    "ifaddrs.h": {f"ifa_{m}": ("ifaddrs", f"ifa_ifu.ifu_{m}") for m in ("broadaddr", "dstaddr")},
    "utmp.h": {"ut_name": ("utmp", "ut_user"), "ut_time": ("utmp", "ut_tv.tv_sec"),
               "ut_xtime": ("utmp", "ut_tv.tv_sec"), "ut_addr": ("utmp", "ut_addr_v6[0]")},
    "netinet/ip_icmp.h": {
        **{f"icmp_{m}": ("icmp", f"icmp_hun.ih_{m}") for m in ("pptr", "gwaddr", "void")},
        "icmp_id": ("icmp", "icmp_hun.ih_idseq.icd_id"), "icmp_seq": ("icmp", "icmp_hun.ih_idseq.icd_seq"),
        "icmp_pmvoid": ("icmp", "icmp_hun.ih_pmtu.ipm_void"), "icmp_nextmtu": ("icmp", "icmp_hun.ih_pmtu.ipm_nextmtu"),
        **{f"icmp_{m}": ("icmp", f"icmp_hun.ih_rtradv.irt_{m}") for m in ("num_addrs", "wpa", "lifetime")},
        **{f"icmp_{m}": ("icmp", f"icmp_dun.id_ts.its_{m}") for m in ("otime", "rtime", "ttime")},
        "icmp_ip": ("icmp", "icmp_dun.id_ip.idi_ip"),
        **{f"icmp_{m}": ("icmp", f"icmp_dun.id_{m}") for m in ("radv", "mask", "data")},
    },
    "netinet/icmp6.h": {
        **{f"icmp6_data{w}": ("icmp6_hdr", f"{_ICMP6_DATA}{w}") for w in (32, 16, 8)},
        "icmp6_pptr": ("icmp6_hdr", f"{_ICMP6_DATA}32[0]"), "icmp6_mtu": ("icmp6_hdr", f"{_ICMP6_DATA}32[0]"),
        "icmp6_id": ("icmp6_hdr", f"{_ICMP6_DATA}16[0]"), "icmp6_seq": ("icmp6_hdr", f"{_ICMP6_DATA}16[1]"),
        "icmp6_maxdelay": ("icmp6_hdr", f"{_ICMP6_DATA}16[0]"),
        **{f"mld_{m}": ("mld_hdr", f"mld_icmp6_hdr.{p}") for m, p in _ICMP6.items()},
        "mld_maxdelay": ("mld_hdr", f"mld_icmp6_hdr.{_ICMP6_DATA}16[0]"),
        "mld_reserved": ("mld_hdr", f"mld_icmp6_hdr.{_ICMP6_DATA}16[1]"),
        **{f"nd_{s}_{m}": (tag, f"nd_{s}_hdr.{p}") for s, tag in (("rs", "nd_router_solicit"), ("ra", "nd_router_advert"),
           ("ns", "nd_neighbor_solicit"), ("na", "nd_neighbor_advert"), ("rd", "nd_redirect")) for m, p in _ICMP6.items()},
        "nd_rs_reserved": ("nd_router_solicit", f"nd_rs_hdr.{_ICMP6_DATA}32[0]"),
        "nd_ra_curhoplimit": ("nd_router_advert", f"nd_ra_hdr.{_ICMP6_DATA}8[0]"),
        "nd_ra_flags_reserved": ("nd_router_advert", f"nd_ra_hdr.{_ICMP6_DATA}8[1]"),
        "nd_ra_router_lifetime": ("nd_router_advert", f"nd_ra_hdr.{_ICMP6_DATA}16[1]"),
        "nd_ns_reserved": ("nd_neighbor_solicit", f"nd_ns_hdr.{_ICMP6_DATA}32[0]"),
        "nd_na_flags_reserved": ("nd_neighbor_advert", f"nd_na_hdr.{_ICMP6_DATA}32[0]"),
        "nd_rd_reserved": ("nd_redirect", f"nd_rd_hdr.{_ICMP6_DATA}32[0]"),
        **{f"rr_{m}": ("icmp6_router_renum", f"rr_hdr.{p}") for m, p in _ICMP6.items()},
        "rr_seqnum": ("icmp6_router_renum", f"rr_hdr.{_ICMP6_DATA}32[0]"),
    },
    "netinet/ip6.h": {
        "ip6_vfc": ("ip6_hdr", "ip6_ctlun.ip6_un2_vfc"),
        **{f"ip6_{m}": ("ip6_hdr", f"ip6_ctlun.ip6_un1.ip6_un1_{m}") for m in ("flow", "plen", "nxt", "hlim")},
        "ip6_hops": ("ip6_hdr", "ip6_ctlun.ip6_un1.ip6_un1_hlim"),
    },
    "netinet/if_ether.h": _ARP,
    "netinet/ether.h": _ARP,
    "net/route.h": {"rt_mss": ("rtentry", "rt_mtu")},
    # tftp(1)'s packet header, RFC 1350 section 5 (the opcode, then the block number or error code, then the data or
    # message): the member names of struct tftphdr of <arpa/tftp.h>, each a macro for a member of the nested unions the
    # layout table records (HEADERS-STAGE2-GAPS-E: inetutils' tftpsubs.c uses th_data; th_msg names the same member).
    "arpa/tftp.h": {
        "th_block": ("tftphdr", "th_u1.th_u2.th_u3.tu_block"),
        "th_code": ("tftphdr", "th_u1.th_u2.th_u3.tu_code"),
        "th_stuff": ("tftphdr", "th_u1.tu_stuff"),
        "th_data": ("tftphdr", "th_u1.th_u2.tu_data"),
        "th_msg": ("tftphdr", "th_u1.th_u2.tu_data"),
    },
    "sys/msg.h": {"msg_cbytes": ("msqid_ds", "__msg_cbytes")},
}
def _fake(i, path, n):
    """`struct niji_mn<i>` with only `path`, whose leaf is `char[n]`, or for `p[k]` an array of k + 2 `char[n]`
    structures (so the element and the array differ in size)."""
    comps = path.split(".")
    leaf, k = comps[-1], None
    if leaf.endswith("]"):
        leaf, k = leaf[:-1].split("[")
        s = f"struct {{ char c[{n}]; }} {leaf}[{int(k) + 2}];"
    else:
        s = f"char {leaf}[{n}];"
    for c in reversed(comps[:-1]):
        s = f"struct {{ {s} }} {c};"
    return f"struct niji_mn{i} {{ {s} }};", k


def member_names(ctx):
    """Member-name macros (no spec clause beyond the man pages and RFCs MEMBER_MACROS names; HEADERS-BUSYBOX-GAPS,
    HEADERS-NET-SYS-NAMES): in each header and configuration where visibility.tsv shows NAME, it is a macro for exactly
    the nested member MEMBER_MACROS records, so a use resolves in a structure that has only that path (the same offset
    alone would accept `ifr_ifindex` as `ifru_mtu`, or `icmp6_seq` as element 0), and its offset in the real tag is the
    nested member's (for an element `[k]`, the array's offset plus k elements); where the header is usable but does not
    show NAME, it does not define it."""
    offs = {(r["type"], r["member"]): r["offset_or_size"] for r in ctx.rows("spec/abi/layouts.tsv") if r["member"]}
    res = []
    for hdr, table in MEMBER_MACROS.items():
        rows = [r for r in ctx.vis if r["header"] == hdr and r["name"] in table]
        if {r["name"] for r in rows} != set(table):
            res.append((ctx.ids[0], f"member names {hdr}", "#error visibility.tsv and MEMBER_MACROS name different sets\n"))
            continue
        shown = {}
        for r in rows:
            for cid in expand(r["configs"], ctx.ids):
                shown.setdefault(cid, []).append(r["name"])
        usable = {cid for r in ctx.vis if r["header"] == hdr and r["kind"] != "include-error"
                  for cid in expand(r["configs"], ctx.ids)}
        for cid in ctx.ids:
            names = sorted(shown.get(cid, []))
            absent = sorted(set(table) - set(names))
            if absent and cid in usable:
                body = f"#include <{hdr}>\n" + "".join(f"#ifdef {n}\n#error {n} is defined where visibility.tsv does not show it\n#endif\n" for n in absent)
                res.append((cid, f"member names {hdr} (not shown)", body))
            if not names:
                continue
            body = [f"#include <{hdr}>"]
            for i, n in enumerate(names):
                tag, path = table[n]
                fake, k = _fake(i, path, i + 2)
                base = path.split("[")[0]
                real = offs[("struct " + tag, base)] + (f" + {k} * sizeof(((struct {tag} *)0)->{base}[0])" if k else "")
                body += [fake, f"typedef char niji_mn{i}_form[sizeof(((struct niji_mn{i} *)0)->{n}) == {i + 2} ? 1 : -1];",
                         f"typedef char niji_mn{i}_foff[__builtin_offsetof(struct niji_mn{i}, {n}) == {int(k or 0) * (i + 2)} ? 1 : -1];",
                         f"typedef char niji_mn{i}_off[__builtin_offsetof(struct {tag}, {n}) == {real} ? 1 : -1];"]
            res.append((cid, f"member names {hdr}", "\n".join(body) + "\n"))
    return res


def units(ctx):
    return member_names(ctx)
