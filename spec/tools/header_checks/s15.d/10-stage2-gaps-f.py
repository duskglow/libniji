"""Slice 15 checks: stage2-gaps-f (header_checks/s15.d/, loaded after s15.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# outb(2): name -> (return type, parameter types), the man page's prototypes (black-box probe of the reference: these
# types, in every configuration that accepts <sys/io.h>, C and C++).
PORT_IO = {}
for _w, _t in (("b", "unsigned char"), ("w", "unsigned short int"), ("l", "unsigned int")):
    for _p in ("", "_p"):
        PORT_IO[f"in{_w}{_p}"] = (_t, "unsigned short int")
        PORT_IO[f"out{_w}{_p}"] = ("void", f"{_t}, unsigned short int")
    PORT_IO[f"ins{_w}"] = ("void", "unsigned short int, void *, unsigned long int")
    PORT_IO[f"outs{_w}"] = ("void", "unsigned short int, const void *, unsigned long int")


def units(ctx):
    """No spec clause beyond outb(2) (HEADERS-STAGE2-GAPS-F: util-linux's sys-utils/hwclock-cmos.c calls outb and
    inb): where visibility.tsv shows each PORT_IO name in <sys/io.h>, it is a function, not a macro, with the man
    page's type (value first in out*, port first in the others; C++17 on, a noexcept function has another type), it
    has internal linkage (a static redeclaration compiles: the reference defines them static inline in the header and
    exports no symbol, so a header that only declares them leaves every caller unlinked), it is not noexcept in C++11
    on, and a call of each form compiles. Defects: a missing function, swapped out* parameters, a narrower port or
    count type, an extern declaration, a macro form."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    vis = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "sys/io.h" and r["kind"] == "function"}
    res = []
    for cid in ctx.ids:
        shown = [n for n in PORT_IO if n in vis and cid in expand(vis[n], ctx.ids)]
        if not shown:
            continue
        cxx = lang[cid] == "c++"
        body = ["#include <sys/io.h>"]
        for n in shown:
            ret, params = PORT_IO[n]
            same = (f"__is_same (__typeof__ ({n}), {ret} ({params}))" if cxx
                    else f"__builtin_types_compatible_p (__typeof__ ({n}), {ret} ({params}))")
            body += [f"#ifdef {n}\n#error \"{n} is a macro\"\n#endif",
                     f"typedef char niji_io_t_{n}[({same}) ? 1 : -1];",
                     f"static {ret} {n} ({params});"]
            if cxx and cid != "cxx98":
                body.append(f"typedef char niji_io_ne_{n}[noexcept ({n} ({', '.join('0' for _ in params.split(','))})) ? -1 : 1];")
        calls = []
        for n in shown:
            if n.startswith(("ins", "outs")):
                calls.append(f"  {n} (0x80, p, 1);")
            elif n.startswith("out"):
                calls.append(f"  {n} (1, 0x80);")
            else:
                calls.append(f"  s += {n} (0x80);")
        body.append("unsigned int niji_io_use (void *p)\n{\n  unsigned int s = 0;\n" + "\n".join(calls)
                    + "\n  return s;\n}")
        res.append((cid, "port I/O sys/io.h", "\n".join(body) + "\n"))
    return res
