"""Slice 13 checks: stage2-gaps-h (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# The FTP_NAMES tables of <arpa/ftp.h>: table -> entries (index 0 .. the last RFC 959 code, no null pointer). Black-box
# probe of the reference (program output, nm of the probe's objects): `char *[]`, defined with external linkage in
# the translation unit that defines FTP_NAMES (with a value or none), undeclared without it, in every configuration.
FTP_TABLES = {"typenames": 5, "strunames": 4, "modenames": 4, "formnames": 4}


def units(ctx):
    """No spec clause beyond RFC 959's names (HEADERS-STAGE2-GAPS-H: inetutils' ftpd/ftpd.c defines FTP_NAMES and
    uses all four tables): where visibility.tsv shows TYPE_A in <arpa/ftp.h>, (1) without FTP_NAMES none of the
    tables is declared (the unit declares each name itself as an int, an error beside the header's declaration);
    (2) with `#define FTP_NAMES` (empty) and with `#define FTP_NAMES 0`, each is defined with type `char *[N]`, N
    from FTP_TABLES. The strings are conformance/c/s13_arpa_ftp_names.c's job (a syntax check cannot read them).
    Defects: a table declared or defined without the switch (a conflict with a program's own name, a duplicate
    definition in every program), the switch tested with #if (FTP_NAMES 0 or empty), an entry missing or extra, a
    const element type (a program storing `char *` from the table)."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    row = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "arpa/ftp.h"}
    res = []
    for cid in ctx.ids:
        if cid not in expand(row.get("TYPE_A", ""), ctx.ids):
            continue
        cxx = lang[cid] == "c++"

        def same(a, b):
            return f"__is_same ({a}, {b})" if cxx else f"__builtin_types_compatible_p ({a}, {b})"

        body = ["#include <arpa/ftp.h>"] + [f"int {t};" for t in FTP_TABLES]
        res.append((cid, "FTP_NAMES absent arpa/ftp.h", "\n".join(body) + "\n"))
        for sw in ("", " 0"):
            body = [f"#define FTP_NAMES{sw}", "#include <arpa/ftp.h>"]
            body += [f"typedef char niji_ftp_{t}[{same(f'__typeof__ ({t})', f'char *[{n}]')} ? 1 : -1];"
                     for t, n in FTP_TABLES.items()]
            body.append("char **niji_ftp_type (int i) { return &typenames[i]; }")
            res.append((cid, f"FTP_NAMES{sw or ' empty'} arpa/ftp.h", "\n".join(body) + "\n"))
    return res
