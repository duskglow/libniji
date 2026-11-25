#!/usr/bin/env python3
"""Check libniji's generated headers (plan/headers.md section 3; SL-HEADERS-S05): P5 hygiene, the level model,
P1 identity with the tables and P8 kernel UAPI coexistence.

    python3 spec/tools/header_check.py [--work DIR] [--include DIR] [--only REGEX]

Needs gcc and g++ only (any host; no reference headers are read: every unit is compiled with `-nostdinc
-isystem <DIR> -isystem <the compiler's own include>`, DIR = libc-include/ by default, or the tree `cargo xtask
so` staged, target/niji/include).

Which headers are checked is data: one file per header slice, `headers/slices/<slice>.tsv`, one row per header
(columns `header`, `p1`, `uapi`; plan/headers.md section 4, "Adding a slice"). Checks of one slice only live in
`spec/tools/header_checks/<slice>.py`, found by listing that directory (SL-HEADERS-REGISTRY): a slice adds files and
edits no list or function here. For each configuration of spec/abi/ftm-configs.tsv:

* slices  every row names a header with a template `headers/<header>.in`, no header is in two rows, and every
          template is in a row (each problem one FAIL naming the row);
* levels  a unit that includes features.h and `#error`s unless each `__NIJI_<level>` equals its
          headers/levels.tsv cell; it must compile, or fail for a configuration levels.tsv rejects;
* P5      each header of the slice tables compiles alone and included twice, `-Wall -Wextra -Wsystem-headers -Werror`;
* P1      per (header, configuration), for the rows with `p1` 1: every macros.tsv integer value (in each header that
          shows the macro, whichever header owns its row; reserved names only in the owner), typedefs.tsv size and
          alignment and layouts.tsv structure size and alignment of what visibility.tsv makes visible there;
* slice   the units of each `header_checks/<slice>.py` (`units(ctx)`, and FAIL lines from `problems(ctx)`); a module
          may also name `p1_skip` ((header, name) rows P1 leaves out) and `must_fail` (headers shipped to fail: no P5
          unit, and a unit titled "... must fail" passes only when it fails with libniji's "is not provided" #error);
* P8      each header with the kernel UAPI headers of its `uapi` cell, both include orders (see `p8`).

Prints one FAIL line per failing unit and `header_check: N failed of M`; exit 1 when any failed.
"""
import argparse
import glob
import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import types

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import header_probe as hp  # noqa: E402  (ROOT, c_integer_names)

ROOT = hp.ROOT
SLICES = "headers/slices"
CHECKS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "header_checks")
COLS = [("isoc", "ISOC"), ("posix", "POSIX"), ("xsi", "XSI"), ("misc", "MISC"), ("gnu", "GNU"), ("lfs64", "LFS64"),
        ("fob64", "FOB64"), ("time64", "TIME64"), ("dynstack", "DYNSTACK"), ("fortify", "FORTIFY"), ("cxx", "CXX")]
# Macros whose definition is not a constant expression in some configurations (limits.h.in: a sysconf call where the
# minimum thread stack size is dynamic; signal.h's MINSIGSTKSZ and SIGSTKSZ under `_DYNAMIC_STACK_SIZE_SOURCE` and
# `_GNU_SOURCE`), so a static assertion on the value cannot be written for them.
NOT_CONSTANT = {"PTHREAD_STACK_MIN", "MINSIGSTKSZ", "SIGSTKSZ"}
# A reserved name (C 7.1.3: `_` and a capital or a second `_`) that a header shows but whose macros.tsv row another
# header owns: emitted only where headers/reserved.tsv says, and `_POSIX_C_SOURCE` takes its value from the
# configuration, so P1 asserts such a value only in the owning header.
RESERVED = re.compile(r"_[A-Z_]")
INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


def slice_rows():
    """(rows, problems): the rows of every headers/slices/*.tsv in file-name order, each a dict with its `where`
    (`headers/slices/s09.tsv:7`), and one message per row or template that breaks the rules of the module docstring."""
    out, problems, seen = [], [], {}
    for path in sorted(glob.glob(os.path.join(ROOT, SLICES, "*.tsv"))):
        rel = os.path.relpath(path, ROOT)
        head = None
        with open(path) as f:
            for n, line in enumerate(f, 1):
                if line.startswith("#") or not line.strip():
                    continue
                cells = line.rstrip("\n").split("\t")
                if head is None:
                    head = cells
                    if head[:3] != ["header", "p1", "uapi"]:
                        problems.append(f"{rel}:{n}: columns are {head}, not header, p1, uapi")
                        break
                    continue
                r = dict(zip(head, cells))
                r["where"] = f"{rel}:{n}"
                h = r.get("header", "")
                if not os.path.isfile(os.path.join(ROOT, "headers", h + ".in")):
                    problems.append(f"{r['where']}: {h}: no template headers/{h}.in")
                if h in seen:
                    problems.append(f"{r['where']}: {h} is also in {seen[h]}")
                    continue
                if r.get("p1") not in ("0", "1"):
                    problems.append(f"{r['where']}: {h}: p1 is {r.get('p1')!r}, not 0 or 1")
                seen[h] = r["where"]
                out.append(r)
    for t in sorted(glob.glob(os.path.join(ROOT, "headers", "**", "*.h.in"), recursive=True)):
        h = os.path.relpath(t, os.path.join(ROOT, "headers"))[:-3]
        if h not in seen:
            problems.append(f"headers/{h}.in: {h} is in no {SLICES}/*.tsv row")
    return out, problems


def slice_checks():
    """The per-slice check modules spec/tools/header_checks/*.py, in file-name order."""
    mods = []
    for path in sorted(glob.glob(os.path.join(CHECKS, "*.py"))):
        name = os.path.basename(path)[:-3]
        if name.startswith("_"):
            continue
        spec = importlib.util.spec_from_file_location(f"header_checks_{name}", path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        mods.append((name, mod))
    return mods


def expand(configs, ids):
    """The configuration ids a visibility.tsv `configs` cell names."""
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


STRING_MACRO = re.compile(r"char\[(\d+)\]")


def exact_units(ctx, header, names, types=None):
    """Units for slice check modules: each name of `names` in <header> exactly where visibility.tsv shows it there
    (no row: nowhere), one unit per configuration. Where shown: a macro is defined, a `char[N]` macros.tsv row has
    size N (and, C++11 on, its characters), a type-valued macro (`types`: name -> type) names that type, and a
    typedef has its typedefs.tsv size and alignment (C++: the `cxx` spelling, which enters linkage names). Where not
    shown: the macro is not defined, and the typedef name is free for the program (`extern int NAME;` compiles)."""
    types = types or {}
    vis = {r["name"]: r for r in ctx.vis if r["header"] == header and r["name"] in names}
    cfg = {c["id"]: c for c in ctx.cfgs}
    out = []
    for cid in ctx.ids:
        cxx, cxx11 = cfg[cid]["language"] == "c++", cfg[cid]["language"] == "c++" and cid != "cxx98"
        lines = [f"#include <{header}>"]
        if cxx11:
            lines.append("constexpr bool niji_streq(const char *a, const char *b) "
                         "{ return *a == *b && (*a == 0 || niji_streq(a + 1, b + 1)); }")
        for i, n in enumerate(names):
            r = vis.get(n)
            shown = r is not None and cid in expand(r["configs"], ctx.ids)
            kind = r["kind"] if r else ("typedef" if n in ctx.typedefs else "macro")
            tests = []
            if kind == "macro":
                lines.append(f"#if{'n' if shown else ''}def {n}\n#error \"{n} {'not ' if shown else ''}defined\"\n#endif")
                m = ctx.macros.get(n)
                size = STRING_MACRO.fullmatch(m["type"]) if m else None
                if shown and size:
                    tests.append(f"sizeof({n}) == {size.group(1)}")
                    if cxx11:
                        lines.append(f"static_assert(niji_streq({n}, {m['value']}), \"{n}\");")
                if shown and n in types:
                    tests.append(f"__is_same({n}, {types[n]})" if cxx else f"__builtin_types_compatible_p({n}, {types[n]})")
            elif shown:
                t = ctx.typedefs[n]
                tests += [f"sizeof({n}) == {t['size']}", f"__alignof__({n}) == {t['align']}"]
                if cxx:
                    tests.append(f"__is_same({n}, {t['cxx']})")
            else:
                lines.append(f"extern int {n};")
            lines += [f"typedef char niji_exact_{i}_{j}[({x}) ? 1 : -1];" for j, x in enumerate(tests)]
        out.append((cid, f"exact {header}", "\n".join(lines) + "\nint niji_exact_tu;\n"))
    return out


def p1_units(ctx):
    """P1 (plan/headers.md section 3) of the headers whose slice row has `p1` 1: per (header, configuration) one unit
    that includes the header and asserts, for what the header makes visible there, every macros.tsv integer value
    (whichever header owns the row; a reserved name only in its owner), every typedef size and alignment of typedefs.tsv and every structure size and alignment of layouts.tsv. A false
    condition is a negative array size, which every language mode rejects (C89 and C++98 have no static assertion)."""
    p1 = {r["header"] for r in ctx.slice_rows if r["p1"] == "1"}
    macros, typedefs, layouts = ctx.macros, ctx.typedefs, ctx.layouts
    out = {}
    for r in ctx.vis:
        h = r["header"]
        if h not in p1 or r["kind"] not in ("macro", "typedef", "struct"):
            continue
        n = r["name"]
        if n in NOT_CONSTANT:
            continue
        if n.startswith("__") or (n.startswith("_") and n.rstrip("_").endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        if (h, n) in ctx.p1_skip:
            continue  # a slice check module's `p1_skip` names it, with the reason
        if r["kind"] == "typedef" and n in typedefs and typedefs[n]["cxx"] == "_IO_FILE" and not any(
                v["header"] == h and v["name"] == "_IO_FILE" and v["kind"] == "struct" for v in ctx.vis):
            continue  # FILE named by a header that does not show struct _IO_FILE: the type stays incomplete there
        tests = []
        # A macro is asserted in every header that shows it, whichever header owns its macros.tsv row (the row's
        # value is the one name's value everywhere): an owner with `p1` 0, or one no slice ships, would otherwise leave
        # the value checked nowhere (HEADERS-GEN-SHARED). Reserved names only in their owner (RESERVED).
        borrowed_ok = n in macros and (macros[n]["header"] == h or not RESERVED.match(n))
        if r["kind"] == "macro" and n in macros and macros[n]["type"] in INT_SUFFIX and borrowed_ok:
            m = macros[n]
            v = int(m["value"])
            sfx = INT_SUFFIX[m["type"]]
            # The most negative 64-bit value has no literal: its magnitude does not fit the type (C 6.4.4.1).
            lit = ("(%d%s - 1)" % (v + 1, sfx)) if v in (-2**63, -2**31) and sfx in ("L", "LL", "") else "(%d%s)" % (v, sfx)
            tests.append(f"(({n}) == {lit})")
        elif r["kind"] == "typedef" and n in typedefs and typedefs[n]["size"]:
            t = typedefs[n]
            tests.append(f"sizeof({n}) == {t['size']}")
            tests.append(f"__alignof__({n}) == {t['align']}")
        elif r["kind"] == "struct" and ("struct " + n) in layouts:
            t = layouts["struct " + n]
            tests.append(f"sizeof(struct {n}) == {t['offset_or_size']}")
            tests.append(f"__alignof__(struct {n}) == {t['align']}")
        for cid in expand(r["configs"], ctx.ids):
            out.setdefault((h, cid), []).extend(tests)
    units = {}
    for (h, cid), tests in out.items():
        if tests:
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
            units.setdefault(cid, []).append((f"P1 {h}", body))
    return units


def uapi_flags():
    """`-isystem` for the kernel UAPI directories (linux-libc-dev) some headers include: $NIJI_UAPI (the tree
    `cargo xtask so` stages at target/niji/uapi) or, when unset, the build host's multiarch and plain include
    directories (their glibc headers stay out of reach: the units use -nostdinc and the libniji tree comes first)."""
    d = os.environ.get("NIJI_UAPI")
    if d:
        return ["-isystem", d]
    return ["-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"] if os.path.isdir("/usr/include/linux") else []


# P8 (plan/headers.md section 3): a libniji header and a kernel UAPI header that defines the same names (the `uapi` cell
# of its slice row). Both include orders must compile, as C and as C++, with and without _GNU_SOURCE; a pair whose
# kernel header is not installed is skipped.
P8_MODES = [("c", "-std=c17"), ("c", "-std=c17 -D_GNU_SOURCE"), ("c", "-std=gnu17"), ("c++", "-std=c++17 -D_GNU_SOURCE"),
            ("c++", "-std=gnu++17")]


def p8_pairs(slice_rows):
    """(libniji header, kernel header) for every `uapi` cell of the slice tables, in table order."""
    return [(r["header"], k) for r in slice_rows if r["uapi"] not in ("", "-") for k in r["uapi"].split(",")]


def reference_works(work):
    """Whether the system's own C library headers compile here (the reference of black-box probes)."""
    src = os.path.join(work, "ref.c")
    with open(src, "w") as f:
        f.write("#include <stdio.h>\n#include <sys/types.h>\nint niji_ref_tu;\n")
    return subprocess.run(["gcc", "-fsyntax-only", src], capture_output=True).returncode == 0


def p8(inc, gccinc, work, pairs):
    """(failures, units) of the P8 include-order units. The kernel's own headers conflict with a libc's for most
    of these pairs (only `linux/libc-compat.h`'s set has a protocol); a unit that does not compile with the reference
    system's headers either is printed as a NOTE and is not a failure (black-box parity, plan/headers.md 2.2)."""
    ref = reference_works(work)
    uapi = uapi_flags()
    dirs = [uapi[i + 1] for i in range(0, len(uapi), 2)]
    fails = total = 0
    for a, b in pairs:
        if not any(os.path.isfile(os.path.join(d, b)) for d in dirs):
            continue
        for lang, flags in P8_MODES:
            for order in ((a, b), (b, a)):
                src = os.path.join(work, f"p8-{total}.{'cc' if lang == 'c++' else 'c'}")
                with open(src, "w") as f:
                    f.write("".join(f"#include <{h}>\n" for h in order) + "int niji_p8_tu;\n")
                cmd = ["g++" if lang == "c++" else "gcc", *flags.split(), "-nostdinc", "-isystem", inc, "-isystem", gccinc, *uapi,
                       "-fsyntax-only", "-Wall", "-Wextra", "-Werror", src]  # a program's flags: no -Wsystem-headers (P5 has it)
                r = subprocess.run(cmd, capture_output=True, text=True)
                total += 1
                if r.returncode != 0 and ref:
                    rr = subprocess.run(["g++" if lang == "c++" else "gcc", *flags.split(), "-fsyntax-only", "-Wall", "-Wextra", "-Werror", src],
                                        capture_output=True)
                    if rr.returncode != 0:
                        print(f"NOTE P8 {order[0]} then {order[1]} [{flags}]: the reference system does not compile it either")
                        continue
                if r.returncode != 0:
                    fails += 1
                    print(f"FAIL P8 {order[0]} then {order[1]} [{flags}]: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    return fails, total


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


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", default="")
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--only", default="", help="run only the units, FAIL lines of the slice checks and P8 pairs whose "
                    "title matches this regular expression (a plant on one header's template: seconds, not minutes)")
    args = ap.parse_args()
    only = re.compile(args.only) if args.only else None
    wanted = (lambda title: only.search(title) is not None) if only else (lambda title: True)
    inc = os.path.abspath(args.include)
    if not os.path.isfile(os.path.join(inc, "features.h")):
        sys.exit(f"header_check: no features.h under {inc}")
    work = args.work or tempfile.mkdtemp(prefix="header_check.")
    os.makedirs(work, exist_ok=True)
    cfgs = rows("spec/abi/ftm-configs.tsv")
    levels = {r["id"]: r for r in rows("headers/levels.tsv")}
    srows, problems = slice_rows()
    layouts = {}
    for r in rows("spec/abi/layouts.tsv"):
        if not r["member"]:
            layouts[r["type"]] = r
    # What a slice's own checks (header_checks/<slice>.py) read: the tables, loaded once.
    ctx = types.SimpleNamespace(
        root=ROOT, hp=hp, rows=rows, cfgs=cfgs, ids=[c["id"] for c in cfgs], slice_rows=srows,
        vis=rows("spec/abi/visibility.tsv"), macros={r["name"]: r for r in rows("spec/abi/macros.tsv")},
        typedefs={r["name"]: r for r in rows("spec/abi/typedefs.tsv") if r["kind"] == "typedef"}, layouts=layouts)
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    # The tree and work directory of this run, for a module that compiles units of its own (header_checks/form.py).
    ctx.inc, ctx.work = inc, work
    ctx.exact = lambda header, names, types=None: exact_units(ctx, header, names, types)
    mods = slice_checks()
    # A slice check module may name `p1_skip` ((header, name) rows its P1 units leave out) and `must_fail` (headers
    # shipped to fail in every configuration: no P5 TU, and the module's own unit asserts the failure).
    ctx.p1_skip = {x for _, mod in mods for x in getattr(mod, "p1_skip", ())}
    must_fail = {x for _, mod in mods for x in getattr(mod, "must_fail", ())}
    p1s = p1_units(ctx)
    for name, mod in mods:
        problems += getattr(mod, "problems", lambda _ctx: [])(ctx)
        for cid, what, body in getattr(mod, "units", lambda _ctx: [])(ctx):
            p1s.setdefault(cid, []).append((what, body))
    problems = [p for p in problems if wanted(p)]
    fails, total = len(problems), len(problems)
    for p in problems:
        print(f"FAIL {p}")
    # The kernel's UAPI headers (linux-libc-dev; libniji ships none): a directory of links to the build host's
    # `linux`, `asm-generic` and `asm`, so the C library's own headers of the host stay out of the units.
    kern = os.path.join(work, "kernel-uapi")
    os.makedirs(kern, exist_ok=True)
    for d in ("linux", "asm-generic", "asm"):
        for base_dir in ("/usr/include", "/usr/include/x86_64-linux-gnu"):
            src = os.path.join(base_dir, d)
            if os.path.isdir(src) and not os.path.lexists(os.path.join(kern, d)):
                os.symlink(src, os.path.join(kern, d))
    for c in cfgs:
        cxx = c["language"] == "c++"
        ext = "cc" if cxx else "c"
        base = ["g++" if cxx else "gcc", *c["flags"].split(), "-nostdinc", "-isystem", inc,
                "-isystem", gccinc, *(uapi_flags() if os.environ.get("NIJI_UAPI") else ["-isystem", kern]),
                "-fsyntax-only", "-Wall", "-Wextra", "-Wsystem-headers", "-Werror"]
        lv = levels[c["id"]]
        rejected = lv["reject"] == "1"
        units = [("levels", "#include <features.h>\n" + "".join(
            f"#if __NIJI_{mac} != {lv[col]}\n#error \"__NIJI_{mac} is not {lv[col]}\"\n#endif\n" for col, mac in COLS))]
        if not rejected:
            units += [(f"P5 {r['header']}", f"#include <{r['header']}>\n#include <{r['header']}>\nint niji_check_tu;\n") for r in srows if r["header"] not in must_fail]
            units += p1s.get(c["id"], [])
        units = [u for u in units if wanted(u[0])]
        for i, (what, text) in enumerate(units):
            src = os.path.join(work, f"{c['id']}-{i}.{ext}")
            with open(src, "w") as f:
                f.write(text)
            r = subprocess.run(base + [src], capture_output=True, text=True)
            total += 1
            if what.endswith(" must fail"):
                # libniji's own header: the failure must be its #error, not a missing file or a syntax slip.
                ok = r.returncode != 0 and "is not provided" in r.stderr
            else:
                ok = (r.returncode != 0) if (rejected and what == "levels") else (r.returncode == 0)
            if not ok:
                fails += 1
                print(f"FAIL {what} {c['id']}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    f8, t8 = p8(inc, gccinc, work, [pr for pr in p8_pairs(srows) if wanted(f"P8 {pr[0]} then {pr[1]}")])
    fails, total = fails + f8, total + t8
    print(f"header_check: {fails} failed of {total}" + (f" (only units matching {args.only!r})" if only else ""))
    if only and total == 0:
        print("header_check: --only matched no unit")
        return 1
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
