"""Slice 15 (plan/headers.md section 4: administration, devices, legacy): P1 of `sys/mount.h` and `sys/timex.h`.

Both headers are `p1` 0 in headers/slices/s15.tsv: the table P1 of spec/tools/header_check.py cannot assert the names
they take from a header of a slice that is not generated yet (`fd_set`, `sigset_t` of <sys/select.h>, `f_owner_ex` and
`O_CLOEXEC` of <fcntl.h>). This module runs the same P1 over the two headers without those names (DEFERRED). Slices
7 and 14 have landed (SL-HEADERS-S07-TAIL), so `f_owner_ex` is checked. `MS_NOUSER` is an int enumerator now (D-X.173;
the kernel's `(1<<31)` macro is replaced), so P1 checks it too. Still left out: `OPEN_TREE_CLOEXEC`, which the
kernel's linux/mount.h defines as `O_CLOEXEC`, a name <sys/mount.h> does not make visible in 17 strict configurations;
the reference's <sys/mount.h> does not compile a use there either (black-box probe, HEADERS-SMALL-FIXES: all 17
rejected), so P1 has no value to assert in them.

`enum_units` (D-X.173): each name of <sys/mount.h> that enumerators.tsv lists and enum-macros.tsv maps to itself
(`MS_RDONLY`, `MNT_FORCE`, ...) is, after <sys/mount.h> and <linux/mount.h> in either order, a macro that expands to
its own name, of the table's value, and in C++ of an enumeration type.

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

DEFERRED = {("sys/mount.h", "OPEN_TREE_CLOEXEC")}
HEADERS = ("sys/mount.h", "sys/timex.h")


def units(ctx):
    sub = copy.copy(ctx)
    sub.slice_rows = [{"header": h, "p1": "1", "uapi": "-"} for h in HEADERS]
    sub.vis = [r for r in ctx.vis if (r["header"], r["name"]) not in DEFERRED]
    p1 = [(cid, what, body) for cid, us in header_check.p1_units(sub).items() for what, body in us]
    return p1 + mtio_units(ctx) + sg_timeout_units(ctx) + enum_units(ctx)


def sg_timeout_units(ctx):
    """No spec clause beyond sg(4): SG_DEFAULT_TIMEOUT is `(60*HZ)` with the program's HZ (black-box probe,
    HEADERS-NET-SYS-NAMES: the header leaves HZ undefined; with `HZ` defined as `1 + 1` the value is 61, so HZ is not
    parenthesised, and `7 % SG_DEFAULT_TIMEOUT` is 7, so the whole is)."""
    cids = {cid for r in ctx.vis if r["header"] == "scsi/sg.h" and r["name"] == "SG_DEFAULT_TIMEOUT"
            for cid in header_check.expand(r["configs"], ctx.ids)}
    if not cids:
        return [(ctx.ids[0], "SG_DEFAULT_TIMEOUT scsi/sg.h", "#error visibility.tsv does not show SG_DEFAULT_TIMEOUT\n")]
    body = ("#include <scsi/sg.h>\n#ifdef HZ\n#error scsi/sg.h defines HZ\n#endif\n#define HZ 1 + 1\n"
            "typedef char niji_sg_t[(SG_DEFAULT_TIMEOUT == 61 && 7 % SG_DEFAULT_TIMEOUT == 7) ? 1 : -1];\n")
    return [(cid, "SG_DEFAULT_TIMEOUT scsi/sg.h", body) for cid in sorted(cids)]


def enum_units(ctx):
    """No spec clause (mount(2) names the flags, not their form; D-X.173, form of the reference's headers by black-box
    probe): per configuration that shows them, with <linux/mount.h> before and after <sys/mount.h>, each self-naming
    enumerator of <sys/mount.h> is a macro whose spelling is its own name, its value is the macros.tsv value, and in
    C++ its type is an enumeration. A plausible defect each catches: the kernel's literal macro left in place (the
    spelling), an enumerator dropped or given the kernel's `(1<<31)` (the value), a macro of the value only (the C++
    type), a redefinition or a second enumerator in one of the orders (the unit fails to compile under -Werror)."""
    selfnamed = {r["name"] for r in ctx.rows("spec/abi/enum-macros.tsv") if r["name"] == r["enumerator"]}
    rows = [r for r in ctx.vis if r["header"] == "sys/mount.h" and r["kind"] == "enumerator" and r["name"] in selfnamed]
    if len(rows) < 31:
        return [(ctx.ids[0], "enum sys/mount.h", "#error visibility.tsv shows fewer than 31 self-naming enumerators\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    res = []
    for cid in ctx.ids:
        names = sorted(r["name"] for r in rows if cid in header_check.expand(r["configs"], ctx.ids))
        if not names:
            continue
        checks = ["#define NIJI_STR_(x) #x", "#define NIJI_STR(x) NIJI_STR_(x)"]
        for i, n in enumerate(names):
            checks.append(f"typedef char niji_en{i}[({n} == ({int(ctx.macros[n]['value'])}L) && sizeof NIJI_STR({n}) == sizeof \"{n}\") ? 1 : -1];")
            if lang[cid] == "c++":
                checks.append(f"typedef char niji_ek{i}[__is_enum(__typeof__({n})) ? 1 : -1];")
        for first, second in (("linux/mount.h", "sys/mount.h"), ("sys/mount.h", "linux/mount.h")):
            res.append((cid, f"enum {first} then {second}", f"#include <{first}>\n#include <{second}>\n" + "\n".join(checks) + "\n"))
    return res


# st(4) <sys/mtio.h>: the generic status tests, each the bit of mt_gstat it tests (black-box probe, HEADERS-BUSYBOX-GAPS:
# each evaluated on every single bit of an unsigned long and on -1; the result has the type of the mask constant,
# unsigned int for GMT_EOF, int for the others).
GMT = {"GMT_EOF": 31, "GMT_BOT": 30, "GMT_EOT": 29, "GMT_SM": 28, "GMT_EOD": 27, "GMT_WR_PROT": 26, "GMT_ONLINE": 24,
       "GMT_D_6250": 23, "GMT_D_1600": 22, "GMT_D_800": 21, "GMT_DR_OPEN": 18, "GMT_IM_REP_EN": 16}
MT_TAPE_INFO_ROWS = 17  # sixteen drive types and the zero row that ends the table (black-box probe)


def mtio_units(ctx):
    """No spec clause beyond st(4): per configuration that shows them, each GMT_* macro keeps exactly its bit of a
    long argument, of an int -1 the same bit with the mask's type, and MT_TAPE_INFO initializes a struct
    mt_tape_info table of 17 rows."""
    rows = {r["name"]: r for r in ctx.vis if r["header"] == "sys/mtio.h" and (r["name"] in GMT or r["name"] == "MT_TAPE_INFO")}
    if set(rows) != set(GMT) | {"MT_TAPE_INFO"}:
        return [(ctx.ids[0], "GMT sys/mtio.h", "#error visibility.tsv does not show every GMT_* name and MT_TAPE_INFO\n")]
    res = []
    for cid in ctx.ids:
        if not all(cid in header_check.expand(r["configs"], ctx.ids) for r in rows.values()):
            continue
        body = ["#include <sys/mtio.h>"]
        for i, (n, b) in enumerate(GMT.items()):
            signed = "<" if b < 31 else ">"
            body += [f"typedef char niji_gmt{i}[({n}(~0UL) == (1UL << {b}) && {n}(~(1UL << {b})) == 0 && {n}(1L << {b}) != 0) ? 1 : -1];",
                     f"typedef char niji_gmt{i}_t[(sizeof({n}(-1)) == sizeof(int) && ({n}(0) - 1) {signed} 0 && {n}(-1) == (1U << {b})) ? 1 : -1];"]
        if not cid.startswith("cxx"):  # t_name is `char *`: C++ does not take a string literal there
            body += ["struct mt_tape_info niji_ti[] = MT_TAPE_INFO;",
                     f"typedef char niji_ti_n[sizeof niji_ti / sizeof niji_ti[0] == {MT_TAPE_INFO_ROWS} ? 1 : -1];"]
        res.append((cid, "GMT sys/mtio.h", "\n".join(body) + "\n"))
    return res
