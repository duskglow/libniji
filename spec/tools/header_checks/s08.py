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
    dyn = dynamic_levels(ctx)
    for h, ids in sorted(seen.items()):
        body = (f"#include <{h}>\n"
                "long niji_stack_sizes (void);\n"
                "long niji_stack_sizes (void) { return (long) MINSIGSTKSZ + (long) SIGSTKSZ; }\n")
        out.extend((cid, f"MINSIGSTKSZ/SIGSTKSZ used in {h}", body) for cid in sorted(ids))
        out.extend((cid, f"MINSIGSTKSZ/SIGSTKSZ value in {h}", value_unit(ctx, h, cid, dyn[cid]))
                   for cid in sorted(ids))
    return out


def dynamic_levels(ctx):
    """Configuration id -> whether features.h sets `__NIJI_DYNSTACK` there (headers/levels.tsv)."""
    return {r["id"]: r["dynstack"] == "1" for r in ctx.rows("headers/levels.tsv")}


def value_unit(ctx, h, cid, dynamic):
    """`value` (no spec clause beyond Appendix C.9 and s20_7_i_0's erratum, D-20.10; HEADERS-SIGSTKSZ-MATCH): where
    the reference's MINSIGSTKSZ and SIGSTKSZ are constants (no `_GNU_SOURCE` or `_DYNAMIC_STACK_SIZE_SOURCE`) they are
    integer constant expressions with macros.tsv's values (a file-scope array size compiles, as old code relies on);
    where they are run-time values neither is a constant (`__builtin_constant_p` is 0, so a `constexpr` or static
    array use fails as with the reference), and both call sysconf with `_SC_SIGSTKSZ`: the reference's MINSIGSTKSZ is
    SIGSTKSZ's value there, not the kernel's minimum (probed: both 14528 on the Skylake-SP host, 8192 on the Zen 1 host). Defining `__sysconf` as
    a function-like macro after the include turns the expansion into its argument, so the name passed is checked at
    compile time."""
    m = ctx.macros
    if not dynamic:
        return (f"#include <{h}>\n"
                f"typedef char niji_stack_constant[(MINSIGSTKSZ == {m['MINSIGSTKSZ']['value']} && "
                f"SIGSTKSZ == {m['SIGSTKSZ']['value']}) ? 1 : -1];\n")
    sc = m["_SC_SIGSTKSZ"]["value"]
    # The expansion's type is sysconf's, `long` (a call to an undeclared function would be `int` in C89 and fail in
    # C++): C compares the types, C++ selects a specialisation (no _Generic there).
    if {c["id"]: c["language"] for c in ctx.cfgs}[cid] == "c++":
        is_long = ("template <class T> struct niji_is_long { enum { v = -1 }; };\n"
                   "template <> struct niji_is_long<long> { enum { v = 1 }; };\n"
                   "typedef char niji_stack_type[niji_is_long<__typeof__ (MINSIGSTKSZ)>::v + "
                   "niji_is_long<__typeof__ (SIGSTKSZ)>::v - 1];\n")
    else:
        is_long = ("typedef char niji_stack_type[(__builtin_types_compatible_p (__typeof__ (MINSIGSTKSZ), long) && "
                   "__builtin_types_compatible_p (__typeof__ (SIGSTKSZ), long)) ? 1 : -1];\n")
    return (f"#include <{h}>\n"
            "typedef char niji_stack_runtime[(!__builtin_constant_p (MINSIGSTKSZ) && "
            "!__builtin_constant_p (SIGSTKSZ)) ? 1 : -1];\n" + is_long +
            "#define __sysconf(niji_name) (niji_name)\n"
            f"typedef char niji_stack_name[(MINSIGSTKSZ == {sc} && SIGSTKSZ == {sc}) ? 1 : -1];\n")
