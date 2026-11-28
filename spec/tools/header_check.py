#!/usr/bin/env python3
"""Check libniji's generated headers (plan/headers.md section 3; SL-HEADERS-S05): P5 hygiene, the level model,
P1 identity with the tables and P8 kernel UAPI coexistence.

    python3 spec/tools/header_check.py [--work DIR] [--include DIR]

Needs gcc and g++ (clang-19 or clang for the clang P5 units, optional; any host; no reference headers are read: every unit is compiled with `-nostdinc
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
          and with clang where one is installed (`clang_p5`: C and C++, strict and GNU modes; skipped with a printed
          reason otherwise);
* P1      per (header, configuration), for the rows with `p1` 1: every macros.tsv integer value (in each header that
          shows the macro, whichever header owns its row; reserved names only in the owner), typedefs.tsv size and
          alignment and layouts.tsv structure size and alignment of what visibility.tsv makes visible there;
* slice   the units of each `header_checks/<slice>.py` (`units(ctx)`, and FAIL lines from `problems(ctx)`), each
          followed by its fragments `header_checks/<slice>.d/<NN-family>.py` in file-name order (the same interface:
          a story adds a family's checks as a new fragment, HEADERS-FRAGMENTS); a module or fragment may also name `p1_skip` ((header, name) rows P1 leaves out) and `must_fail` (headers shipped to fail: no P5
          unit, and a unit titled "... must fail" passes only when it fails with libniji's "is not provided" #error);
* P8      each header with the kernel UAPI headers of its `uapi` cell, both include orders (see `p8`).
* P9      each header that shows a typedef the compiler's <stddef.h> or <stdarg.h> defines, with those headers
          before and after it (see `compiler_order_units`).
* P1G     the include guards headers/reserved.tsv emits: defined with their value where visibility.tsv shows them in
          a `p1` 1 header, not defined where it does not (see `guard_units`).

Prints one FAIL line per failing unit and `header_check: N failed of M`; exit 1 when any failed. `--only REGEX` runs
only the units whose title matches (no slice-module problems, no P8; a plant's command); selecting none fails.
"""
import argparse
import glob
import hashlib
import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import types

# Check fragments (header_checks/<slice>.d/*.py) import shared helpers (`expand`) from this module by name, also when
# it runs as a script.
sys.modules.setdefault("header_check", sys.modules[__name__])
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
INT_SUFFIX = {"int": "", "char": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


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
    """The per-slice check modules spec/tools/header_checks/*.py, in file-name order, each followed by its fragments
    header_checks/<slice>.d/*.py in file-name order (named `<slice>.d/<fragment>`)."""
    mods = []

    def load(name, path):
        key = re.sub(r"[^A-Za-z0-9_]", "_", name)
        spec = importlib.util.spec_from_file_location(f"header_checks_{key}", path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        mods.append((name, mod))

    for path in sorted(glob.glob(os.path.join(CHECKS, "*.py"))):
        name = os.path.basename(path)[:-3]
        if name.startswith("_"):
            continue
        load(name, path)
        for frag in sorted(glob.glob(os.path.join(CHECKS, name + ".d", "*.py"))):
            load(f"{name}.d/{os.path.basename(frag)[:-3]}", frag)
    # A fragment directory without its slice module would be skipped silently.
    for d in sorted(glob.glob(os.path.join(CHECKS, "*.d"))):
        if not os.path.isfile(d[:-2] + ".py"):
            raise SystemExit(f"{d}: no slice module {os.path.basename(d)[:-2]}.py beside it")
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


COMPILER_TYPEDEFS = ("max_align_t", "size_t", "ptrdiff_t", "wchar_t", "wint_t", "nullptr_t", "va_list", "__gnuc_va_list")
ORDER_CONFIGS = ("c89", "c11", "gnu2x", "cxx98", "cxx11", "cxx23")


def compiler_order_units(ctx):
    """P9 (HEADERS-STAGE2-GAPS-A; no spec clause: C 7.21 and 7.16 give <stddef.h> and <stdarg.h> to the compiler):
    each header that shows a typedef the compiler's own <stddef.h> or <stdarg.h> also defines (visibility.tsv)
    compiles with those two headers included before it and after it, in C89, C11, GNU C2X, C++98, C++11 and C++23.
    A typedef the header spells itself as another type (an anonymous `max_align_t` structure is a type of its own)
    conflicts in one order or both; a header that relies on the compiler's guard macros fails the order it does not
    guard."""
    heads = sorted({r["header"] for r in ctx.vis if r["kind"] == "typedef" and r["name"] in COMPILER_TYPEDEFS
                    and r["header"] != "-"} & {r["header"] for r in ctx.slice_rows})
    tail = "#include <stddef.h>\n#include <stdarg.h>\n"
    return {cid: [(f"P9 {h} {o}", (tail + f"#include <{h}>\n" if o == "after" else f"#include <{h}>\n" + tail)
                   + "int niji_p9_tu;\n") for h in heads for o in ("after", "before")]
            for cid in ORDER_CONFIGS if cid in ctx.ids}


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
        # A header whose name has another value in the reference (spec/abi/macro-values.tsv) is asserted with that one.
        m = ctx.header_values.get((h, n), macros.get(n))
        if r["kind"] == "macro" and m and m["type"] in INT_SUFFIX and borrowed_ok:
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
            # A tag defined inside another structure (headers/nested-tags.tsv) is spelled through that structure's
            # member, the same type in C and C++ (C++ scopes the tag to the outer class).
            spell = nested_spelling(ctx, "struct " + n) or f"struct {n}"
            tests.append(f"sizeof({spell}) == {t['offset_or_size']}")
            tests.append(f"__alignof__({spell}) == {t['align']}")
        for cid in expand(r["configs"], ctx.ids):
            out.setdefault((h, cid), []).extend(tests)
    units = {}
    for (h, cid), tests in out.items():
        if tests:
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
            units.setdefault(cid, []).append((f"P1 {h}", body))
    return units


def nested_spelling(ctx, tag):
    """`__typeof__` of the member of the outer structure that has the nested tag `tag` as its (element) type, or None
    when headers/nested-tags.tsv does not nest it."""
    outer = ctx.nested_tags.get(tag)
    if outer is None:
        return None
    for r in ctx.rows("spec/abi/layouts.tsv"):
        # The member may sit in an unnamed union of the outer structure (`icmp_hun.ih_idseq`): the path is the
        # member access as written.
        if r["type"] == outer and r["member"] and "<anon>" not in r["member"] and r["member_type"].split("[")[0] == tag:
            idx = "[0]" if r["member_type"].endswith("]") else ""
            return f"__typeof__ ((({outer} *) 0)->{r['member']}{idx})"
    return None


def guard_units(ctx):
    """P1G (no spec clause: owner ruling R-H2, the reserved include guards that other software tests): for every
    include guard `headers/reserved.tsv` emits and every header with `p1` 1 that visibility.tsv shows it in, one unit
    per configuration. Where shown, the guard is defined with its macros.tsv value, usable in #if (perl's nostdio.h
    and the kernel's linux/libc-compat.h test it with #if); where not shown, it is not defined, since it states that
    its header was included. P1 leaves the reserved names out (they are emitted only where reserved.tsv says)."""
    emit = {r["name"] for r in ctx.rows("headers/reserved.tsv") if r["policy"] in ("emit", "emit-once")}
    guards = {n for n in emit if n.startswith("_") and not n.startswith("__") and n.rstrip("_").endswith("_H")}
    p1 = {r["header"] for r in ctx.slice_rows if r["p1"] == "1"}
    units = {}
    for r in ctx.vis:
        n, h = r["name"], r["header"]
        if n not in guards or h not in p1 or r["kind"] != "macro":
            continue
        shown = expand(r["configs"], ctx.ids)
        v = ctx.macros[n]["value"]
        for cid in ctx.ids:
            test = (f"#if !defined {n} || {n} != {v}\n#error \"{n} is not {v}\"\n#endif\n" if cid in shown
                    else f"#ifdef {n}\n#error \"{n} defined\"\n#endif\n")
            units.setdefault(cid, []).append((f"P1G {h} {n}", f"#include <{h}>\n{test}int niji_p1g_tu;\n"))
    return units


def uapi_flags():
    """`-isystem` for the kernel UAPI directories (linux-libc-dev) some headers include: $NIJI_UAPI (the tree
    `cargo xtask so` stages at target/niji/uapi) or, when unset, the build host's multiarch and plain include
    directories (their glibc headers stay out of reach: the units use -nostdinc and the libniji tree comes first)."""
    d = os.environ.get("NIJI_UAPI")
    if d:
        return ["-isystem", d]
    return ["-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"] if os.path.isdir("/usr/include/linux") else []


def kernel_dir(work):
    """The kernel's UAPI headers (linux-libc-dev; libniji ships none) for the units: a directory `work`/kernel-uapi of
    links to the build host's `linux`, `asm-generic` and `asm`, so the C library's own headers of the host stay out."""
    kern = os.path.join(work, "kernel-uapi")
    os.makedirs(kern, exist_ok=True)
    for d in ("linux", "asm-generic", "asm"):
        for base_dir in ("/usr/include", "/usr/include/x86_64-linux-gnu"):
            src = os.path.join(base_dir, d)
            if os.path.isdir(src) and not os.path.lexists(os.path.join(kern, d)):
                os.symlink(src, os.path.join(kern, d))
    return kern


def kernel_flags(work):
    """`-isystem` for the kernel UAPI headers as the units get them: $NIJI_UAPI when set, else `kernel_dir(work)`."""
    return uapi_flags() if os.environ.get("NIJI_UAPI") else ["-isystem", kernel_dir(work)]


def deprecation_flags(flags):
    """`-Wno-cpp` for flags (a list) that define _BSD_SOURCE or _SVID_SOURCE without _DEFAULT_SOURCE: <features.h>
    warns there with `#warning` that they are deprecated, as the reference's does (HEADERS-STAGE2-GAPS-M, #8815;
    s13.d/80 checks the warning itself). Any other flags keep every `#warning` an error."""
    names = {f[2:].split("=")[0] for f in flags if f.startswith("-D")}
    return ["-Wno-cpp"] if names & {"_BSD_SOURCE", "_SVID_SOURCE"} and "_DEFAULT_SOURCE" not in names else []


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


# P5 under clang (HEADERS-GETTEXT-CLANG: postgresql-17 compiles its JIT code with clang, which has none of the _FloatN
# keywords but _Float16): (title, language, flags), C and C++, a strict and a GNU mode of each, and strict C with
# _GNU_SOURCE (the configuration of that package's failure).
CLANG_CONFIGS = [("clang-c17", "c", "-std=c17"), ("clang-gnu2x", "c", "-std=gnu2x"),
                 ("clang-c17-gnu", "c", "-std=c17 -D_GNU_SOURCE"), ("clang-c++17", "c++", "-std=c++17"),
                 ("clang-gnu++20", "c++", "-std=gnu++20")]


def clang_compilers():
    """(clang, clang++): `NIJI_CLANG` (a C compiler path; its `++` twin beside it), else clang-19 or clang on PATH."""
    import shutil
    env = os.environ.get("NIJI_CLANG")
    for cc in ([env] if env else ["clang-19", "clang"]):
        path = shutil.which(cc)
        if path:
            cxx = shutil.which(cc.replace("clang", "clang++", 1))
            if cxx:
                return path, cxx
    return None


def clang_p5(inc, kern, work, headers, only):
    """P5 with clang: each header alone and twice, in each CLANG_CONFIGS mode, `-Wall -Wextra -Werror`. The search
    order is the compiler driver's own: clang's resource directory (its <stddef.h>, <float.h>, <tgmath.h>, ...) before
    the C library's. Skipped, with the reason printed, where no clang is installed. Returns (failed, run)."""
    cc = clang_compilers()
    if not cc:
        print("header_check: clang P5 skipped: no clang-19 or clang (and its clang++) on PATH, NIJI_CLANG unset")
        return 0, 0
    res = subprocess.run([cc[0], "-print-resource-dir"], capture_output=True, text=True, check=True).stdout.strip()
    fails = total = 0
    for cid, lang, flags in CLANG_CONFIGS:
        units = [(f"P5 {h} {cid}", f"#include <{h}>\n#include <{h}>\nint niji_check_tu;\n") for h in headers]
        if only:
            units = [u for u in units if only.search(u[0])]
        for i, (what, text) in enumerate(units):
            src = os.path.join(work, f"{cid}-{i}.{'cc' if lang == 'c++' else 'c'}")
            with open(src, "w") as f:
                f.write(text)
            cmd = [cc[1] if lang == "c++" else cc[0], *flags.split(), "-nostdinc", "-isystem", os.path.join(res, "include"),
                   "-isystem", inc, *(uapi_flags() if os.environ.get("NIJI_UAPI") else ["-isystem", kern]),
                   "-fsyntax-only", "-Wall", "-Wextra", "-Werror", src]
            r = subprocess.run(cmd, capture_output=True, text=True)
            total += 1
            if r.returncode != 0:
                fails += 1
                print(f"FAIL {what}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
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
    # A plant's command (docs/plants.md, 600 s per command): only the units whose title (`P9 sys/un.h after`, `P1
    # langinfo.h _NL_ITEM forms`, ...) matches; the slice modules' problems, their reports and P8 are not run. A
    # pattern that selects no unit is a failure, never a vacuous pass.
    ap.add_argument("--only", default="", metavar="REGEX")
    # The result list (HEADERS-FRAGMENTS): one line per configuration unit, `config  index  title  text-sha1  ok|FAIL`,
    # so a restructuring of the checks can show that it ran the same units with the same results (diff two lists).
    ap.add_argument("--list", default="", metavar="FILE")
    args = ap.parse_args()
    listing = open(args.list, "w") if args.list else None
    only = re.compile(args.only) if args.only else None
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
        header_values={(r["header"], r["name"]): r for r in rows("spec/abi/macro-values.tsv")},
        nested_tags={r["tag"]: r["outer"] for r in rows("headers/nested-tags.tsv")},
        typedefs={r["name"]: r for r in rows("spec/abi/typedefs.tsv") if r["kind"] == "typedef"}, layouts=layouts)
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    # The tree and work directory of this run, for a module that compiles units of its own (header_checks/form.py).
    ctx.inc, ctx.work = inc, work
    ctx.exact = lambda header, names, types=None: exact_units(ctx, header, names, types)
    ctx.expand = lambda configs: expand(configs, ctx.ids)
    mods = slice_checks()
    # A slice check module may name `p1_skip` ((header, name) rows its P1 units leave out) and `must_fail` (headers
    # shipped to fail in every configuration: no P5 TU, and the module's own unit asserts the failure).
    ctx.p1_skip = {x for _, mod in mods for x in getattr(mod, "p1_skip", ())}
    must_fail = {x for _, mod in mods for x in getattr(mod, "must_fail", ())}
    ctx.must_fail = must_fail
    p1s = p1_units(ctx)
    for cid, us in compiler_order_units(ctx).items():
        p1s.setdefault(cid, []).extend(us)
    for cid, us in guard_units(ctx).items():
        p1s.setdefault(cid, []).extend(us)
    for name, mod in mods:
        if not only:
            problems += getattr(mod, "problems", lambda _ctx: [])(ctx)
        for cid, what, body in getattr(mod, "units", lambda _ctx: [])(ctx):
            p1s.setdefault(cid, []).append((what, body))
    fails, total = len(problems), len(problems)
    for p in problems:
        print(f"FAIL {p}")
    kern = kernel_dir(work)
    for c in cfgs:
        cxx = c["language"] == "c++"
        ext = "cc" if cxx else "c"
        base = ["g++" if cxx else "gcc", *c["flags"].split(), "-nostdinc", "-isystem", inc,
                "-isystem", gccinc, *(uapi_flags() if os.environ.get("NIJI_UAPI") else ["-isystem", kern]),
                "-fsyntax-only", "-Wall", "-Wextra", "-Wsystem-headers", "-Werror", *deprecation_flags(c["flags"].split())]
        lv = levels[c["id"]]
        rejected = lv["reject"] == "1"
        units = [("levels", "#include <features.h>\n" + "".join(
            f"#if __NIJI_{mac} != {lv[col]}\n#error \"__NIJI_{mac} is not {lv[col]}\"\n#endif\n" for col, mac in COLS))]
        if not rejected:
            units += [(f"P5 {r['header']}", f"#include <{r['header']}>\n#include <{r['header']}>\nint niji_check_tu;\n") for r in srows if r["header"] not in must_fail]
            units += p1s.get(c["id"], [])
        if only:
            units = [u for u in units if only.search(u[0])]
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
            if listing:
                digest = hashlib.sha1(text.encode()).hexdigest()
                listing.write(f"{c['id']}\t{i}\t{what}\t{digest}\t{'ok' if ok else 'FAIL'}\n")
            if not ok:
                fails += 1
                print(f"FAIL {what} {c['id']}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    f5, t5 = clang_p5(inc, kern, work, [r["header"] for r in srows if r["header"] not in must_fail], only)
    fails, total = fails + f5, total + t5
    if only:
        if total == 0:
            print(f"FAIL --only {args.only}: no unit selected")
            fails = 1
    else:
        f8, t8 = p8(inc, gccinc, work, p8_pairs(srows))
        fails, total = fails + f8, total + t8
    if listing:
        listing.close()
    print(f"header_check: {fails} failed of {total}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
