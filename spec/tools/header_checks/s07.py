"""Slice 7's own header check (plan/headers.md section 4.1), loaded by spec/tools/header_check.py, which lists this
directory.

`problems` (no spec clause; ruling #6406, SL-HEADERS-S07-TAIL): no generated header guards an include of a header
libniji ships. A `__has_include(<H>)` test around `#include <H>` was the stand-in while H had no template
(`headers/H.in`); once H is generated the include is plain, like every other include of a libniji header, so a
guard left behind means the generated header silently depends on whether H is found on the include path, and on a
system tree it would reach another C library's H.

The P1 of the macros slice 7's headers show but another header owns (`MAXSYMLINKS` in sys/param.h, `TIOCGWINSZ` in
sys/ioctl.h) is the shared table P1's now (header_check.py `p1_units`, HEADERS-GEN-SHARED), for every slice.
"""
import os
import re

HAS_INCLUDE = re.compile(r"__has_include\s*\(\s*<([^>]+)>\s*\)")


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
