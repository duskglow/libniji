"""Slice 12 checks: ipproto-max (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py; header_checks/README.md)."""
from header_check import expand


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


def units(ctx):
    return ipproto_max_units(ctx)
