"""Slice 7's own header check (plan/headers.md section 4.1), loaded by spec/tools/header_check.py, which lists this
directory.

`problems` (no spec clause; ruling #6406, SL-HEADERS-S07-TAIL): no generated header guards an include of a header
libniji ships. A `__has_include(<H>)` test around `#include <H>` was the stand-in while H had no template
(`headers/H.in`); once H is generated the include is plain, like every other include of a libniji header, so a
guard left behind means the generated header silently depends on whether H is found on the include path, and on a
system tree it would reach another C library's H.

`units` (plan/headers.md section 3, P1): the table P1 asserts a macros.tsv value only in the header that owns its row
(the row's `header` column). Slice 7's last five headers define many macros owned by a header whose P1 does not run
(`MAXSYMLINKS` of arpa/nameser.h, `p1` 0; `TIOCGWINSZ` of net/if_ppp.h, which no slice ships), so their values were
asserted nowhere. These units run the same P1 over each of the five headers for the integer macros visible there that
another header owns.
"""
import copy
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check  # noqa: E402

HAS_INCLUDE = re.compile(r"__has_include\s*\(\s*<([^>]+)>\s*\)")
TAIL = ("sys/wait.h", "wait.h", "sys/param.h", "spawn.h", "sys/ioctl.h")


def units(ctx):
    """P1 of the macros each TAIL header shows but another header owns (the module docstring says why)."""
    out = []
    for h in TAIL:
        sub = copy.copy(ctx)
        sub.slice_rows = [{"header": h, "p1": "1", "uapi": "-"}]
        sub.vis = [r for r in ctx.vis if r["header"] == h and r["kind"] == "macro"
                   and r["name"] in ctx.macros and ctx.macros[r["name"]]["header"] != h]
        sub.macros = {r["name"]: dict(ctx.macros[r["name"]], header=h) for r in sub.vis}
        for cid, us in header_check.p1_units(sub).items():
            out.extend((cid, what + " (macros another header owns)", body) for what, body in us)
    return out


def problems(ctx):
    out = []
    base = os.path.join(ctx.root, "libc-include")
    for d, _, files in sorted(os.walk(base)):
        for f in sorted(files):
            if not f.endswith(".h"):
                continue
            path = os.path.join(d, f)
            with open(path) as fh:
                for n, line in enumerate(fh, 1):
                    for inc in HAS_INCLUDE.findall(line):
                        if os.path.isfile(os.path.join(ctx.root, "headers", inc + ".in")):
                            rel = os.path.relpath(path, ctx.root)
                            out.append(f"{rel}:{n}: guards the include of <{inc}>, which libniji ships (headers/{inc}.in): include it plainly")
    return out
