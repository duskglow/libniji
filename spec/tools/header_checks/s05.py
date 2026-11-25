"""Slice 5 (plan/headers.md section 4: generator and base layer): P1 identity of the C 7.20 integer types (ruling #6047).

Every C 7.20.1 type spelled by header_probe.c_integer_names(): C asserts its size and alignment, C++ asserts `__is_same`
with the typedefs.tsv `cxx` spelling (the spelling enters C++ linkage names, so `long` against `long long` is ABI).
Loaded by spec/tools/header_check.py, which lists this directory (plan/headers.md section 4, "Adding a slice").
"""

# static_assert is C++11 and _Static_assert C11 (C89 mode has neither as a keyword).
NO_STATIC_ASSERT = ("cxx98", "c89", "isoc99")


def wanted(ctx):
    return [n for n in ctx.hp.c_integer_names() if n.islower()] + ["intmax_t", "uintmax_t", "intptr_t", "uintptr_t"]


def identity_unit(cxx, types):
    """P1 lines for the C 7.20.1 types (typedefs.tsv rows), C or C++."""
    lines = ["#include <stdint.h>"]
    for name, r in types:
        if cxx:
            lines.append(f"static_assert(__is_same({name}, {r['cxx']}), \"{name} is {r['cxx']}\");")
        else:
            lines.append(f"_Static_assert(sizeof({name}) == {r['size']} && _Alignof({name}) == {r['align']}, \"{name}\");")
    return "\n".join(lines) + "\n"


def problems(ctx):
    return [f"P1 {n}: no typedefs.tsv row" for n in wanted(ctx) if n not in ctx.typedefs]


def units(ctx):
    types = [(n, ctx.typedefs[n]) for n in wanted(ctx) if n in ctx.typedefs]
    out = [(c["id"], "P1 stdint", identity_unit(c["language"] == "c++", types)) for c in ctx.cfgs if c["id"] not in NO_STATIC_ASSERT]
    # Spec clause: XBD <sys/types.h> and the BSD/System V names beside them (HEADERS-SNAPSHOT-GAPS): each shown exactly
    # where the reference shows it (tree's build used u_long in the default mode), with its size, alignment and C++
    # spelling, and free for the program elsewhere. sys/types.h has no table P1 (`p1` 0 in s05.tsv), so these are its
    # rows.
    return out + ctx.exact("sys/types.h", SYS_TYPES_NAMES)


SYS_TYPES_NAMES = ("u_char", "u_short", "u_int", "u_long", "ushort", "uint", "ulong", "quad_t", "u_quad_t", "fsid_t",
                   "loff_t", "daddr_t", "caddr_t", "register_t", "u_int8_t", "u_int16_t", "u_int32_t", "u_int64_t",
                   "blksize_t", "blkcnt64_t", "fsblkcnt_t", "fsblkcnt64_t", "fsfilcnt_t", "fsfilcnt64_t", "ino64_t",
                   "suseconds_t", "useconds_t")
