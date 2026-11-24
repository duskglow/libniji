"""Slice 8's own header check (plan/headers.md section 4), loaded by spec/tools/header_check.py, which lists this
directory.

`units` (no spec clause beyond Appendix C.9; HEADERS-SMALL-FIXES): MINSIGSTKSZ and SIGSTKSZ are used in an expression,
in every configuration where a header shows them (visibility.tsv). Under `_DYNAMIC_STACK_SIZE_SOURCE` and `_GNU_SOURCE`
they expand to a call (C.9: sysconf's value), which the table P1 leaves out (not a constant), so nothing compiled a
use: the expansion's function was undeclared there, which C++ rejects and C99 and later warn about (`-Werror`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check  # noqa: E402

HEADERS = ("signal.h", "sys/signal.h", "sys/wait.h")


def units(ctx):
    seen = {}
    for r in ctx.vis:
        if r["header"] in HEADERS and r["name"] in ("MINSIGSTKSZ", "SIGSTKSZ") and r["kind"] == "macro":
            ids = header_check.expand(r["configs"], ctx.ids)
            seen[r["header"]] = ids if r["header"] not in seen else seen[r["header"]] & ids
    out = []
    for h, ids in sorted(seen.items()):
        body = (f"#include <{h}>\n"
                "long niji_stack_sizes (void);\n"
                "long niji_stack_sizes (void) { return (long) MINSIGSTKSZ + (long) SIGSTKSZ; }\n")
        out.extend((cid, f"MINSIGSTKSZ/SIGSTKSZ used in {h}", body) for cid in sorted(ids))
    return out
