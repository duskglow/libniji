"""Slice 11 (plan/headers.md section 4: math): P1 of the math group of headers, and the P6 units of tgmath.h and ieee754.h.

The slice table lists these headers with `p1` 0: this module does the table P1 itself, because the shared P1 knows
neither floating macros nor unions, and a header the C++ standard library wraps (math.h, complex.h) must be asserted
by its C configurations only. Per (header, configuration) one unit includes the header and asserts, for what the
header makes visible there, the integer and floating macro values of macros.tsv, the typedef sizes and alignments and
the structure and union sizes and alignments of layouts.tsv. A false condition is a negative array size (C89 and C++98
have no static assertion); a floating comparison is not an integer constant expression and uses static assertions
where the language has them.
Loaded by spec/tools/header_check.py, which lists this directory (plan/headers.md section 4, "Adding a slice").
"""
import decimal
import os

HEADERS = ["math.h", "complex.h", "tgmath.h", "fenv.h", "fpu_control.h", "ieee754.h", "values.h"]

INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}
FLOAT_SUFFIX = {"float": ("f", 4), "double": ("", 8), "long double": ("L", 16), "_Float32": ("f32", 4), "_Float64": ("f64", 8),
                "_Float128": ("f128", 16), "_Float32x": ("f32x", 8), "_Float64x": ("f64x", 16)}


def decimal_literal(hexconst):
    """The exact value of a probed hexadecimal floating constant as a decimal literal (60 digits: well above the 36 a
    binary128 value needs, so the compiler reads it back as the same value). Hexadecimal floating constants are not
    tokens in C89 and C++98 to C++14, and the unit is compiled in every mode."""
    neg = hexconst.startswith("-")
    h = hexconst.lstrip("-")
    mant, exp = h[2:].split("p")
    ip, _, fp = mant.partition(".")
    m = int((ip + fp) or "0", 16)
    e = int(exp) - 4 * len(fp)
    with decimal.localcontext() as ctx:
        ctx.prec = 60
        v = decimal.Decimal(m) * (decimal.Decimal(2) ** e)
    return ("-" if neg else "") + format(v, "e")


def expand(configs, ids):
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def p1(ctx):
    """[(configuration id, label, unit text)] of the table P1 of this slice's headers."""
    # A macro of a type the generator has no literal form for stays undefined and is a reviewed exception of
    # headers/rows-exceptions.tsv (HEADERS-TABLES-COMPLETE): P1 asserts only what the generator emits.
    excepted = set()
    with open(os.path.join(ctx.root, "headers", "rows-exceptions.tsv")) as f:
        for l in f:
            c = l.rstrip("\n").split("\t")
            if len(c) > 1 and c[0] == "macros":
                excepted.add(c[1])
    out = {}
    for r in ctx.vis:
        h = r["header"]
        if h not in HEADERS or r["kind"] not in ("macro", "typedef", "struct", "union"):
            continue
        n = r["name"]
        if n in excepted:
            continue
        if n.startswith("__") or (n.startswith("_") and n.endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        tests = []
        m = ctx.macros.get(n)
        if r["kind"] == "macro" and m and m["type"] in INT_SUFFIX and m["header"] == h:
            v = int(m["value"])
            sfx = INT_SUFFIX[m["type"]]
            # The most negative value has no literal (its magnitude is unsigned): (-max - 1).
            lit = "(%d%s)" % (v, sfx) if v > -(1 << 31) or (v != -(1 << 31) and v != -(1 << 63)) else "(%d%s-1)" % (v + 1, sfx)
            tests.append(f"(({n}) == {lit})")
        elif r["kind"] == "macro" and m and m["type"] in FLOAT_SUFFIX and m["header"] == h and m["value"].startswith("0x"):
            # A floating macro (C 7.12, TS 18661): its size is its type's and its value the exact probed constant.
            suffix, size = FLOAT_SUFFIX[m["type"]]
            tests.append(f"H:sizeof({n}) == {size}")
            tests.append(f"F:(({n}) == {decimal_literal(m['value'])}{suffix})")
        elif r["kind"] == "typedef" and n in ctx.typedefs and ctx.typedefs[n]["size"]:
            t = ctx.typedefs[n]
            tests.append(f"sizeof({n}) == {t['size']}")
            tests.append(f"__alignof__({n}) == {t['align']}")
        elif r["kind"] in ("struct", "union") and (r["kind"] + " " + n) in ctx.layouts:
            t = ctx.layouts[r["kind"] + " " + n]
            tests.append(f"sizeof({r['kind']} {n}) == {t['offset_or_size']}")
            tests.append(f"__alignof__({r['kind']} {n}) == {t['align']}")
        for cid in expand(r["configs"], ctx.ids):
            out.setdefault((h, cid), []).extend(tests)
    units = []
    for (h, cid), tests in sorted(out.items()):
        if tests:
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests) if t[:2] not in ("F:", "H:"))
            # A floating macro the generator spells as a hexadecimal constant is a token only where the language has
            # hexadecimal floating constants (C99 on, C++17 on): earlier modes cannot use it (RULING NEEDED,
            # HEADERS-TABLES-COMPLETE: a decimal spelling there), so the unit names it only where the use can compile.
            ht = [(i, t[2:]) for i, t in enumerate(tests) if t.startswith("H:")]
            if ht:
                body += "#if (defined __cplusplus && __cplusplus >= 201703L) || (!defined __cplusplus && defined __STDC_VERSION__ && __STDC_VERSION__ >= 199901L)\n"
                body += "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in ht) + "#endif\n"
            ft = [t[2:] for t in tests if t.startswith("F:")]
            if ft:
                body += "#if defined __cplusplus && __cplusplus >= 201703L\n"
                body += "".join(f"static_assert({t}, \"niji_p1_float\");\n" for t in ft)
                body += "#elif !defined __cplusplus && defined __STDC_VERSION__ && __STDC_VERSION__ >= 201112L\n"
                body += "".join(f"_Static_assert({t}, \"niji_p1_float\");\n" for t in ft) + "#endif\n"
            units.append((cid, f"P1 {h}", body))
    return units


def units(ctx):
    out = p1(ctx)
    # P6: the generic macros of tgmath.h choose the function by argument type; ieee754.h's bit-fields are the formats'
    # fields (the second unit needs the optimiser: its text says `niji: codegen`). Both read in the GNU configuration.
    for label, name in (("P6 tgmath", "tgmath_types.c"), ("P6 ieee754 fields", "ieee754_fields.c")):
        with open(os.path.join(ctx.root, "spec", "tools", name)) as f:
            out.append(("gnu", label, f.read()))
    return out
