"""Slice 12 checks: initializers (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py; header_checks/README.md)."""
from header_check import expand


def initializers(ctx):
    """RFC 3493 section 3.8 and the resolver interface version (no spec clause; HEADERS-STAGE2-GAPS-B): where
    netinet/in.h shows them, IN6ADDR_ANY_INIT and IN6ADDR_LOOPBACK_INIT initialize a struct in6_addr (C, and a table
    of them), and in C++11 and later their bytes are :: and ::1 (a constant expression); where resolv.h shows `__RES`,
    the preprocessor sees macros.tsv's value."""
    res = []
    init = {r["name"]: r for r in ctx.vis if r["header"] == "netinet/in.h" and r["name"] in ("IN6ADDR_ANY_INIT", "IN6ADDR_LOOPBACK_INIT")}
    ver = next((r for r in ctx.vis if r["header"] == "resolv.h" and r["name"] == "__RES" and r["kind"] == "macro"), None)
    if len(init) != 2 or ver is None or "__RES" not in ctx.macros:
        return [(ctx.ids[0], "initializers netinet/in.h", "#error visibility.tsv or macros.tsv lacks IN6ADDR_*_INIT or __RES\n")]
    for cid in ctx.ids:
        if all(cid in expand(r["configs"], ctx.ids) for r in init.values()):
            body = ["#include <netinet/in.h>",
                    "static const struct in6_addr niji_any = IN6ADDR_ANY_INIT, niji_lo = IN6ADDR_LOOPBACK_INIT;",
                    "struct in6_addr niji_tab[] = { IN6ADDR_ANY_INIT, IN6ADDR_LOOPBACK_INIT };",
                    "typedef char niji_tab_n[sizeof niji_tab == 2 * sizeof (struct in6_addr) ? 1 : -1];",
                    "const struct in6_addr *niji_use[] = { &niji_any, &niji_lo };"]
            if cid.startswith("cxx") and cid != "cxx98":
                body += ["constexpr struct in6_addr niji_ca = IN6ADDR_ANY_INIT, niji_cl = IN6ADDR_LOOPBACK_INIT;"]
                body += [f"static_assert (niji_ca.s6_addr[{i}] == 0 && niji_cl.s6_addr[{i}] == {int(i == 15)}, \"byte {i}\");"
                         for i in range(16)]
            res.append((cid, "initializers netinet/in.h", "\n".join(body) + "\n"))
        if cid in expand(ver["configs"], ctx.ids):
            res.append((cid, "initializers resolv.h __RES",
                        f"#include <resolv.h>\n#if !defined __RES || __RES != {ctx.macros['__RES']['value']}\n#error __RES\n#endif\nint niji_res;\n"))
    # resolv.h's base64 helpers (no spec clause; HEADERS-STAGE2-GAPS-B-3): where resolv.h shows the macros b64_ntop and
    # b64_pton, each names the exported function, so a call with the resolver's argument types compiles and the
    # function-pointer types are exactly the exported functions' (the macros are the unprefixed names programs use).
    b64 = {r["name"]: r for r in ctx.vis if r["header"] == "resolv.h" and r["name"] in ("b64_ntop", "b64_pton") and r["kind"] == "macro"}
    if len(b64) != 2:
        return res + [(ctx.ids[0], "initializers resolv.h b64", "#error visibility.tsv lacks the macros b64_ntop and b64_pton\n")]
    for cid in expand(b64["b64_ntop"]["configs"], ctx.ids):
        if cid in expand(b64["b64_pton"]["configs"], ctx.ids):
            res.append((cid, "initializers resolv.h b64",
                        "#include <resolv.h>\n"
                        "#ifndef b64_ntop\n#error b64_ntop\n#endif\n#ifndef b64_pton\n#error b64_pton\n#endif\n"
                        "int (*niji_enc) (const unsigned char *, size_t, char *, size_t) = b64_ntop;\n"
                        "int (*niji_dec) (const char *, unsigned char *, size_t) = b64_pton;\n"
                        "int niji_b64 (const unsigned char *s, size_t n, char *d, size_t m, unsigned char *o)\n"
                        "{\n  int e = b64_ntop (s, n, d, m);\n  return e < 0 ? e : b64_pton (d, o, n);\n}\n"))
    # ELF gABI, e_ident[EI_MAG0..EI_MAG3] (HEADERS-STAGE2-GAPS-B): where elf.h shows ELFMAG, it is a string literal of
    # SELFMAG bytes (so `char m[] = ELFMAG` and memcmp(e_ident, ELFMAG, SELFMAG) work), and in C++11 and later its
    # bytes are ELFMAG0..ELFMAG3 (whose values the P1 units check) then the terminator.
    mag = next((r for r in ctx.vis if r["header"] == "elf.h" and r["name"] == "ELFMAG" and r["kind"] == "macro"), None)
    if mag is None or ctx.macros.get("ELFMAG", {}).get("type") != "char[5]":
        return res + [(ctx.ids[0], "initializers elf.h ELFMAG", "#error visibility.tsv or macros.tsv lacks ELFMAG (char[5])\n")]
    for cid in expand(mag["configs"], ctx.ids):
        body = ["#include <elf.h>", "static const char niji_mag[] = ELFMAG;",
                "typedef char niji_mag_n[sizeof niji_mag == SELFMAG + 1 && sizeof ELFMAG == SELFMAG + 1 ? 1 : -1];",
                "const char *niji_mag_use = niji_mag;"]
        if cid.startswith("cxx") and cid != "cxx98":
            body += ["static_assert (ELFMAG[0] == ELFMAG0 && ELFMAG[1] == ELFMAG1 && ELFMAG[2] == ELFMAG2"
                     " && ELFMAG[3] == ELFMAG3 && ELFMAG[4] == 0, \"ELFMAG bytes\");"]
        res.append((cid, "initializers elf.h ELFMAG", "\n".join(body) + "\n"))
    return res


def units(ctx):
    return initializers(ctx)
