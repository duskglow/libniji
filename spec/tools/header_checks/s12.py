"""Slice 12 (plan/headers.md section 4: network core and name services): P1 of its sixteen headers with member offsets,
and P10, the network macros' output against the build host's own headers.

* P1  (`units(ctx)`): per (header, configuration) one unit that includes the header and asserts, for what it makes visible
  there, every macros.tsv integer value, every typedef and structure size and alignment, and the offset of every
  structure member that is not a bit-field. Names another slice's header owns are left to that header's P1 (signal.h and
  sys/param.h belong to slices 7 and 8, which are not in this tree yet).
* Member names (`member_names`): the member-name macros of MEMBER_MACROS (`ifr_mtu`, `icmp_id`, `ip6_nxt`, ...), one
  table for the headers of slices 12 to 14 that have them, as their form is one pattern.
* P10 (`python3 spec/tools/header_checks/s12.py [--include DIR] [--work DIR]`): spec/tools/p10_net.c (CMSG_*,
  IN6_IS_ADDR_*, the byte-order conversions) built on the system's headers and on libniji's (`-nostdinc`) must print the
  same bytes. The system's output is a diagnostic comparison, never read as text.
* enum (`nameser_enum_units`): the `enum __ns_*` enumerations of <arpa/nameser.h>, their members and typedefs.
Loaded by spec/tools/header_check.py, which lists this directory (plan/headers.md section 4, "Adding a slice").
"""
import os
import subprocess
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
P8_CONFIGS = ("gnu17", "default", "gnu", "c11", "cxx17")
INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


def mine():
    """The headers of headers/slices/s12.tsv."""
    with open(os.path.join(ROOT, "headers", "slices", "s12.tsv")) as f:
        return [l.split("\t")[0] for l in f if l.strip() and not l.startswith("#")][1:]


def nameser_enum_units(ctx):
    """No spec clause beyond RFC 1035's field values (the type of the names by black-box probe, HEADERS-ENUM-FOLLOWUPS):
    per configuration that shows them, every enumeration `enum __ns_*` of <arpa/nameser.h> is declared with each of its
    enumerators.tsv members at its value, a member converts to its enumeration without a cast (C++: an int or a
    structure in its place fails), the typedefs `ns_type`, `ns_class`, `ns_sect` name those enumerations, and a DNS
    style switch over `ns_class` compiles under -Wall -Werror. Defects it catches: an enumeration left opaque, an
    enumeration of a reserved tag dropped with all its public members (`ns_r_servfail`), a wrong value."""
    tags = sorted(r["name"] for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["kind"] == "enum" and r["name"].startswith("__ns_"))
    members = {}
    for r in ctx.rows("spec/abi/enumerators.tsv"):
        if r["tag"].startswith("enum ") and r["tag"][5:] in tags:
            members.setdefault(r["tag"][5:], []).append((r["name"], r["value"]))
    shown = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["kind"] in ("enumerator", "enum", "typedef")}
    if len(tags) < 8 or set(members) != set(tags) or not {"ns_type", "ns_class", "ns_sect"} <= set(shown):
        return [(ctx.ids[0], "enum arpa/nameser.h", "#error the tables do not show the eight __ns_* enumerations and their typedefs\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    res = []
    for cid in ctx.ids:
        if cid not in expand(shown["ns_type"], ctx.ids):
            continue
        body = ["#include <arpa/nameser.h>", "ns_type niji_t = ns_t_a;", "ns_sect niji_s = ns_s_an;",
                "int niji_ns_f (ns_class c) { switch (c) { case ns_c_in: return 1; case ns_c_chaos: return 3; default: return 0; } }"]
        for tag in tags:
            for i, (n, v) in enumerate(members[tag]):
                if cid not in expand(shown.get(n, ""), ctx.ids):
                    continue
                body += [f"enum {tag} niji_{tag}_{i} = {n};", f"typedef char niji_v{tag}_{i}[({n}) == ({v}) ? 1 : -1];"]
            if lang[cid] == "c++":
                body.append(f"typedef char niji_k{tag}[__is_enum (enum {tag}) ? 1 : -1];")
        res.append((cid, "enum arpa/nameser.h", "\n".join(body) + "\n"))
    return res


def expand(configs, ids):
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def units(ctx):
    own = set(mine())
    others = {r["header"] for r in ctx.slice_rows if r["header"] not in own} | {"signal.h", "sys/param.h"}
    foreign = {r["name"] for r in ctx.vis if r["header"] in others}
    members = {}
    for r in ctx.rows("spec/abi/layouts.tsv"):
        if r["member"] and not r["bitfield"] and r["offset_or_size"].isdigit():
            members.setdefault(r["type"], []).append(r)
    out = {}
    for r in ctx.vis:
        h, n = r["header"], r["name"]
        if h not in own or r["kind"] not in ("macro", "typedef", "struct") or n in foreign:
            continue
        if n.startswith("__") or (n.startswith("_") and n.endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        tests = []
        if r["kind"] == "macro" and n in ctx.macros and ctx.macros[n]["type"] in INT_SUFFIX and ctx.macros[n]["header"] == h:
            m = ctx.macros[n]
            tests.append(f"(({n}) == ({int(m['value'])}{INT_SUFFIX[m['type']]}))")
        elif r["kind"] == "typedef" and n in ctx.typedefs and ctx.typedefs[n]["size"]:
            t = ctx.typedefs[n]
            tests += [f"sizeof({n}) == {t['size']}", f"__alignof__({n}) == {t['align']}"]
        elif r["kind"] == "struct" and ("struct " + n) in ctx.layouts:
            t = ctx.layouts["struct " + n]
            tests += [f"sizeof(struct {n}) == {t['offset_or_size']}", f"__alignof__(struct {n}) == {t['align']}"]
            tests += [f"__builtin_offsetof(struct {n}, {m['member']}) == {m['offset_or_size']}" for m in members.get("struct " + n, [])]
        for cid in expand(r["configs"], ctx.ids):
            out.setdefault((h, cid), []).extend(tests)
    res = []
    for (h, cid), tests in out.items():
        if tests:
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
            res.append((cid, f"P1 {h}", body))
    res += member_names(ctx) + nameser_enum_units(ctx) + stage2_units(ctx)
    # P8, libc-first only (ruling #6458: the kernel header first is not supported; slice 13 measures it): <netinet/in.h>
    # with linux/in6.h, once, with the shared tags used so a missing or doubled definition fails to compile. The other
    # kernel pairs are the `uapi` cells of headers/slices/s12.tsv, both orders.
    for cid in P8_CONFIGS:
        if cid in ctx.ids:
            res.append((cid, "P8 netinet/in.h then linux/in6.h", "#include <netinet/in.h>\n#include <linux/in6.h>\n"
                        "struct in_addr a; struct in6_addr b; struct sockaddr_in c; struct sockaddr_in6 d;\nint niji_p8_tu;\n"))
    # Spec clause: RFC 1035 4.1.1 (HEADERS-APT-GAPS: apt's SRV lookup): the DNS message header type `HEADER` exactly
    # where the reference shows it, with its size and alignment (P1 above; its bit positions are the rows test's).
    for h in ("arpa/nameser.h", "arpa/nameser_compat.h"):
        res += ctx.exact(h, ["HEADER"])
    # Spec clause: XSH <sys/socket.h> (accept, bind, connect, getpeername, getsockname, recvfrom, sendto; GNU accept4):
    # the address argument takes a `struct sockaddr *` (`const` for the const form) and a null pointer, in C++ too,
    # where the C transparent union does not exist (HEADERS-APT-GAPS: apt's methods, any C++ socket program).
    cfg = {c["id"]: c for c in ctx.cfgs}
    for cid in ctx.ids:
        if cfg[cid]["language"] != "c++":
            continue
        res.append((cid, "C++ socket calls with struct sockaddr *",
                    "#include <sys/socket.h>\n#include <netinet/in.h>\n"
                    "int niji_sock (int s, struct sockaddr *p, const struct sockaddr *c, socklen_t l, socklen_t *pl)\n"
                    "{\n  return connect (s, p, l) + connect (s, c, l) + bind (s, p, l) + bind (s, c, l)\n"
                    "    + accept (s, 0, 0) + accept (s, p, pl) + accept4 (s, p, pl, 0) + accept4 (s, 0, 0, 0)\n"
                    "    + getsockname (s, p, pl) + getpeername (s, p, pl)\n"
                    "    + (int) recvfrom (s, 0, 0, 0, p, pl) + (int) recvfrom (s, 0, 0, 0, 0, 0)\n"
                    "    + (int) sendto (s, 0, 0, 0, p, l) + (int) sendto (s, 0, 0, 0, c, l) + (int) sendto (s, 0, 0, 0, 0, 0);\n}\n"))
    return res


# The address-class tests of <netinet/in.h> (HEADERS-STAGE2-GAPS-D, glib's ginetaddress.c uses IN_MULTICAST): name:
# ((address, expected truth) ...). Each class boundary from both sides, and -1 (an int argument: the test reads the
# address as 32 unsigned bits). Values from a black-box probe of the reference over all 256 top bytes.
IN_CLASS_CASES = {
    "IN_CLASSA": (("0x7f000001u", 1), ("0x80000000u", 0), ("0", 1), ("-1", 0)),
    "IN_CLASSB": (("0x80000000u", 1), ("0xbfffffffu", 1), ("0xc0000000u", 0), ("0x7fffffffu", 0)),
    "IN_CLASSC": (("0xc0000000u", 1), ("0xdfffffffu", 1), ("0xe0000000u", 0), ("0xbfffffffu", 0)),
    "IN_CLASSD": (("0xe0000001u", 1), ("0xefffffffu", 1), ("0xf0000000u", 0), ("0xdfffffffu", 0)),
    "IN_MULTICAST": (("0xe0000001u", 1), ("0xefffffffu", 1), ("0xf0000000u", 0), ("0xdfffffffu", 0)),
    "IN_EXPERIMENTAL": (("0xe0000000u", 1), ("-1", 1), ("0xdfffffffu", 0), ("0x7f000001u", 0)),
    "IN_BADCLASS": (("0xf0000000u", 1), ("-1", 1), ("0xefffffffu", 0), ("0x7fffffffu", 0)),
}
# The members of the GNU C transparent union `__CONST_SOCKADDR_ARG` / `__SOCKADDR_ARG` as the reference names them
# (black-box probe by name and pointer type; fakechroot's bind wrapper reads `addr.__sockaddr__`).
SOCKADDR_UNION_TAGS = ("sockaddr", "sockaddr_at", "sockaddr_ax25", "sockaddr_dl", "sockaddr_eon", "sockaddr_in",
                       "sockaddr_in6", "sockaddr_inarp", "sockaddr_ipx", "sockaddr_iso", "sockaddr_ns", "sockaddr_un",
                       "sockaddr_x25")
# RFC 3678 section 4.1.3's source-filter request (perl's Configure tests for it): the layout the reference has (black-box
# probe), written here rather than read from layouts.tsv, which also generates the structure.
IP_MREQ_SOURCE = ("sizeof (struct ip_mreq_source) == 12", "__alignof__ (struct ip_mreq_source) == 4",
                  "__builtin_offsetof (struct ip_mreq_source, imr_multiaddr) == 0",
                  "__builtin_offsetof (struct ip_mreq_source, imr_interface) == 4",
                  "__builtin_offsetof (struct ip_mreq_source, imr_sourceaddr) == 8")


def stage2_units(ctx):
    """Yohane stage 2 (HEADERS-STAGE2-GAPS-D). Spec clause: RFC 1112 section 4 and RFC 791 section 3.2 (address
    classes): where visibility.tsv shows IN_CLASSA ... IN_BADCLASS in <netinet/in.h>, each is defined, an integer
    constant expression with IN_CLASS_CASES' truth values, of type int in C. Defects: a missing macro (glib), a class
    mask off by one bit, a test that sign-extends an int argument. No spec clause (GNU C socket calls, black-box
    probe): where the union typedefs are shown, every SOCKADDR_UNION_TAGS member `__<tag>__` of both unions has the
    pointer type (`const` in the const union), so a wrapper that names a member compiles (fakechroot). No spec clause
    beyond netdb.h's 4.2BSD name (perl, heimdal): where `h_addr` is shown it names `h_addr_list[0]` (in a structure
    whose h_addr_list is an array it is one pointer at offset 0, not the array or a later element), and the header
    shows exactly that set. RFC 3678: where
    struct ip_mreq_source is shown in <netinet/in.h>, it has IP_MREQ_SOURCE's size, alignment and member offsets.
    POSIX <netinet/in.h> (IN6ADDR_ANY_INIT, IN6ADDR_LOOPBACK_INIT; perl's Socket module): where shown, each initializes
    a struct in6_addr and a structure of one union of 16 bytes with no missing-braces warning (braced for structure,
    union and array, at most 16 elements, no designators: the reference's form by black-box probe), and in C++11 and
    later the 16 bytes are :: and ::1. Defects: a missing macro, a flat brace list, the 1 in the wrong byte."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    vis = {(r["header"], r["name"], r["kind"]): r["configs"] for r in ctx.vis}
    res = (ctx.exact("netinet/in.h", list(IN_CLASS_CASES) + ["IN6ADDR_ANY_INIT", "IN6ADDR_LOOPBACK_INIT"])
           + ctx.exact("netdb.h", ["h_addr"]))
    for cid in ctx.ids:
        shown = [n for n in IN_CLASS_CASES if cid in expand(vis.get(("netinet/in.h", n, "macro"), ""), ctx.ids)]
        if shown:
            body = ["#include <netinet/in.h>"]
            for n in shown:
                for i, (a, want) in enumerate(IN_CLASS_CASES[n]):
                    body.append(f"typedef char niji_{n}_{i}[(!!{n} ({a})) == {want} ? 1 : -1];")
                if lang[cid] == "c":
                    body.append(f"typedef char niji_{n}_t[__builtin_types_compatible_p (__typeof__ ({n} (0u)), int) ? 1 : -1];")
            res.append((cid, "address classes netinet/in.h", "\n".join(body) + "\n"))
        if cid in expand(vis.get(("netdb.h", "h_addr", "macro"), ""), ctx.ids):
            res.append((cid, "h_addr netdb.h", "#include <netdb.h>\nstruct niji_h { char *h_addr_list[3]; };\n"
                        "typedef char niji_h_form[sizeof (((struct niji_h *) 0)->h_addr) == sizeof (char *) ? 1 : -1];\n"
                        "typedef char niji_h_off[__builtin_offsetof (struct niji_h, h_addr) == 0 ? 1 : -1];\n"
                        "char *niji_h_get (struct hostent *h) { return h->h_addr; }\n"))
        if cid in expand(vis.get(("netinet/in.h", "ip_mreq_source", "struct"), ""), ctx.ids):
            res.append((cid, "ip_mreq_source netinet/in.h", "#include <netinet/in.h>\n" + "".join(
                f"typedef char niji_ims{i}[{t} ? 1 : -1];\n" for i, t in enumerate(IP_MREQ_SOURCE))))
        if cid in expand(vis.get(("netinet/in.h", "IN6ADDR_LOOPBACK_INIT", "macro"), ""), ctx.ids):
            body = ["#include <netinet/in.h>", '#pragma GCC diagnostic error "-Wmissing-braces"',
                    "struct in6_addr niji_any = IN6ADDR_ANY_INIT, niji_lo = IN6ADDR_LOOPBACK_INIT;",
                    "struct niji_s { union { unsigned char b[16]; } u; };",
                    "struct niji_s niji_s_any = IN6ADDR_ANY_INIT, niji_s_lo = IN6ADDR_LOOPBACK_INIT;"]
            if lang[cid] != "c" and cid != "cxx98":
                body.append("constexpr niji_s niji_c_any = IN6ADDR_ANY_INIT, niji_c_lo = IN6ADDR_LOOPBACK_INIT;")
                body += [f'static_assert (niji_c_any.u.b[{i}] == 0 && niji_c_lo.u.b[{i}] == {int(i == 15)}, "byte {i}");'
                         for i in range(16)]
            res.append((cid, "in6addr initializers netinet/in.h", "\n".join(body) + "\n"))
        for u, q in (("__CONST_SOCKADDR_ARG", "const "), ("__SOCKADDR_ARG", "")):
            if lang[cid] != "c" or cid not in expand(vis.get(("sys/socket.h", u, "typedef"), ""), ctx.ids):
                continue
            body = ["#include <sys/socket.h>", f"{u} niji_u;"]
            body += [f"typedef char niji_{t}[__builtin_types_compatible_p (__typeof__ (niji_u.__{t}__), {q}struct {t} *) ? 1 : -1];"
                     for t in SOCKADDR_UNION_TAGS]
            body.append(f"const void *niji_wrap ({u} a) {{ return a.__sockaddr__; }}")
            res.append((cid, f"union members {u} sys/socket.h", "\n".join(body) + "\n"))
    return res


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


def p10(inc, work):
    src = os.path.join(ROOT, "spec", "tools", "p10_net.c")
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    outs = {}
    for tag, extra in (("reference", []), ("libniji", ["-nostdinc", "-isystem", inc, "-isystem", gccinc])):
        exe = os.path.join(work, "p10_" + tag)
        c = subprocess.run(["gcc", "-std=gnu17", "-O2", "-Wall", "-Wextra", "-Werror", *extra, src, "-o", exe], capture_output=True, text=True)
        if c.returncode:
            print(f"FAIL P10 {tag} build: {' | '.join(c.stderr.strip().splitlines()[:3])[:400]}")
            return 1
        outs[tag] = subprocess.run([exe], capture_output=True, text=True).stdout
    ok = outs["reference"] == outs["libniji"] and outs["reference"].count("\n") > 1000
    print(f"P10 net macros: {outs['reference'].count(chr(10))} lines, {'identical' if ok else 'DIFFERENT'}")
    if not ok:
        for i, (x, y) in enumerate(zip(outs["reference"].splitlines(), outs["libniji"].splitlines())):
            if x != y:
                print(f"FAIL P10 line {i}: reference `{x}` libniji `{y}`")
                break
    return 0 if ok else 1


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser(description="P10 of slice 12")
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--work", default="")
    a = ap.parse_args()
    w = a.work or tempfile.mkdtemp(prefix="p10.")
    os.makedirs(w, exist_ok=True)
    sys.exit(p10(os.path.abspath(a.include), w))
