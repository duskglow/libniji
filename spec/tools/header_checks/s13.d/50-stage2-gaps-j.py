"""Slice 13 checks: stage2-gaps-j (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# inetutils' talkd (process.c) switches on the request type: DELETE must differ from the other request types.
TALKD_SWITCH = """#include <protocols/talkd.h>
int niji_talkd_switch (int t)
{
  switch (t)
    {
    case LEAVE_INVITE: return 1;
    case LOOK_UP: return 2;
    case DELETE: return 3;
    case ANNOUNCE: return 4;
    }
  return 0;
}
"""
# Each macro-values.tsv name, with only its header, has the table's value in a constant expression (P1 asserts it
# too where the slice row has p1 1; this one runs whatever the p1 cell). The form (the reference replaces a
# program's earlier definition) is macro-forms.tsv's default, checked by header_checks/form.py.
VALUE = """#include <{h}>
typedef char niji_gj_value[({n}) == {v} ? 1 : -1];
"""
WHOENT_CXX = """#include <protocols/rwhod.h>
typedef char niji_gj_whoent[sizeof (whod::whoent) == sizeof (((struct whod *) 0)->wd_we[0]) ? 1 : -1];
typedef char niji_gj_outmp[sizeof (::outmp) == 20 ? 1 : -1];
void niji_gj_whoent_use (whod::whoent *w) { w->we_idle = 0; }
"""
ANYADDR = """#include <protocols/timed.h>
#undef NULL
#define NULL 42
typedef char niji_gj_anyaddr[(ANYADDR) == 42 ? 1 : -1];
"""
TABLES = [
    ("protocols/timed.h", "TSPTYPES", "tsptype", "TSPTYPENUMBER", '"ANY"', '"LOOP"'),
    ("protocols/routed.h", "RIPCMDS", "ripcmds", "RIPCMD_MAX", '"#0"', '"TRACEOFF"'),
]
TABLE = """#define {sw}
#include <{h}>
typedef char niji_gj_count[sizeof {a} / sizeof {a}[0] == {n} ? 1 : -1];
char **niji_gj_table = {a};
int niji_gj_ends (void) {{ return __builtin_strcmp ({a}[0], {first}) == 0 && __builtin_strcmp ({a}[{n} - 1], {last}) == 0; }}
"""


def units(ctx):
    """No spec clause beyond the reference's headers (HEADERS-STAGE2-GAPS-J; RELEASE-SNAPSHOT-1127A: inetutils' talkd
    process.c "duplicate case value", <protocols/rwhod.h> in C++): (1) each spec/abi/macro-values.tsv name, where
    visibility.tsv shows it, has its own value with its header alone;
    talkd's request types switch; (2) C++ names `whod::whoent` and `::outmp`; (3) ANYADDR expands to whatever NULL
    is where it is used; (4) TSPTYPES and RIPCMDS define the name tables, TSPTYPENUMBER and RIPCMD_MAX entries;
    (5) llvm-19's sanitizer set: <sys/ioctl.h>, <net/if_ppp.h>, <netax25/ax25.h>, <netrom/netrom.h> together, the
    three structures complete at the reference's sizes, the ioctl requests at its values.
    Defects: one value per name across headers (talkd's DELETE 0), whoent at namespace scope in C++, ANYADDR spelled
    as its own null pointer constant, a table missing or of another length, a structure defined twice in the set
    (<net/if_ppp.h> repeating the kernel's struct compstat), incomplete or of another size, a request's wrong value."""
    shown = {}
    for r in ctx.vis:
        shown.setdefault((r["header"], r["name"]), set()).update(expand(r["configs"], ctx.ids))
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    pick = ("gnu17", "default", "cxx98", "cxx17")
    out = []
    for (h, n), row in sorted(ctx.header_values.items()):
        body = VALUE.format(n=n, h=h, v=row["value"])
        out += [(cid, f"GAPS-J value {n} {h}", body) for cid in pick if cid in shown.get((h, n), ())]
    out += [(cid, "GAPS-J talkd switch", TALKD_SWITCH) for cid in pick if cid in shown.get(("protocols/talkd.h", "DELETE"), ())]
    out += [(cid, "GAPS-J whod::whoent", WHOENT_CXX) for cid in pick if lang[cid] == "c++" and cid in shown.get(("protocols/rwhod.h", "whod"), ())]
    out += [(cid, "GAPS-J ANYADDR is NULL", ANYADDR) for cid in pick if cid in shown.get(("protocols/timed.h", "tsp"), ())]
    for h, sw, a, n, first, last in TABLES:
        body = TABLE.format(sw=sw, h=h, a=a, n=n, first=first, last=last)
        out += [(cid, f"GAPS-J table {a}", body) for cid in pick if cid in shown.get((h, n), ())]
    need = [("net/if_ppp.h", "ppp_stats"), ("netax25/ax25.h", "ax25_parms_struct"), ("netrom/netrom.h", "nr_parms_struct")]
    out += [(cid, "GAPS-J sanitizer ax25/netrom/ppp", SANITIZER) for cid in pick if all(cid in shown.get(k, ()) for k in need)]
    return out


# llvm-19 compiler-rt's sanitizer_platform_limits_posix.cpp includes these together and takes the sizes of the three
# structures and the values of the ioctl requests. Sizes and values: black-box probes of the reference
# (HEADERS-STAGE2-GAPS-J; SIOCPROTOPRIVATE from <sys/ioctl.h>).
SANITIZER_SIZES = [("struct ax25_parms_struct", 48), ("struct nr_parms_struct", 36), ("struct ppp_stats", 76)]
SANITIZER_VALUES = [
    ("PPPIOCGASYNCMAP", 2147775576), ("PPPIOCGDEBUG", 2147775553), ("PPPIOCGFLAGS", 2147775578),
    ("PPPIOCGUNIT", 2147775574), ("PPPIOCGXASYNCMAP", 2149610576), ("PPPIOCSASYNCMAP", 1074033751),
    ("PPPIOCSDEBUG", 1074033728), ("PPPIOCSFLAGS", 1074033753), ("PPPIOCSMAXCID", 1074033745),
    ("PPPIOCSMRU", 1074033746), ("PPPIOCSXASYNCMAP", 1075868751), ("SIOCAX25ADDUID", 35297),
    ("SIOCAX25DELUID", 35298), ("SIOCAX25GETPARMS", 35301), ("SIOCAX25GETUID", 35296), ("SIOCAX25NOUID", 35299),
    ("SIOCAX25SETPARMS", 35302), ("SIOCNRDECOBS", 35298), ("SIOCNRGETPARMS", 35296), ("SIOCNRRTCTL", 35299),
    ("SIOCNRSETPARMS", 35297),
]
SANITIZER = "#include <sys/ioctl.h>\n#include <net/if_ppp.h>\n#include <netax25/ax25.h>\n#include <netrom/netrom.h>\n" + "".join(
    f"typedef char niji_gj_size{i}[sizeof ({t}) == {s} ? 1 : -1];\n" for i, (t, s) in enumerate(SANITIZER_SIZES)
) + "".join(f"typedef char niji_gj_ioctl_{n}[(unsigned long) ({n}) == {v}UL ? 1 : -1];\n" for n, v in SANITIZER_VALUES)
