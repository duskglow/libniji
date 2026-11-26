"""Slice 12 checks: stage2-gaps-d (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py; header_checks/README.md)."""
from header_check import expand


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


def units(ctx):
    return stage2_units(ctx)
