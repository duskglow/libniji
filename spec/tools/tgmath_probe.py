#!/usr/bin/env python3
"""Probe what the type-generic macros of <tgmath.h> and the macros of <complex.h> resolve to (SL-HEADERS-COMPLEX-TGMATH).

    python3 spec/tools/tgmath_probe.py [--out FILE] [--niji-cc CMD] [--jobs N] [--configs id,...]

Without `--niji-cc` the probe compiles with the build host's own compilers and C library headers (run on the
reference system, Appendix C.0) and writes spec/abi/typegeneric.tsv (or --out). `--niji-cc CMD` compiles every unit
with that driver instead (scripts/niji-cc: libniji's staged headers, and in C++ the C++ library's headers above
them, as a program sees them); header_checks/s11.py uses `check()` for that, in batches.

Clean room (plan/headers.md section 2.2, ruling R-H0): each case is a translation unit of the probe's own that
includes one header and uses one macro. Read back are only the exit status and facts of the probe's own object
(compiled `-O0 -fno-builtin -fno-stack-protector -ffunction-sections -c`): the symbols the relocations of its function
`niji_f_K` reference (readelf -rW), and the sizes of its objects (nm -S), which carry a type code (C `_Generic`, C++
a template specialisation) and whether the name is a macro. No header text, `-E` or `-dM` output is read.

Rows (header, config, expr, symbol, type, form):
* tgmath.h  `MACRO(T1,T2,...)`: each argument a volatile object of that type (`int*` a pointer); the result is
  stored in a volatile object of its own type, so a call to a `const` function stays. `symbol` the functions the
  call references, comma-separated (`-` none; a conversion helper of libgcc counts), `type` the result type, `form`
  in C++ `macro` or `name` (an ordinary name of the C++ library), `-` in C. `undefined` when the macro is not defined
  (C), `error` when the unit does not compile.
* complex.h  `NAME` or `NAME(T,T)`: `symbol` `-`; `type` the expression's type; `form` in C `const` when it
  initializes a static object (an arithmetic constant expression), else `runtime`, `-` in C++. `complex` and
  `imaginary` are keyword macros: `type` is the type `double NAME` declares.
"""
import argparse
import concurrent.futures
import os
import re
import shlex
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, "spec", "abi", "typegeneric.tsv")

# (id, language, flags): strict C89, C99, C11 and C2X (the standard sets), the compiler's default (gnu17),
# _GNU_SOURCE (the extensions), C++98 (the C++ library's wrapper takes the C header) and g++'s default.
CONFIGS = [("c89", "c", ["-std=c89"]), ("c99", "c", ["-std=c99"]), ("c11", "c", ["-std=c11"]), ("c2x", "c", ["-std=c2x"]),
           ("gnu17", "c", []), ("gnu", "c", ["-D_GNU_SOURCE"]), ("cxx98", "c++", ["-std=c++98"]),
           ("gnuxx17", "c++", ["-std=gnu++17"])]
FLAGS = ["-O0", "-fno-builtin", "-fno-stack-protector", "-ffunction-sections", "-fno-common", "-w"]

REAL = ["float", "double", "long double"]
COMPLEX = ["float _Complex", "double _Complex", "long double _Complex"]
FLOATN = ["_Float32", "_Float64", "_Float128", "_Float32x", "_Float64x"]
CFLOATN = [t + " _Complex" for t in FLOATN]
INTS = ["int", "unsigned int", "long", "unsigned long", "long long", "unsigned long long", "short", "char", "_Bool"]
TYPES = REAL + COMPLEX + FLOATN + CFLOATN + INTS
CXX_TYPES = REAL + COMPLEX + ["int"]

# The macros of C 7.25 (C2X), with the GNU additions the visibility table shows in tgmath.h. Shapes: `x` a
# generic argument, `i` int, `l` long, `u` unsigned int, `p` int *, `L` long double.
SHAPES = {}
for m in ("acos asin atan acosh asinh atanh cos sin tan cosh sinh tanh exp log sqrt fabs cbrt ceil erf erfc exp2 expm1 "
          "floor ilogb lgamma llrint llround log10 log1p log2 logb lrint lround nearbyint rint round tgamma trunc exp10 "
          "roundeven nextup nextdown llogb carg cimag conj cproj creal").split():
    SHAPES[m] = "x"
for m in ("pow atan2 copysign fdim fmax fmin fmod hypot nextafter remainder fmaximum fmaximum_mag fmaximum_mag_num "
          "fmaximum_num fminimum fminimum_mag fminimum_mag_num fminimum_num fmaxmag fminmag scalb").split():
    SHAPES[m] = "xx"
SHAPES.update({"fma": "xxx", "frexp": "xp", "ldexp": "xi", "scalbn": "xi", "scalbln": "xl", "remquo": "xxp",
               "nexttoward": "xL", "fromfp": "xiu", "ufromfp": "xiu", "fromfpx": "xiu", "ufromfpx": "xiu"})
for p in ("f", "d", "f32", "f32x", "f64", "f64x"):
    for op in ("add", "sub", "mul", "div"):
        SHAPES[p + op] = "xx"
    SHAPES[p + "sqrt"] = "x"
    SHAPES[p + "fma"] = "xxx"
# Mixed argument types of the macros with more than one generic parameter (C 7.25 p3: the usual arithmetic
# conversions over the real types, a complex argument selecting the complex function).
MIXED = [("float", "double"), ("double", "float"), ("float", "long double"), ("long double", "double"), ("int", "float"),
         ("float", "int"), ("float", "double _Complex"), ("float _Complex", "double"),
         ("_Float32", "double"), ("_Float128", "long double"), ("float", "_Float64x")]

COMPLEX_MACROS = ["complex", "imaginary", "_Complex_I", "_Imaginary_I", "I"]
CMPLX_ARG = {"CMPLX": "double", "CMPLXF": "float", "CMPLXL": "long double", "CMPLXF32": "_Float32", "CMPLXF64": "_Float64",
             "CMPLXF128": "_Float128", "CMPLXF32X": "_Float32x", "CMPLXF64X": "_Float64x"}


def spell(t, cxx):
    return t.replace(" _Complex", " __complex__") if cxx else t


def type_code(expr, cxx):
    if cxx:
        return f"niji_ty<__typeof__({expr})>::v"
    assoc = ", ".join(f"{t}: {i + 1}" for i, t in enumerate(TYPES))
    return f"_Generic(({expr}), {assoc}, default: 99)"


def prelude(header, cxx):
    out = f"#include <{header}>\n"
    if cxx:
        out += "template <class T> struct niji_ty { enum { v = 99 }; };\n"
        out += "".join(f"template <> struct niji_ty<{spell(t, True)}> {{ enum {{ v = {TYPES.index(t) + 1} }}; }};\n" for t in CXX_TYPES)
    return out


def cases(cfg):
    """[(header, label, macro, part)] of one configuration; `part(k)` is the unit text of the case with suffix k."""
    cid, lang, _ = cfg
    cxx = lang == "c++"
    out = []
    tys = CXX_TYPES if cxx else TYPES
    for m, shape in sorted(SHAPES.items()):
        gen = shape.count("x")
        combos = [tuple([t] * gen) for t in tys]
        if gen > 1:
            combos += [tuple(list(p) + [p[-1]] * (gen - 2)) for p in MIXED if all(a in tys for a in p)]
        for combo in combos:
            it = iter(combo)
            decls, args, label = [], [], []
            for j, s in enumerate(shape):
                if s == "x":
                    t = next(it)
                    decls.append(f"volatile {spell(t, cxx)} niji_a{j}_@;")
                    label.append(t)
                else:
                    t = {"i": "int", "l": "long", "u": "unsigned int", "p": "int *", "L": "long double"}[s]
                    init = f"&niji_v{j}_@" if s == "p" else "0"
                    decls.append(f"int niji_v{j}_@; {t} niji_a{j}_@ = {init};")
                    label.append(t.replace(" ", ""))
                args.append(f"niji_a{j}_@")
            expr = f"{m}({', '.join(args)})"

            def part(k, m=m, decls=decls, expr=expr):
                e = expr.replace("@", k)
                t = "\n".join(decls).replace("@", k) + "\n"
                t += f"#ifndef {m}\n# ifndef __cplusplus\n#  error undefined\n# endif\nchar niji_m_{k}[1];\n#else\nchar niji_m_{k}[2];\n#endif\n"
                t += ("extern \"C\" " if cxx else "") + f"void niji_f_{k}(void) {{ volatile __typeof__({e}) niji_r = {e}; (void) niji_r; }}\n"
                return t + f"char niji_t_{k}[{type_code(e, cxx)}];\n"
            out.append(("tgmath.h", f"{m}({','.join(label)})", m, part))
    for m in COMPLEX_MACROS + list(CMPLX_ARG):
        a = CMPLX_ARG.get(m)
        expr, label = (f"{m}(({a})1, ({a})2)", f"{m}({a},{a})") if a else (m, m)

        def part(k, m=m, expr=expr):
            t = f"#ifndef {m}\n#error undefined\n#endif\n"
            if m in ("complex", "imaginary"):
                return t + f"double {m} niji_c_{k};\nchar niji_t_{k}[{type_code('niji_c_' + k, cxx)}];\n"
            t += f"char niji_t_{k}[{type_code(expr, cxx)}];\n"
            if not cxx:
                t += f"#ifdef NIJI_FORM\nstatic __typeof__({expr}) niji_c_{k} = {expr};\nvoid *niji_k_{k} = &niji_c_{k};\n#endif\n"
            return t
        out.append(("complex.h", label, m, part))
    return out


def relocs(obj):
    """{K: [symbols]} of the relocations in each function niji_f_K."""
    r = subprocess.run(["readelf", "-rW", obj], capture_output=True, text=True)
    out, k = {}, None
    for line in r.stdout.splitlines():
        h = re.match(r"Relocation section '[^']*niji_f_(\w+)'", line)
        if line.startswith("Relocation section"):
            k = h.group(1) if h else None
            if k is not None:
                out.setdefault(k, [])
            continue
        f = line.split()
        if k is not None and len(f) >= 5 and re.match(r"^[0-9a-f]+$", f[0]):
            name = f[4].split("@")[0]
            if not name.startswith("niji_") and not name.startswith("."):
                out[k].append(name)
    return {k: sorted(set(v)) for k, v in out.items()}


def sizes(obj):
    r = subprocess.run(["nm", "-S", obj], capture_output=True, text=True)
    return {f[3]: int(f[1], 16) for f in (l.split() for l in r.stdout.splitlines()) if len(f) == 4}


def compile_unit(compiler, cfg, text, path, extra=()):
    cid, lang, flags = cfg
    with open(path, "w") as f:
        f.write(text)
    return subprocess.run(compiler(lang) + flags + FLAGS + list(extra) + ["-c", path, "-o", path + ".o"], capture_output=True, text=True)


def facts(compiler, cfg, header, label, macro, part, obj, k, src_text, work):
    """The row of one case compiled into `obj` as suffix k."""
    cid, lang, _ = cfg
    sz = sizes(obj)
    code = sz.get(f"niji_t_{k}", 0)
    ty = TYPES[code - 1] if 1 <= code <= len(TYPES) else f"other{code}"
    if header == "tgmath.h":
        calls = relocs(obj).get(k, [])
        form = ("macro" if sz.get(f"niji_m_{k}") == 2 else "name") if lang == "c++" else "-"
        return (header, cid, label, ",".join(calls) or "-", ty, form)
    form = "-"
    if lang == "c" and macro not in ("complex", "imaginary"):
        # A static initializer: an arithmetic constant expression (C 6.7.9 p4) or not.
        r = compile_unit(compiler, cfg, src_text, os.path.join(work, f"{cid}-form-{k}.c"), ["-DNIJI_FORM"])
        form = "const" if r.returncode == 0 else "runtime"
    return (header, cid, label, "-", ty, form)


def failed(compiler, cfg, header, macro):
    cid, lang, flags = cfg
    und = subprocess.run(compiler(lang) + flags + ["-fsyntax-only", "-x", "c++" if lang == "c++" else "c", "-"],
                         input=f"#include <{header}>\n#ifndef {macro}\n#error u\n#endif\n", capture_output=True, text=True)
    # In C++ a tgmath name that is no macro is an ordinary name there (the unit still says what it does).
    return "undefined" if und.returncode != 0 and not (header == "tgmath.h" and lang == "c++") else "error"


def run_batch(compiler, cfg, batch, work, tag):
    """Rows of the cases of `batch` ([(index, case)]), one unit; None when the unit does not compile."""
    cid, lang, _ = cfg
    ext = "cc" if lang == "c++" else "c"
    by_header = {}
    for i, c in batch:
        by_header.setdefault(c[0], []).append((i, c))
    rows = {}
    for header, items in by_header.items():
        text = prelude(header, lang == "c++") + "".join(c[3](str(i)) for i, c in items)
        path = os.path.join(work, f"{cid}-{tag}-{header[0]}.{ext}")
        r = compile_unit(compiler, cfg, text, path)
        if r.returncode != 0:
            if len(items) == 1:
                i, c = items[0]
                w = failed(compiler, cfg, header, c[2])
                rows[i] = (header, cid, c[1], w, w, w)
                continue
            return None
        for i, c in items:
            single = prelude(header, lang == "c++") + c[3](str(i))
            rows[i] = facts(compiler, cfg, header, c[1], c[2], c[3], path + ".o", str(i), single, work)
    return rows


def probe(compiler, configs, jobs, select=None, batch=1):
    """Rows for every case of `configs` (or the (config, label) pairs `select` maps to True: batched, a failing
    batch retried case by case)."""
    work = tempfile.mkdtemp(prefix="tgmath_probe.")
    out = {}
    try:
        groups = []
        for cfg in configs:
            cs = [(i, c) for i, c in enumerate(cases(cfg)) if select is None or (cfg[0], c[0], c[1]) in select]
            ok = [x for x in cs if select is None or select[(cfg[0], x[1][0], x[1][1])]]
            bad = [x for x in cs if x not in ok]
            for j in range(0, len(ok), batch):
                groups.append((cfg, ok[j:j + batch]))
            groups += [(cfg, [x]) for x in bad]
        with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
            futs = {ex.submit(run_batch, compiler, cfg, b, work, f"b{n}"): (cfg, b) for n, (cfg, b) in enumerate(groups)}
            retry = []
            for f, (cfg, b) in futs.items():
                r = f.result()
                if r is None:
                    retry += [(cfg, [x]) for x in b]
                else:
                    out.update({(cfg[0], i): row for i, row in r.items()})
            futs2 = {ex.submit(run_batch, compiler, cfg, b, work, f"r{n}"): cfg for n, (cfg, b) in enumerate(retry)}
            for f, cfg in futs2.items():
                out.update({(cfg[0], i): row for i, row in f.result().items()})
    finally:
        subprocess.run(["rm", "-rf", work])
    order = {c[0]: n for n, c in enumerate(CONFIGS)}
    return [row for _, row in sorted(out.items(), key=lambda kv: (order[kv[0][0]], kv[0][1]))]


def compiler_for(niji_cc):
    if niji_cc:
        base = shlex.split(niji_cc)
        return lambda lang: base + (["-x", "c++"] if lang == "c++" else [])
    return lambda lang: ["g++" if lang == "c++" else "gcc"]


def read(path):
    rows = []
    with open(path) as f:
        for line in f:
            if not line.startswith("#") and not line.startswith("header\t"):
                rows.append(tuple(line.rstrip("\n").split("\t")))
    return rows


def write(rows, path):
    with open(path, "w") as f:
        f.write("# Generated by spec/tools/tgmath_probe.py on the reference system (spec/abi/PROVENANCE).\n")
        f.write("header\tconfig\texpr\tsymbol\ttype\tform\n")
        for r in rows:
            f.write("\t".join(r) + "\n")


def tree_compiler(inc):
    """The compiler of a header tree `inc` alone, as scripts/niji-cc orders it (no built products needed: the probe
    only compiles): in C++ the C++ library's directories first, then the tree, then the compiler's own."""
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    v = subprocess.run(["g++", "-x", "c++", "-E", "-v", "-"], input="", capture_output=True, text=True).stderr
    cxx = [d.strip() for d in v.split("search starts here:")[-1].split("End of search list.")[0].splitlines() if "/c++/" in d]
    tail = ["-nostdinc", "-isystem", inc, "-isystem", gccinc]
    return lambda lang: (["g++"] + [a for d in cxx for a in ("-isystem", d)] + tail) if lang == "c++" else (["gcc"] + tail)


def check(niji_cc, expected, jobs, sample=True, only=None):
    """Differences between the table `expected` and libniji's headers through `niji_cc` (a driver command, or a
    compiler function as `tree_compiler` returns): the cases the reference compiles, in batches; of those it does
    not, one per (configuration, macro, outcome) when `sample`; only the configuration ids `only` if given."""
    select, seen = {}, set()
    for h, cid, label, sym, ty, form in expected:
        if only is not None and cid not in only:
            continue
        ok = sym not in ("error", "undefined")
        key = (cid, label.split("(")[0], sym)
        if ok or not sample or key not in seen:
            seen.add(key)
            select[(cid, h, label)] = ok
    configs = [c for c in CONFIGS if any(k[0] == c[0] for k in select)]
    compiler = niji_cc if callable(niji_cc) else compiler_for(niji_cc)
    got = {(r[0], r[1], r[2]): r for r in probe(compiler, configs, jobs, select, batch=150)}
    diffs = []
    for r in expected:
        if (r[1], r[0], r[2]) in select:
            g = got.get((r[0], r[1], r[2]))
            if g != r:
                diffs.append(f"{r[0]} {r[1]} {r[2]}: reference {'/'.join(r[3:])}, libniji {'/'.join(g[3:]) if g else 'no row'}")
    return len(select), diffs


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--out", default=OUT)
    ap.add_argument("--niji-cc", default="")
    ap.add_argument("--check", default="", help="compare libniji (--niji-cc) with this table; print the differences")
    ap.add_argument("--all", action="store_true", help="with --check: every failing case, not one per macro")
    ap.add_argument("--jobs", type=int, default=min(32, os.cpu_count() or 2))
    ap.add_argument("--configs", default="")
    args = ap.parse_args()
    if args.check:
        n, diffs = check(args.niji_cc, read(args.check), args.jobs, not args.all)
        for d in diffs:
            print("DIFF " + d)
        print(f"tgmath_probe: {len(diffs)} differences in {n} cases")
        return 1 if diffs else 0
    configs = [c for c in CONFIGS if not args.configs or c[0] in args.configs.split(",")]
    rows = probe(compiler_for(args.niji_cc), configs, args.jobs)
    write(rows, args.out)
    print(f"tgmath_probe: {len(rows)} rows -> {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
