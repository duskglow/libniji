"""Slice 15 (plan/headers.md section 4: administration, devices, legacy): P1 of `sys/mount.h` and `sys/timex.h`.

Both headers are `p1` 0 in headers/slices/s15.tsv: the table P1 of spec/tools/header_check.py cannot assert the names
they take from a header of a slice that is not generated yet (`fd_set`, `sigset_t` of <sys/select.h>, `f_owner_ex` and
`O_CLOEXEC` of <fcntl.h>), and `MS_NOUSER`, which the kernel's linux/mount.h defines as `(1<<31)`, an expression that
is not an integer constant expression for P1's array bound. This module runs the same P1 over the two headers without
those names (DEFERRED). Slices 7 and 14 have landed (SL-HEADERS-S07-TAIL), so `f_owner_ex` is checked. Still left out:
`MS_NOUSER`, and `OPEN_TREE_CLOEXEC`, which the kernel's linux/mount.h defines as `O_CLOEXEC`, a name <sys/mount.h>
does not make visible in 17 strict configurations; the reference's <sys/mount.h> does not compile a use there either
(black-box probe, HEADERS-SMALL-FIXES: all 17 rejected), so P1 has no value to assert in them.

`problems` (no spec clause; HEADERS-WINSIZE-TWICE): a template of this slice includes only a header libniji generates
(a template `headers/<name>.in`), a kernel UAPI header (`linux/`, `asm/`, `asm-generic/`) or one of the compiler's
own. Any other include, `__has_include` guarded or not, can only reach another C library's header, whose definitions
carry no `__NIJI_D_` guard: `struct winsize` of a system <sys/ioctl.h> was defined a second time by <sys/mount.h>.
Loaded by spec/tools/header_check.py, which lists this directory.
"""
import copy
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check  # noqa: E402

DEFERRED = {("sys/mount.h", "OPEN_TREE_CLOEXEC"), ("sys/mount.h", "MS_NOUSER")}
HEADERS = ("sys/mount.h", "sys/timex.h")


def units(ctx):
    sub = copy.copy(ctx)
    sub.slice_rows = [{"header": h, "p1": "1", "uapi": "-"} for h in HEADERS]
    sub.vis = [r for r in ctx.vis if (r["header"], r["name"]) not in DEFERRED]
    return [(cid, what, body) for cid, us in header_check.p1_units(sub).items() for what, body in us]


INCLUDE = re.compile(r"^\s*#\s*include\s*<([^>]+)>")
UAPI = ("linux/", "asm/", "asm-generic/")


def problems(ctx):
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    out = []
    for r in ctx.slice_rows:
        if not r["where"].startswith("headers/slices/s15.tsv:"):
            continue
        tmpl = os.path.join("headers", r["header"] + ".in")
        with open(os.path.join(ctx.root, tmpl)) as f:
            for n, line in enumerate(f, 1):
                m = INCLUDE.match(line)
                if not m:
                    continue
                inc = m.group(1)
                if inc.startswith(UAPI) or os.path.isfile(os.path.join(ctx.root, "headers", inc + ".in")) or os.path.isfile(os.path.join(gccinc, inc)):
                    continue
                out.append(f"{tmpl}:{n}: includes <{inc}>, which is neither libniji's (no headers/{inc}.in), the kernel's nor the compiler's")
    return out
