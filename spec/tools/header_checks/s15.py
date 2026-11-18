"""Slice 15 (plan/headers.md section 4: administration, devices, legacy): P1 of `sys/mount.h` and `sys/timex.h`.

Both headers are `p1` 0 in headers/slices/s15.tsv: the table P1 of spec/tools/header_check.py cannot assert the names
they take from a header of a slice that is not generated yet (`fd_set`, `sigset_t` of <sys/select.h>, `f_owner_ex` and
`O_CLOEXEC` of <fcntl.h>), and `MS_NOUSER`, which the kernel's linux/mount.h defines as `(1<<31)`, an expression that
is not an integer constant expression for P1's array bound. This module runs the same P1 over the two headers without
those names (DEFERRED); once slices 7 and 14 land the entries are removed and the rows become `p1` 1.
Loaded by spec/tools/header_check.py, which lists this directory.
"""
import copy
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check  # noqa: E402

DEFERRED = {("sys/timex.h", "fd_set"), ("sys/timex.h", "sigset_t"), ("sys/mount.h", "f_owner_ex"),
            ("sys/mount.h", "OPEN_TREE_CLOEXEC"), ("sys/mount.h", "MS_NOUSER")}
HEADERS = ("sys/mount.h", "sys/timex.h")


def units(ctx):
    sub = copy.copy(ctx)
    sub.slice_rows = [{"header": h, "p1": "1", "uapi": "-"} for h in HEADERS]
    sub.vis = [r for r in ctx.vis if (r["header"], r["name"]) not in DEFERRED]
    return [(cid, what, body) for cid, us in header_check.p1_units(sub).items() for what, body in us]
