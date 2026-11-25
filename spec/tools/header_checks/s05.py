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
    # rows. The reserved `__u_char` family (HEADERS-STAGE2-GAPS-E: libnsl and rpc/types.h name them) is shown in every
    # configuration that accepts the header, strict ISO included.
    out += ctx.exact("sys/types.h", SYS_TYPES_NAMES + U_CHAR_FAMILY)
    # Spec clause: C 7.2 p3 (C11, C17: <assert.h> defines `static_assert` as `_Static_assert`; C23 makes it a keyword,
    # and in C++ it is one) (HEADERS-STAGE2-GAPS-C, elogind): defined exactly where the reference defines it, and usable
    # at file scope there.
    out += ctx.exact("assert.h", ("static_assert",))
    out += [(cid, "static_assert use", "#include <assert.h>\nstatic_assert (sizeof (int) >= 2, \"int\");\nint niji_s05_sa;\n")
            for cid in ctx.ids if cid in shown_in(ctx, "assert.h", "static_assert")]
    return out + guard_units(ctx)


def shown_in(ctx, header, name):
    """The configuration ids where visibility.tsv shows `name` in `header` (none without a row)."""
    return set().union(*(ctx.expand(r["configs"]) for r in ctx.vis if r["header"] == header and r["name"] == name))


def guard_units(ctx):
    """Spec clause: no spec clause (HEADERS-STAGE2-GAPS-C, Yohane stage 2: gmp's <gmp.h> declares its `FILE *`
    functions only when it sees a stdio include guard). Every shipped header defines exactly the include-guard names
    (header_probe.guard_candidates) visibility.tsv shows for it, with the reference's value (`1`, or empty where the
    reference's is empty: the spelling of the expansion, through `#`, is compared), and leaves the other candidates
    undefined. One unit per header and distinct pattern of shown guards (C and C++ always: complex.h and tgmath.h show
    theirs only outside C++11 and later); configurations levels.tsv rejects and headers shipped to fail are left out."""
    rejected = {r["id"] for r in ctx.rows("headers/levels.tsv") if r["reject"] == "1"}
    rows = {}
    for r in ctx.vis:
        rows.setdefault(r["header"], []).append(r)
    out = []
    for h in sorted({r["header"] for r in ctx.slice_rows} - ctx.must_fail):
        hv = rows.get(h, [])
        failing = set().union(*(ctx.expand(r["configs"]) for r in hv if r["kind"] == "include-error"))
        cands = ctx.hp.guard_candidates(h)
        shown = {n: set().union(*(ctx.expand(r["configs"]) for r in hv if r["name"] == n and r["kind"] == "macro")) for n in cands}
        seen = set()
        for cid in ctx.ids:
            if cid in rejected or cid in failing:
                continue
            lang = next(c["language"] for c in ctx.cfgs if c["id"] == cid)
            sig = (lang, frozenset(n for n in cands if cid in shown[n]))
            if sig in seen:
                continue
            seen.add(sig)
            lines = [f"#include <{h}>", "#define NIJI_G_STR2(x) #x", "#define NIJI_G_STR(x) NIJI_G_STR2(x)"]
            for i, n in enumerate(cands):
                if cid in shown[n]:
                    m = ctx.macros.get(n)
                    v = m["value"] if m else ""
                    lines.append(f"#ifndef {n}\n#error \"{n} not defined\"\n#endif")
                    lines.append(f"typedef char niji_guard_{i}[sizeof (NIJI_G_STR ({n})) == {len(v) + 1} ? 1 : -1];")
                else:
                    lines.append(f"#ifdef {n}\n#error \"{n} defined\"\n#endif")
            out.append((cid, f"guards {h}", "\n".join(lines) + "\nint niji_guard_tu;\n"))
    return out


U_CHAR_FAMILY = ("__u_char", "__u_short", "__u_int", "__u_long", "__quad_t", "__u_quad_t")


SYS_TYPES_NAMES = ("u_char", "u_short", "u_int", "u_long", "ushort", "uint", "ulong", "quad_t", "u_quad_t", "fsid_t",
                   "loff_t", "daddr_t", "caddr_t", "register_t", "u_int8_t", "u_int16_t", "u_int32_t", "u_int64_t",
                   "blksize_t", "blkcnt64_t", "fsblkcnt_t", "fsblkcnt64_t", "fsfilcnt_t", "fsfilcnt64_t", "ino64_t",
                   "suseconds_t", "useconds_t")
