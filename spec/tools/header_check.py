#!/usr/bin/env python3
"""Check libniji's generated headers (plan/headers.md section 3; SL-HEADERS-S05): P5 hygiene, the level model,
and P1 identity of the C 7.20 integer types.

    python3 spec/tools/header_check.py [--work DIR] [--include DIR]

Needs gcc and g++ only (any host; no reference headers are read: every unit is compiled with `-nostdinc
-isystem <DIR> -isystem <the compiler's own include>`, DIR = libc-include/ by default, or the tree `cargo xtask
so` staged, target/niji/include). For each configuration of spec/abi/ftm-configs.tsv:

* levels  a unit that includes features.h and `#error`s unless each `__NIJI_<level>` equals its
          headers/levels.tsv cell; it must compile, or fail for a configuration levels.tsv rejects;
* P5      each header of HEADERS compiles alone and included twice, `-Wall -Wextra -Wsystem-headers -Werror`;
* P1      (ruling #6047) every C 7.20.1 type spelled by header_probe.c_integer_names(): C asserts its size and
          alignment, C++ asserts `__is_same` with the typedefs.tsv `cxx` spelling (the spelling enters C++
          linkage names, so `long` against `long long` is ABI).

Prints one FAIL line per failing unit and `header_check: N failed of M`; exit 1 when any failed.
"""
import argparse
import glob
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import header_probe as hp  # noqa: E402  (ROOT, c_integer_names)

ROOT = hp.ROOT
HEADERS = ["features.h", "features-time64.h", "stdc-predef.h", "sys/cdefs.h", "sys/types.h", "sys/bitypes.h",
           "stdint.h", "inttypes.h", "limits.h", "endian.h", "byteswap.h", "errno.h", "sys/errno.h", "alloca.h",
           "gnu-versions.h", "gnu/stubs.h",
           # slice 6
           "stdio.h", "stdlib.h", "string.h", "strings.h", "memory.h", "unistd.h", "sys/unistd.h", "assert.h", "ctype.h",
           "getopt.h", "time.h",
           # slice 11
           "math.h", "complex.h", "tgmath.h", "fenv.h", "fpu_control.h", "ieee754.h", "values.h"]
COLS = [("isoc", "ISOC"), ("posix", "POSIX"), ("xsi", "XSI"), ("misc", "MISC"), ("gnu", "GNU"), ("lfs64", "LFS64"),
        ("fob64", "FOB64"), ("time64", "TIME64"), ("dynstack", "DYNSTACK"), ("fortify", "FORTIFY"), ("cxx", "CXX")]


SLICE6 = ["stdio.h", "stdlib.h", "string.h", "strings.h", "memory.h", "unistd.h", "sys/unistd.h", "assert.h", "ctype.h",
          "getopt.h", "time.h", "math.h", "complex.h", "tgmath.h", "fenv.h", "fpu_control.h", "ieee754.h", "values.h"]
FLOAT_SUFFIX = {"float": ("f", 4), "double": ("", 8), "long double": ("L", 16), "_Float32": ("f32", 4), "_Float64": ("f64", 8),
                "_Float128": ("f128", 16), "_Float32x": ("f32x", 8), "_Float64x": ("f64x", 16)}
INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


def expand(configs, ids):
    """The configuration ids a visibility.tsv `configs` cell names."""
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def decimal_literal(hexconst):
    """The exact value of a probed hexadecimal floating constant as a decimal literal (60 digits: well above the 36 a
    binary128 value needs, so the compiler reads it back as the same value). Hexadecimal floating constants are not
    tokens in C89 and C++98 to C++14, and the unit is compiled in every mode."""
    import decimal
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


def p1_units(ids, vis, macros, typedefs, layouts):
    """P1 of slice 6 (plan/headers.md section 3): per (header, configuration) one unit that includes the header and
    asserts, for what the header makes visible there, every macros.tsv integer value, every typedef size and
    alignment of typedefs.tsv and every structure size and alignment of layouts.tsv. A false condition is a
    negative array size, which every language mode rejects (C89 and C++98 have no static assertion)."""
    out = {}
    # A template with `@@emit-c` (slice 11) decides a name by its C configurations only: the C++ names of such a header
    # come from the C++ standard library's wrappers, so a name with no C row is not the header's (same filter as headers-gen).
    emit_c = set()
    for t in glob.glob(os.path.join(ROOT, "headers", "**", "*.h.in"), recursive=True):
        with open(t) as f:
            if any(l.startswith("@@emit-c") for l in f):
                emit_c.add(os.path.relpath(t, os.path.join(ROOT, "headers"))[:-3])
    c_names = {(r["header"], r["name"]) for r in vis if any(not c.startswith("cxx") for c in re.split(r"[,!]", r["configs"]) if c)}
    for r in vis:
        h = r["header"]
        if h not in SLICE6 or r["kind"] not in ("macro", "typedef", "struct", "union"):
            continue
        n = r["name"]
        if h in emit_c and (h, n) not in c_names:
            continue
        if n.startswith("__") or (n.startswith("_") and n.endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        tests = []
        if r["kind"] == "macro" and n in macros and macros[n]["type"] in INT_SUFFIX and macros[n]["header"] == h:
            m = macros[n]
            v = int(m["value"])
            sfx = INT_SUFFIX[m["type"]]
            # The most negative value has no literal (its magnitude is unsigned): (-max - 1).
            lit = "(%d%s)" % (v, sfx) if v > -(1 << 31) or (v != -(1 << 31) and v != -(1 << 63)) else "(%d%s-1)" % (v + 1, sfx)
            tests.append(f"(({n}) == {lit})")
        elif r["kind"] == "macro" and n in macros and macros[n]["type"] in FLOAT_SUFFIX and macros[n]["header"] == h and macros[n]["value"].startswith("0x"):
            # A floating macro (C 7.12, TS 18661): its size is its type's and its value the exact probed constant.
            m = macros[n]
            suffix, size = FLOAT_SUFFIX[m["type"]]
            tests.append(f"sizeof({n}) == {size}")
            tests.append(f"F:(({n}) == {decimal_literal(m['value'])}{suffix})")
        elif r["kind"] == "typedef" and n in typedefs and typedefs[n]["size"]:
            t = typedefs[n]
            tests.append(f"sizeof({n}) == {t['size']}")
            tests.append(f"__alignof__({n}) == {t['align']}")
        elif r["kind"] in ("struct", "union") and (r["kind"] + " " + n) in layouts:
            t = layouts[r["kind"] + " " + n]
            tests.append(f"sizeof({r['kind']} {n}) == {t['offset_or_size']}")
            tests.append(f"__alignof__({r['kind']} {n}) == {t['align']}")
        for cid in expand(r["configs"], ids):
            out.setdefault((h, cid), []).extend(tests)
    units = {}
    for (h, cid), tests in out.items():
        if tests:
            # A floating comparison is not an integer constant expression: those use _Static_assert (C11, C++11).
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests) if not t.startswith("F:"))
            ft = [t[2:] for t in tests if t.startswith("F:")]
            if ft:
                body += "#if defined __cplusplus && __cplusplus >= 201103L\n"
                body += "".join(f"static_assert({t}, \"niji_p1_float\");\n" for t in ft)
                body += "#elif !defined __cplusplus && defined __STDC_VERSION__ && __STDC_VERSION__ >= 201112L\n"
                body += "".join(f"_Static_assert({t}, \"niji_p1_float\");\n" for t in ft) + "#endif\n"
            units.setdefault(cid, []).append((f"P1 {h}", body))
    return units


def rows(rel):
    out, head = [], None
    with open(os.path.join(ROOT, rel)) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            cells = line.rstrip("\n").split("\t")
            if head is None:
                head = cells
            else:
                out.append(dict(zip(head, cells)))
    return out


def identity_unit(cxx, types):
    """P1 lines for the C 7.20.1 types (typedefs.tsv rows), C or C++."""
    lines = ["#include <stdint.h>"]
    for name, r in types:
        if cxx:
            lines.append(f"static_assert(__is_same({name}, {r['cxx']}), \"{name} is {r['cxx']}\");")
        else:
            lines.append(f"_Static_assert(sizeof({name}) == {r['size']} && _Alignof({name}) == {r['align']}, \"{name}\");")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", default="")
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    args = ap.parse_args()
    inc = os.path.abspath(args.include)
    if not os.path.isfile(os.path.join(inc, "features.h")):
        sys.exit(f"header_check: no features.h under {inc}")
    work = args.work or tempfile.mkdtemp(prefix="header_check.")
    os.makedirs(work, exist_ok=True)
    cfgs = rows("spec/abi/ftm-configs.tsv")
    levels = {r["id"]: r for r in rows("headers/levels.tsv")}
    typedefs = {r["name"]: r for r in rows("spec/abi/typedefs.tsv") if r["kind"] == "typedef"}
    wanted = [n for n in hp.c_integer_names() if n.islower()] + ["intmax_t", "uintmax_t", "intptr_t", "uintptr_t"]
    missing = [n for n in wanted if n not in typedefs]
    types = [(n, typedefs[n]) for n in wanted if n in typedefs]
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    vis = rows("spec/abi/visibility.tsv")
    macros = {r["name"]: r for r in rows("spec/abi/macros.tsv")}
    layouts = {}
    for r in rows("spec/abi/layouts.tsv"):
        if not r["member"]:
            layouts[r["type"]] = r
    p1s = p1_units([c["id"] for c in cfgs], vis, macros, typedefs, layouts)
    fails, total = len(missing), len(missing)
    for n in missing:
        print(f"FAIL P1 {n}: no typedefs.tsv row")
    for c in cfgs:
        cxx = c["language"] == "c++"
        ext = "cc" if cxx else "c"
        base = ["g++" if cxx else "gcc", *c["flags"].split(), "-nostdinc", "-isystem", inc,
                "-isystem", gccinc, "-fsyntax-only", "-Wall", "-Wextra", "-Wsystem-headers", "-Werror"]
        lv = levels[c["id"]]
        rejected = lv["reject"] == "1"
        units = [("levels", "#include <features.h>\n" + "".join(
            f"#if __NIJI_{mac} != {lv[col]}\n#error \"__NIJI_{mac} is not {lv[col]}\"\n#endif\n" for col, mac in COLS))]
        if not rejected:
            units += [(f"P5 {h}", f"#include <{h}>\n#include <{h}>\nint niji_check_tu;\n") for h in HEADERS]
            # static_assert is C++11 and _Static_assert C11 (C89 mode has neither as a keyword).
            if c["id"] not in ("cxx98", "c89", "isoc99"):
                units.append(("P1 stdint", identity_unit(cxx, types)))
        if not rejected:
            units += p1s.get(c["id"], [])
        if c["id"] == "gnu":
            with open(os.path.join(ROOT, "spec", "tools", "tgmath_types.c")) as f:
                units.append(("P6 tgmath", f.read()))
            with open(os.path.join(ROOT, "spec", "tools", "ieee754_fields.c")) as f:
                units.append(("P6 ieee754 fields", f.read()))
        for i, (what, text) in enumerate(units):
            src = os.path.join(work, f"{c['id']}-{i}.{ext}")
            with open(src, "w") as f:
                f.write(text)
            # A unit whose first lines say `niji: codegen` needs the optimiser (its checks are folded, not evaluated by the parser).
            cmd = [("-O1" if a == "-fsyntax-only" else a) for a in base] + ["-S", "-o", os.devnull] if "niji: codegen" in text else base
            r = subprocess.run(cmd + [src], capture_output=True, text=True)
            total += 1
            ok = (r.returncode != 0) if (rejected and what == "levels") else (r.returncode == 0)
            if not ok:
                fails += 1
                print(f"FAIL {what} {c['id']}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    print(f"header_check: {fails} failed of {total}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
