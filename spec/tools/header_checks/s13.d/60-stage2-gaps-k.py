"""Slice 13 checks: stage2-gaps-k (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# TTYDEF_SPEED is B9600 in every header (spec/abi/macro-spellings.tsv): with a header that does not show the termios
# constants it is no integer constant, as the reference's (black-box probe, HEADERS-STAGE2-GAPS-K). P1 cannot
# evaluate it there; the units below check it where B9600 is shown.
p1_skip = {(h, "TTYDEF_SPEED") for h in ("net/if_ppp.h", "net/if_shaper.h", "sgtty.h", "sys/ioctl.h", "sys/mount.h",
                                         "sys/mtio.h", "sys/raw.h", "sys/ttydefaults.h")}

# Header pairs a program includes together, either order: no macro of one is redefined by the other (the unit runs
# with -Wsystem-headers -Werror). <net/if_ppp.h> with the kernel's <linux/ppp-ioctl.h> is no pair: both define
# struct npioctl, with the reference's headers too; its PPP names are checked by their kernel spelling below.
PAIRS = [("termios.h", "sys/ioctl.h"), ("termio.h", "sys/ioctl.h"), ("pty.h", "sys/ioctl.h"), ("sgtty.h", "termios.h"),
         ("sys/ttydefaults.h", "termios.h"), ("sys/ttydefaults.h", "sys/ioctl.h"), ("termios.h", "asm/ioctls.h"),
         ("netipx/ipx.h", "sys/ioctl.h")]

# After the header, a program's definition of the name in the kernel header's text: no redefinition diagnostic, so
# the header's own text is the same token sequence.
RESPELL = "#define {n} {s}\n"

IPX = """#include <sys/ioctl.h>
#include <netipx/ipx.h>
#include <stddef.h>
typedef char niji_gk_crt[(SIOCAIPXITFCRT) == SIOCPROTOPRIVATE + 0 ? 1 : -1];
typedef char niji_gk_slt[(SIOCAIPXPRISLT) == SIOCPROTOPRIVATE + 1 ? 1 : -1];
typedef char niji_gk_cfg[(SIOCIPXCFGDATA) == SIOCPROTOPRIVATE + 2 ? 1 : -1];
typedef char niji_gk_ncp[(SIOCIPXNCPCONN) == SIOCPROTOPRIVATE + 3 ? 1 : -1];
typedef char niji_gk_par[2 * SIOCAIPXPRISLT == 2 * (SIOCPROTOPRIVATE + 1) && 2 * SIOCIPXNCPCONN == 2 * (SIOCPROTOPRIVATE + 3) ? 1 : -1];
typedef char niji_gk_special[offsetof (struct sockaddr_ipx, sipx_special) == 2 && sizeof (((struct sockaddr_ipx *) 0)->sipx_special) == 2 ? 1 : -1];
"""
SPEED = """#include <{h}>
typedef char niji_gk_speed[(TTYDEF_SPEED) == (B9600) ? 1 : -1];
"""


def units(ctx):
    """No spec clause beyond the reference's headers (HEADERS-STAGE2-GAPS-K; llvm-19's
    sanitizer_platform_limits_posix.cpp, Yohane stage 2): (1) the PAIRS together, either order, without a
    redefinition; (2) each spec/abi/macro-spellings.tsv name with a kernel source, in each header that shows it,
    respelled after the header in the kernel's text without a redefinition; (3) <netipx/ipx.h>'s requests count from
    SIOCPROTOPRIVATE, parenthesised, and sipx_special is the port member; (4) TTYDEF_SPEED is B9600 where both show.
    Defects: TTYDEF_SPEED or TIOCSER_TEMT spelled two ways (termios.h against sys/ioctl.h or the kernel's
    asm/ioctls.h), a PPP, BLK*, MTIO* name spelled otherwise than the kernel's header (a redefinition with it), an
    IPX request missing, unparenthesised or off by one, sipx_special aliasing another member."""
    shown = {}
    for r in ctx.vis:
        shown.setdefault((r["header"], r["name"]), set()).update(expand(r["configs"], ctx.ids))
    headers = {h for h, _ in shown}
    pick = ("gnu17", "default", "cxx98", "cxx17")
    out = []
    for a, b in PAIRS:
        lib = a if a in headers else b
        for x, y in ((a, b), (b, a)):
            body = f"#include <{x}>\n#include <{y}>\nint niji_gk_pair;\n"
            out += [(cid, f"GAPS-K pair {x} {y}", body) for cid in pick if any(cid in s for (h, _), s in shown.items() if h == lib)]
    spell = [r for r in ctx.rows("spec/abi/macro-spellings.tsv") if r["source"] != "-"]
    by_header = {}
    for r in spell:
        for (h, n), cfgs in shown.items():
            if n == r["name"]:
                by_header.setdefault(h, []).append((r, cfgs))
    for h, rows in sorted(by_header.items()):
        for cid in pick:
            body = "".join(RESPELL.format(n=r["name"], s=r["spelling"]) for r, cfgs in rows if cid in cfgs)
            if body:
                out.append((cid, f"GAPS-K kernel spelling {h}", f"#include <{h}>\n{body}"))
    out += [(cid, "GAPS-K netipx requests", IPX) for cid in pick if cid in shown.get(("netipx/ipx.h", "SIOCAIPXITFCRT"), ())]
    for h in ("termios.h", "termio.h", "pty.h", "sys/termios.h"):
        out += [(cid, f"GAPS-K TTYDEF_SPEED {h}", SPEED.format(h=h)) for cid in pick
                if cid in shown.get((h, "TTYDEF_SPEED"), ()) and cid in shown.get((h, "B9600"), ())]
    return out
