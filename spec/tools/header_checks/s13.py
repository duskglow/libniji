"""Slice 13 (plan/headers.md section 4: network protocols and families): P1 of `net/if_ppp.h` and `net/if_shaper.h`.

Both headers are `p1` 0 in headers/slices/s13.tsv: the table P1 of spec/tools/header_check.py cannot assert the names
they take from <sys/ioctl.h> (`struct winsize`, `CBRK`, `TCSETS2`, ...), a header of slice 7 that has no template yet; their
templates name it in an `@@needs`, so it is included once it exists. This module runs the same P1 over the two headers
without the names `visibility.tsv` lists for <sys/ioctl.h> (DEFERRED). When sys/ioctl.h lands, delete this file and set
`p1` to 1 for both rows.
"""
import copy
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check  # noqa: E402

HEADERS = ("net/if_ppp.h", "net/if_shaper.h")


def units(ctx):
    sub = copy.copy(ctx)
    sub.slice_rows = [{"header": h, "p1": "1", "uapi": "-"} for h in HEADERS]
    deferred = {r["name"] for r in ctx.vis if r["header"] == "sys/ioctl.h"}
    sub.vis = [r for r in ctx.vis if not (r["header"] in HEADERS and r["name"] in deferred)]
    return [(cid, what, body) for cid, us in header_check.p1_units(sub).items() for what, body in us]
