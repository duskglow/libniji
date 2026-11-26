"""Slice 14 checks: stage2-gaps-g (header_checks/s14.d/, loaded after s14.py by spec/tools/header_check.py;
header_checks/README.md)."""

# The headers that define CTRL themselves (headers/<h>.d/010-ctrl.frag).
CTRL_HEADERS = ("sys/ttydefaults.h", "termios.h", "termio.h", "pty.h", "sgtty.h", "sys/ioctl.h")


def units(ctx):
    """No spec clause beyond the BSD tty names (black-box probe of the reference, HEADERS-STAGE2-GAPS-G: sysvinit's
    sulogin and bsd-mailx use them): CTRL and CCEQ defined exactly where visibility.tsv shows them in
    <sys/ttydefaults.h> and <termios.h>; where shown, CTRL(c) is `(c) & 037`, an integer constant expression usable in
    `#if`, of type int for a char argument, CCEQ(val, c) is 1 for equal enabled characters and 0 for _POSIX_VDISABLE,
    and every header that defines CTRL can meet the others in one translation unit in either order. Defects: a missing
    macro (sulogin does not build), a different mask, a CCEQ that matches the disabled character, two spellings of
    CTRL that conflict when two of the headers are included."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    out = ctx.exact("sys/ttydefaults.h", ("CTRL",)) + ctx.exact("termios.h", ("CTRL", "CCEQ"))
    vis = {}
    for r in ctx.vis:
        if r["name"] in ("CTRL", "CCEQ") and r["header"] in CTRL_HEADERS:
            vis[(r["header"], r["name"])] = ctx.expand(r["configs"])
    for cid in ctx.ids:
        if not all(cid in vis.get((h, "CTRL"), ()) for h in CTRL_HEADERS):
            continue
        cxx = lang[cid] == "c++"
        t = "__is_same (__typeof__ (CTRL ((char) 'a')), int)" if cxx else \
            "__builtin_types_compatible_p (__typeof__ (CTRL ((char) 'a')), int)"
        # Two families: <termios.h>'s, in the order <termio.h> and <pty.h> themselves include it (after the kernel's
        # <asm/ioctls.h>), and <sys/ioctl.h>'s with <sys/ttydefaults.h>, in both orders. Across the families
        # TTYDEF_SPEED is spelled differently (B9600 and 13), which is not this family's (reported apart).
        for fam, heads in (("termios", ("termio.h", "pty.h", "termios.h")), ("ioctl", ("sys/ttydefaults.h", "sys/ioctl.h", "sgtty.h"))):
            body = [f"#include <{h}>" for h in heads]
            if fam == "ioctl":
                body += [f"#include <{h}>" for h in reversed(heads)]
            body += ["#if CTRL ('c') != 3 || CTRL ('C') != 3 || CTRL ('?') != 037\n#error CTRL\n#endif",
                     f"typedef char niji_ct[{t} ? 1 : -1];"]
            if fam == "termios" and cid in vis.get(("termios.h", "CCEQ"), ()):
                body += ["#include <unistd.h>",
                         "typedef char niji_cq[CCEQ (3, 3) == 1 && CCEQ (3, 4) == 0 && CCEQ (_POSIX_VDISABLE, _POSIX_VDISABLE) == 0 ? 1 : -1];"]
            out.append((cid, f"CTRL and CCEQ forms {fam}", "\n".join(body) + "\n"))
    return out
