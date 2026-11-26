"""Slice 13 checks: stage2-gaps-f (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# The name tables of <arpa/telnet.h> and their switches: table -> (switch that defines it, entries with the final
# null). Black-box probe of the reference (program output): RFC 854's commands EOF .. IAC (20), the options BINARY ..
# NEW-ENVIRON (40), the line-mode special characters (RFC 1184, 0 .. FORW2, 19), RFC 2941's authentication types
# (NULL .. MINK, 5) and RFC 2946's encryption commands (IS .. DEC-KEYID, 9) and types (ANY .. DES_OFB64, 3).
TABLES = {"authtype_names": ("AUTH_NAMES", 6), "encrypt_names": ("ENCRYPT_NAMES", 10),
          "enctype_names": ("ENCRYPT_NAMES", 4), "telcmds": ("TELCMDS", 21), "telopts": ("TELOPTS", 41),
          "slc_names": ("SLC_NAMES", 20)}
# Lookup macros without a switch, and those TELOPTS adds (exact: defined only with it).
MACROS = ("AUTHTYPE_NAME", "AUTHTYPE_NAME_OK", "ENCRYPT_NAME", "ENCRYPT_NAME_OK", "ENCTYPE_NAME", "ENCTYPE_NAME_OK",
          "TELCMD", "TELCMD_OK", "SLC_NAME", "SLC_NAME_OK", "SLC_NAMELIST", "SLC_NAMES")
TELOPTS_MACROS = ("TELOPT", "TELOPT_OK", "TELOPT_FIRST", "TELOPT_LAST")
# Range macros: (expression, truth). Each boundary from both sides, and -1 (the test converts to unsigned int).
OK_CASES = (("AUTHTYPE_NAME_OK (4)", 1), ("AUTHTYPE_NAME_OK (5)", 0), ("AUTHTYPE_NAME_OK (-1)", 0),
            ("ENCRYPT_NAME_OK (8)", 1), ("ENCRYPT_NAME_OK (9)", 0), ("ENCRYPT_NAME_OK (-1)", 0),
            ("ENCTYPE_NAME_OK (2)", 1), ("ENCTYPE_NAME_OK (3)", 0), ("ENCTYPE_NAME_OK (-1)", 0),
            ("TELCMD_OK (236)", 1), ("TELCMD_OK (255)", 1), ("TELCMD_OK (235)", 0), ("TELCMD_OK (-1)", 0),
            ("SLC_NAME_OK (18)", 1), ("SLC_NAME_OK (19)", 0), ("SLC_NAME_OK (-1)", 0))
TELOPT_CASES = (("TELOPT_FIRST == 0", 1), ("TELOPT_LAST == 39", 1), ("TELOPT_OK (39)", 1), ("TELOPT_OK (40)", 0),
                ("TELOPT_OK (-1)", 0))
SLC = ("0", "SYNCH", "BRK", "IP", "AO", "AYT", "EOR", "ABORT", "EOF", "SUSP", "EC", "EL", "EW", "RP", "LNEXT", "XON",
       "XOFF", "FORW1", "FORW2")


def units(ctx):
    """No spec clause beyond RFC 854, 1184, 2941 and 2946's names (HEADERS-STAGE2-GAPS-F: inetutils' libtelnet/auth.c
    and encrypt.c use AUTHTYPE_NAME and ENCTYPE_NAME): where visibility.tsv shows AUTHTYPE_NAME in <arpa/telnet.h>,
    (1) without switches: MACROS are defined and TELOPTS_MACROS are not, each table is declared `extern char *[]`
    (incomplete), each *_NAME macro indexes its table (TELCMD from TELCMD_FIRST), each *_OK macro is an integer
    constant expression of type int (bool in C++) with OK_CASES' truth values; (2) with every switch defined before
    the header, one translation unit: each table is defined with TABLES' entry count, TELOPTS adds TELOPT_CASES, and
    in C++11 on SLC_NAMELIST is the 19 SLC names and a null. Defects: a missing macro (inetutils), a range off by one,
    a signed comparison, a table defined without its switch (a duplicate definition in every program), an entry
    missing or extra, SLC_NAMELIST out of order."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    row = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "arpa/telnet.h"}
    res = ctx.exact("arpa/telnet.h", MACROS + TELOPTS_MACROS)
    for cid in ctx.ids:
        if cid not in expand(row.get("AUTHTYPE_NAME", ""), ctx.ids):
            continue
        cxx = lang[cid] == "c++"

        def same(a, b):
            return f"__is_same ({a}, {b})" if cxx else f"__builtin_types_compatible_p ({a}, {b})"

        okt = "bool" if cxx else "int"
        body = ["#include <arpa/telnet.h>"]
        body += [f"typedef char niji_tn_d_{t}[{same(f'__typeof__ ({t})', 'char *[]')} ? 1 : -1];"
                 for t in TABLES if t != "telopts"]
        for i, (e, want) in enumerate(OK_CASES):
            body.append(f"typedef char niji_tn_ok{i}[(!!{e}) == {want} ? 1 : -1];")
            body.append(f"typedef char niji_tn_okt{i}[{same(f'__typeof__ ({e})', okt)} ? 1 : -1];")
        body.append("char **niji_tn_lookup (int i)\n{\n  return i == 0 ? &AUTHTYPE_NAME (i) : i == 1 ? &ENCRYPT_NAME (i)"
                    " : i == 2 ? &ENCTYPE_NAME (i) : i == 3 ? &SLC_NAME (i) : &TELCMD (i);\n}")
        body.append("typedef char niji_tn_cmd[sizeof (TELCMD (TELCMD_FIRST)) == sizeof (char *) ? 1 : -1];")
        res.append((cid, "name macros arpa/telnet.h", "\n".join(body) + "\n"))
        body = [f"#define {s}" for s in dict.fromkeys(s for s, _ in TABLES.values())] + ["#include <arpa/telnet.h>"]
        body += [f"typedef char niji_tn_n_{t}[sizeof ({t}) == {n} * sizeof (char *) ? 1 : -1];"
                 for t, (_, n) in TABLES.items()]
        body += [f"typedef char niji_tn_to{i}[(!!({e})) == {want} ? 1 : -1];" for i, (e, want) in enumerate(TELOPT_CASES)]
        body.append("char **niji_tn_opt (int i) { return &TELOPT (i); }")
        if cxx and cid != "cxx98":
            body.append("constexpr bool niji_streq(const char *a, const char *b) "
                        "{ return *a == *b && (*a == 0 || niji_streq(a + 1, b + 1)); }")
            body.append("constexpr const char *niji_slc[] = { SLC_NAMELIST };")
            body.append(f"static_assert (sizeof (niji_slc) == {len(SLC) + 1} * sizeof (char *) && niji_slc[{len(SLC)}] == 0, \"SLC_NAMELIST\");")
            body += [f"static_assert (niji_streq (niji_slc[{i}], \"{s}\"), \"SLC_NAMELIST {i}\");" for i, s in enumerate(SLC)]
        res.append((cid, "name tables arpa/telnet.h", "\n".join(body) + "\n"))
    return res
