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
    return [(c["id"], "P1 stdint", identity_unit(c["language"] == "c++", types)) for c in ctx.cfgs if c["id"] not in NO_STATIC_ASSERT]
