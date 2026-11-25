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


NAMESER_ACCESSORS = ("ns_msg_id", "ns_msg_base", "ns_msg_end", "ns_msg_size", "ns_msg_count", "ns_rr_name", "ns_rr_type",
                     "ns_rr_class", "ns_rr_ttl", "ns_rr_rdlen", "ns_rr_rdata", "NS_GET16", "NS_GET32", "NS_PUT16",
                     "NS_PUT32", "GETSHORT", "GETLONG", "PUTSHORT", "PUTLONG")
# Each accessor's type in the reference (black-box form probe, HEADERS-STAGE2-GAPS-E); every one is a prvalue there.
NAMESER_FORMS = (("ns_msg_id (niji_h)", "int"), ("ns_msg_base (niji_h)", "const unsigned char *"),
                 ("ns_msg_end (niji_h)", "const unsigned char *"), ("ns_msg_size (niji_h)", "long"),
                 ("ns_msg_count (niji_h, ns_s_an)", "int"), ("ns_rr_name (niji_rr)", "const char * (C: char *)"),
                 ("ns_rr_type (niji_rr)", "ns_type"), ("ns_rr_class (niji_rr)", "ns_class"),
                 ("ns_rr_ttl (niji_rr)", "unsigned int"), ("ns_rr_rdlen (niji_rr)", "int"),
                 ("ns_rr_rdata (niji_rr)", "const unsigned char *"))
# C++14 on: the values, evaluated at compile time (each check returns its own number when it fails). The byte-order
# macros read and write big-endian (RFC 1035 section 2.3.2), keep the low 16 or 32 bits and advance the pointer.
NAMESER_VALUES = """constexpr int niji_ns_values () {
  ns_msg h {}; ns_rr rr {};
  unsigned char b[8] = {0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0}, o[8] = {};
  h._msg = b; h._eom = b + 6; h._id = 0x1234; h._counts[0] = 11; h._counts[2] = 33;
  if (ns_msg_id (h) != 0x1234 || ns_msg_base (h) != b || ns_msg_end (h) != b + 6 || ns_msg_size (h) != 6) return 1;
  if (ns_msg_count (h, ns_s_qd) != 11 || ns_msg_count (h, ns_s_ns) != 33 || ns_msg_count (h, 1 ? 2 : 0) != 33) return 2;
  rr.type = 18; rr.rr_class = 1; rr.ttl = 300; rr.rdlength = 7; rr.rdata = b + 3;
  if (ns_rr_type (rr) != ns_t_afsdb || ns_rr_class (rr) != ns_c_in || ns_rr_ttl (rr) != 300 || ns_rr_rdlen (rr) != 7
      || ns_rr_rdata (rr) != b + 3) return 3;
  if (ns_rr_name (rr)[0] != '.' || ns_rr_name (rr)[1] != 0) return 4;
  rr.name[0] = 'a';
  if (ns_rr_name (rr) != rr.name) return 5;
  const unsigned char *cp = b; unsigned int s = 0; unsigned long l = 0;
  NS_GET16 (s, cp); NS_GET32 (l, cp);
  if (s != 0x1234 || l != 0x56789abcUL || cp != b + 6) return 6;
  unsigned char *wp = o;
  NS_PUT16 (0xa1b2c3, wp); NS_PUT32 (0x1122334455ULL, wp);
  if (o[0] != 0xb2 || o[1] != 0xc3 || o[2] != 0x22 || o[3] != 0x33 || o[4] != 0x44 || o[5] != 0x55 || o[6] || wp != o + 6) return 7;
  cp = b + 2; GETSHORT (s, cp); GETLONG (l, cp);
  if (s != 0x5678 || l != 0x9abcdef0UL || cp != b + 8) return 8;
  wp = o; PUTSHORT (0x0102, wp); PUTLONG (0x03040506, wp);
  if (o[0] != 1 || o[1] != 2 || o[2] != 3 || o[5] != 6 || wp != o + 6) return 9;
  return 0;
}
static_assert (niji_ns_values () == 0, "the arpa/nameser.h accessors' values");
"""
# Each byte-order macro is one statement: `if (c) M (...); else ...` compiles (a bare block would end the if).
NAMESER_STATEMENTS = """void niji_ns_stmt (int c, const unsigned char *p, unsigned char *w, unsigned int *s, unsigned long *l) {
  if (c) NS_GET16 (*s, p); else NS_GET32 (*l, p);
  if (c) NS_PUT16 (*s, w); else NS_PUT32 (*l, w);
  if (c) GETSHORT (*s, p); else GETLONG (*l, p);
  if (c) PUTSHORT (*s, w); else PUTLONG (*l, w);
}
"""


def nameser_accessor_units(ctx):
    """resolver(3) and RFC 1035's message layout (no spec clause beyond them; HEADERS-STAGE2-GAPS-E: keyutils'
    dns.afsdb.c counts answers with ns_msg_count): the ns_msg_*/ns_rr_* accessors and the byte-order macros of
    <arpa/nameser.h> exactly where the reference shows them; where shown, each accessor has the reference's type as a
    prvalue (C++11 on `decltype ((e))`: an lvalue would let `ns_msg_id (h) = 1` compile), the byte-order macros are
    single statements, and (C++17) the values: section counts by index, the empty owner name read as ".", big-endian
    reads and writes that keep the low bits and advance the pointer. Defects it catches: a missing accessor, an
    accessor that is an lvalue or has another type, a byte-order macro that is a brace block or reads little-endian."""
    out = ctx.exact("arpa/nameser.h", NAMESER_ACCESSORS)
    row = next((r for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["name"] == "ns_msg_id"), None)
    if row is None:
        return out + [(ctx.ids[0], "nameser accessors", "#error visibility.tsv does not show ns_msg_id\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    for cid in expand(row["configs"], ctx.ids):
        cxx = lang[cid] == "c++"
        body = "#include <arpa/nameser.h>\nextern ns_msg niji_h;\nextern ns_rr niji_rr;\n"
        for i, (e, t) in enumerate(NAMESER_FORMS):
            # The empty name's "." is a `char[2]` in C, so the C conditional is `char *` (C probe of the reference).
            t = t.split(" (C: ")[0] if cxx else t.split(" (C: ")[-1].rstrip(")")
            same = (f"__is_same (__typeof__ ({e}), {t})" if cid == "cxx98" else f"__is_same (decltype (({e})), {t})") if cxx \
                else f"__builtin_types_compatible_p (__typeof__ ({e}), {t})"
            body += f"typedef char niji_ns_form{i}[{same} ? 1 : -1];\n"
        if cxx:
            body += "extern \"C\" {\n" + NAMESER_STATEMENTS + "}\n"
        else:
            body += NAMESER_STATEMENTS
        if cid == "cxx17":
            body += NAMESER_VALUES
        out.append((cid, "nameser accessors arpa/nameser.h", body))
    return out


def ipproto_max_units(ctx):
    """RFC 5237's protocol number space and ip(7) (no spec clause beyond them; HEADERS-STAGE2-GAPS-E: libnftnl and
    libnetfilter-conntrack size tables by IPPROTO_MAX): where visibility.tsv shows it, <netinet/in.h>'s IPPROTO_MAX is
    an enumerator (the reference defines no macro of that name: `#ifdef IPPROTO_MAX` is false there) with its
    enumerators.tsv value, usable as an array bound."""
    val = next((r["value"] for r in ctx.rows("spec/abi/enumerators.tsv") if r["name"] == "IPPROTO_MAX"), None)
    row = next((r for r in ctx.vis if r["header"] == "netinet/in.h" and r["name"] == "IPPROTO_MAX"), None)
    if val is None or row is None or row["kind"] != "enumerator":
        return [(ctx.ids[0], "IPPROTO_MAX netinet/in.h", "#error the tables do not show IPPROTO_MAX as an enumerator\n")]
    body = ("#include <netinet/in.h>\n#ifdef IPPROTO_MAX\n#error IPPROTO_MAX is a macro\n#endif\n"
            f"typedef char niji_ipm[IPPROTO_MAX == {val} ? 1 : -1];\nextern int niji_ipm_table[IPPROTO_MAX];\n")
    return [(cid, "IPPROTO_MAX netinet/in.h", body) for cid in expand(row["configs"], ctx.ids)]


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
    res += member_names(ctx) + nameser_enum_units(ctx) + nameser_accessor_units(ctx) + ipproto_max_units(ctx)
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


# Member-name macros: a name for a member nested in a structure's union or inner structure, each a macro for exactly
# that nested member (black-box form probes: HEADERS-BUSYBOX-GAPS for <net/if.h>, poison-after-header; HEADERS-NET-SYS-
# NAMES for the others, a unit whose `struct niji_f` has only the candidate path compiles a use of the name with the
# reference's headers exactly for its expansion). header: {name: (tag, the nested member it names; `[k]` an element)}.
# The names of one table come from the man page or RFC named in the header's template: netdevice(7) (<net/if.h>),
# RFC 792 (<netinet/ip_icmp.h>), RFC 4443, 2710, 4861 and 2894 (<netinet/icmp6.h>), RFC 8200 (<netinet/ip6.h>),
# RFC 826 (<netinet/if_ether.h>, also shown by <netinet/ether.h>), route(8) (<net/route.h>), msgctl(2) (<sys/msg.h>).
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
    },
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
