"""Macro form probe (HEADERS-MACRO-FORM-AUDIT; standing rule "probe a macro's form, not only its value", owner
2026-11-24): every object-like macro libniji's generated headers define has the reference's FORM in each probed
configuration, not only its value. A macro can have the reference's value and still be a constant on one side and a
run-time call on the other (MINSIGSTKSZ, SIGSTKSZ, PTHREAD_STACK_MIN under _DYNAMIC_STACK_SIZE_SOURCE), which decides
whether it compiles as a static array size, in `_Static_assert`, `#if` or a C++ `constexpr`.

Black box: the same probe unit is compiled against libniji's tree (`-nostdinc -isystem <tree>`, as header_check.py)
and against the build host's own C library headers (the reference); only the exit status and the line numbers of
the probe unit's own diagnostics are read (`-fdiagnostics-plain-output`, no caret, no text of any header), and the
sizes of the probe's own objects (`nm -S`). No reference header text is read.

Inventory: every `#define NAME` without a parameter list in the generated tree (libc-include/), except libniji's
own `__NIJI_*` names, each probed through every header that defines it (one macro can be spelled differently in two
headers); headers shipped to fail (`must_fail`) are left out.

Facts per (macro, configuration, side): `def` defined; `pp` usable in `#if`; `expr` usable as an expression;
`ice` an integer constant expression in C (`_Static_assert`) or a constant expression in C++ (`static_assert`);
`arr` usable as a file-scope array size; `cx` a C++ `constexpr` initializer; `ty` its type (a `_Generic` / overload
code, below) and `sz` its `sizeof`; `val` its value at run time (only with --values: needs a libniji to link).

As a header_check.py module: prints one `FORM-DIFF` line per (macro, configuration) whose form differs, grouped by
cause, one `FORM-RUNTIME` line per macro the reference defines as a run-time expression, and a summary. It fails the
run for a difference of type in C++ (an enumeration or `char` against `int`) or of enumeration (`ut` its underlying
type, `same-as` whether it is one type with the enumeration-typed name before it) (HEADERS-CXX-ENUM-TYPES), and for
any difference of an `ENFORCED` (macro, header) pair (HEADERS-FORM-BREAKS: the groups A1, B1, B2, B3); the other
groups stay report mode until their follow-ups land. Set
NIJI_FORM_PROBE=0 to skip it, NIJI_FORM_JOBS=<n> for the parallel compilers (default: CPUs, at most 16).

Stand-alone: python3 spec/tools/header_checks/form.py [--include DIR] [--work DIR] [--tsv FILE] [--diff FILE]
[--values --niji-cc CMD --niji-cxx CMD] [--headers H,...]
"""
import argparse
import concurrent.futures
import os
import re
import shlex
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_probe as hp  # noqa: E402

ROOT = hp.ROOT
# The configurations of the rule (developer-flow-reference section 13) and the brief: no feature-test macro (the
# compiler's default mode), strict C11, strict C11 with POSIX.1-2008, X/Open 7, _GNU_SOURCE, _DEFAULT_SOURCE,
# _DYNAMIC_STACK_SIZE_SOURCE (the macro that selects the run-time forms) and C++17 with g++'s defaults.
CONFIGS = [("none", "c", []), ("c11", "c", ["-std=c11"]), ("posix2008", "c", ["-std=c11", "-D_POSIX_C_SOURCE=200809L"]),
           ("xopen700", "c", ["-D_XOPEN_SOURCE=700"]), ("gnu", "c", ["-D_GNU_SOURCE"]), ("default", "c", ["-D_DEFAULT_SOURCE"]),
           ("dynstack", "c", ["-D_DYNAMIC_STACK_SIZE_SOURCE"]), ("cxx17", "c++", ["-std=c++17"])]
# Type codes (C `_Generic` association and C++ overload): the same number names the same type in both languages.
TYPES = [(1, "int"), (2, "unsigned int"), (3, "long"), (4, "unsigned long"), (5, "long long"), (6, "unsigned long long"),
         (7, "float"), (8, "double"), (9, "long double"), (10, "char *"), (11, "const char *"), (12, "void *"),
         (13, "_Bool"), (14, "char"), (15, "signed char"), (16, "unsigned char"), (17, "short"), (18, "unsigned short"),
         (19, "_Float32"), (20, "_Float64"), (21, "_Float128"), (22, "_Float32x"), (23, "_Float64x"),
         (24, "void (*)(int)"), (25, "const void *"), (26, "__int128"), (27, "unsigned __int128"), (28, "int *"),
         (29, "const int *")]
TYPE_NAME = dict(TYPES)
TYPE_NAME[30] = "other"
INT_TYPES = {"int", "unsigned int", "long", "unsigned long", "long long", "unsigned long long", "short", "unsigned short",
             "char", "signed char", "unsigned char", "_Bool"}
FACTS = ("def", "pp", "expr", "ice", "arr", "init", "cx")
C_ONLY, CXX_ONLY = {"init"}, {"cx"}
DEFINE = re.compile(r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)(\(?)")
OWN = ("__NIJI",)


def must_fail_headers():
    """The headers the slice modules ship to fail in every configuration (their `must_fail`)."""
    import header_check as hc
    return {x for _, mod in hc.slice_checks() for x in getattr(mod, "must_fail", ())}


def inventory(inc, macros_tsv=None, skip=()):
    """[(name, probe header, [every defining header])]: each object-like macro the tree defines."""
    where = {}
    for dirpath, _, files in os.walk(inc):
        for fn in files:
            if not fn.endswith(".h"):
                continue
            path = os.path.join(dirpath, fn)
            h = os.path.relpath(path, inc)
            with open(path, errors="replace") as f:
                for line in f:
                    m = DEFINE.match(line)
                    if m and not m.group(2) and not m.group(1).startswith(OWN):
                        where.setdefault(m.group(1), set()).add(h)
    out = []
    for n in sorted(where):
        hs = sorted(where[n])
        usable = [h for h in hs if h not in skip]
        if not usable:
            continue
        # Every defining header: one macro can be spelled differently in two headers (PTHREAD_STACK_MIN).
        out += [(n, h, hs) for h in usable]
    return out


def ty_c():
    return "_Generic((x), " + ", ".join(f"{t}: {k}" for k, t in TYPES) + ", default: 30)"


def cxx_prelude():
    lines = ["template<int K> struct niji_k { char a[K]; };", "template<class T> niji_k<30> niji_ty(T);"]
    lines += [f"niji_k<{k}> niji_ty({'bool' if t == '_Bool' else t});" for k, t in TYPES]
    # The underlying type of an enumeration (C++17 [dcl.enum]/7: a fixed or an implementation-chosen integer type that
    # holds every value), `int` for any other type: `std::underlying_type` without a standard header.
    lines += ["template<class T, bool = __is_enum(T)> struct niji_ut { typedef int type; };",
              "template<class T> struct niji_ut<T, true> { typedef __underlying_type(T) type; };",
              "template<class T> typename niji_ut<T>::type niji_ul();"]
    return "\n".join(lines) + "\n"


def compile_dropping(cmd, src, text_of, items):
    """Compiles text_of(items) (returns (text, {line: item})) with `cmd ... src` until it compiles clean: each round
    drops the items whose lines have an error. One clean compile of every item kept is the proof that each compiles:
    the C front end reports an undeclared identifier once only, so an item that shares one with a failing item can
    look clean in the round that names the other. Returns (items kept, True), or (items, False) when a round fails
    with no error on a line of the unit (the header itself fails, or a link error)."""
    for _ in range(200):
        if not items:
            return [], True
        text, at = text_of(items)
        with open(src, "w") as f:
            f.write(text)
        r = subprocess.run(cmd + [src], capture_output=True, text=True)
        if r.returncode == 0:
            return items, True
        bad = {at[ln] for ln in error_lines(r.stderr, src) if ln in at}
        if not bad or "fatal error" in r.stderr:
            return items, False
        items = [x for x in items if x not in bad]
    return items, False


def tu_tests(header, tests, cxx):
    """(text, {line: (index, fact)}) of the syntax unit over `tests` [(index, name, fact)]: one test per line, so a
    diagnostic's line names the test; each test but `def` under `#ifdef`."""
    lines = [f"#include <{header}>"]
    at = {}
    std = "static_assert" if cxx else "_Static_assert"
    for i, n, fact in tests:
        # `((M) & 1) + 1` depends on the value, so the compiler cannot fold it without M being a constant (an `M == M`
        # or `M ? 1 : 1` test can be folded for a call of a `const` function), and it is an integer test: a floating
        # or pointer macro is no integer constant expression and no array size.
        text = {"def": "#error niji-undefined", "pp": f"#if ({n}) || 1",
                "expr": f"static void niji_e_{i}({'' if cxx else 'void'}) {{ (void)({n}); }}",
                "ice": f"{std}((({n}) & 1) + 1, \"niji\");", "arr": f"typedef char niji_a_{i}[(({n}) & 1) + 1];",
                "init": f"static __typeof__({n}) niji_i_{i} = {n};",
                "cx": f"constexpr auto niji_c_{i} = ({n});"}[fact]
        lines.append(f"#ifndef {n}" if fact == "def" else f"#ifdef {n}")
        lines.append(text)
        at[len(lines)] = (i, n, fact)
        if fact == "pp":
            lines.append("#endif")
        lines.append("#endif")
    return "\n".join(lines) + "\n", at


def tu_types(header, names, cxx):
    """The object unit: per expression-usable macro, an array whose size is its type code and one of its sizeof."""
    lines = [f"#include <{header}>"] + (cxx_prelude() if cxx else f"#define NIJI_TY(x) {ty_c()}\n").rstrip("\n").split("\n")
    at = {}
    for i, n in names:
        lines.append(f"char niji_t_{i}[{'sizeof(niji_ty((' + n + ')))' if cxx else 'NIJI_TY(' + n + ')'}];")
        at[len(lines)] = (i, n)
        lines.append(f"char niji_s_{i}[sizeof({n})];")
        at[len(lines)] = (i, n)
        if cxx:  # an enumerator has its enumeration's type in C++ (C++17 [dcl.enum]), int in C (C11 6.7.2.2)
            lines.append(f"char niji_en_{i}[__is_enum(decltype(({n}))) ? 2 : 1];")
            at[len(lines)] = (i, n)
            lines.append(f"char niji_ut_{i}[sizeof(niji_ty(niji_ul<decltype(({n}))>()))];")
            at[len(lines)] = (i, n)
    return "\n".join(lines) + "\n", at


def tu_same(header, pairs):
    """The C++ object unit over [(index, name, previous name)] of enumeration-typed macros: whether each has the
    type of the one before it (one enumeration or two: `decltype` and templates tell them apart)."""
    lines = [f"#include <{header}>"]
    at = {}
    for i, n, p in pairs:
        lines.append(f"char niji_sp_{i}[__is_same(decltype(({n})), decltype(({p}))) ? 2 : 1];")
        at[len(lines)] = (i, n, p)
    return "\n".join(lines) + "\n", at


def tu_values(header, names, cxx):
    """The run unit: prints each macro's value (integers in decimal, floating values as %La, strings as hex bytes,
    pointers as an integer when within a page of 0 or -1, else `addr`)."""
    t = [f"#include <{header}>", "#include <stdio.h>"]
    t.append('static void niji_pi(const char *n, long long v) { printf("%s\\t%lld\\n", n, v); }')
    t.append('static void niji_pu(const char *n, unsigned long long v) { printf("%s\\t%llu\\n", n, v); }')
    t.append('static void niji_pf(const char *n, long double v) { printf("%s\\t%La\\n", n, v); }')
    t.append('static void niji_ps(const char *n, const char *v) { printf("%s\\ts:", n); while (*v) printf("%02x", (unsigned char) *v++); printf("\\n"); }')
    t.append('static void niji_pa(const char *n, unsigned long v) { if (v < 4096 || v > -4096UL) printf("%s\\tp:%ld\\n", n, (long) v); else printf("%s\\tp:addr\\n", n); }')
    if cxx:
        for ty in ("bool", "char", "signed char", "short", "int", "long", "long long"):
            t.append(f"static void niji_pv(const char *n, {ty} v) {{ niji_pi(n, v); }}")
        for ty in ("unsigned char", "unsigned short", "unsigned", "unsigned long", "unsigned long long", "unsigned __int128"):
            t.append(f"static void niji_pv(const char *n, {ty} v) {{ niji_pu(n, (unsigned long long) v); }}")
        t.append("static void niji_pv(const char *n, __int128 v) { niji_pi(n, (long long) v); }")
        for ty in ("float", "double", "long double", "_Float32", "_Float64", "_Float32x", "_Float64x", "_Float128"):
            t.append(f"static void niji_pv(const char *n, {ty} v) {{ niji_pf(n, (long double) v); }}")
        t.append("static void niji_pv(const char *n, const char *v) { niji_ps(n, v); }")
        t.append("template<class T> static void niji_pv(const char *n, T *v) { niji_pa(n, (unsigned long) v); }")
        t.append('template<class T> static void niji_pv(const char *n, T) { printf("%s\\tother\\n", n); }')
        call = "niji_pv"
    else:  # C
        t.append('static void niji_px(const char *n, ...) { printf("%s\\tother\\n", n); }')
        t.append("static void niji_pfn(const char *n, void (*v)(int)) { niji_pa(n, (unsigned long) v); }")
        t.append("static void niji_pvp(const char *n, const void *v) { niji_pa(n, (unsigned long) v); }")
        assoc = {"int": "niji_pi", "long": "niji_pi", "long long": "niji_pi", "short": "niji_pi", "signed char": "niji_pi",
                 "char": "niji_pi", "_Bool": "niji_pi", "__int128": "niji_pi", "unsigned int": "niji_pu",
                 "unsigned long": "niji_pu", "unsigned long long": "niji_pu", "unsigned short": "niji_pu",
                 "unsigned char": "niji_pu", "unsigned __int128": "niji_pu", "float": "niji_pf", "double": "niji_pf",
                 "long double": "niji_pf", "_Float32": "niji_pf", "_Float64": "niji_pf", "_Float128": "niji_pf",
                 "_Float32x": "niji_pf", "_Float64x": "niji_pf", "char *": "niji_ps", "const char *": "niji_ps",
                 "void *": "niji_pvp", "const void *": "niji_pvp", "void (*)(int)": "niji_pfn"}
        t.append("#define niji_pv(n, x) _Generic((x), " + ", ".join(f"{k}: {v}" for k, v in assoc.items()) + ", default: niji_px)(n, (x))")
        call = "niji_pv"
    t.append(f"int main({'' if cxx else 'void'}) {{")
    at = {}
    for i, n in names:
        t.append(f'  {call}("{n}", ({n}));')
        at[len(t)] = (i, n)
    t.append("  return 0;\n}")
    return "\n".join(t) + "\n", at


def error_lines(stderr, src):
    """The probe unit's lines that have an error: an error in the unit itself, or one inside a header whose
    `in expansion of macro` note points at the unit. Only file names, line numbers and the word `error` are read."""
    base = os.path.basename(src)
    out, pending = set(), False
    for line in stderr.splitlines():
        m = re.match(r"^(.*?):(\d+):(?:\d+:)? (error|fatal error|note|warning):", line)
        if not m:
            continue
        mine = os.path.basename(m.group(1)) == base
        kind = m.group(3)
        if kind in ("error", "fatal error"):
            if mine:
                out.add(int(m.group(2)))
                pending = False
            else:
                pending = True
        elif kind == "note" and pending and mine:
            out.add(int(m.group(2)))
            pending = False
    return out


class Side:
    """One side of the comparison: libniji's tree (`inc` set) or the build host's own headers."""

    def __init__(self, name, inc, gccinc, kern, cc=None, cxx=None):
        self.name, self.inc, self.gccinc, self.kern = name, inc, gccinc, kern
        self.cc, self.cxx = cc, cxx

    def flags(self):
        if not self.inc:
            return []
        return ["-nostdinc", "-isystem", self.inc, "-isystem", self.gccinc] + (["-isystem", self.kern] if self.kern else [])

    def compiler(self, lang, link=False):
        if link and self.inc:
            return shlex.split(self.cxx if lang == "c++" else self.cc)
        return ["g++" if lang == "c++" else "gcc"]


def probe_one(side, header, cfg, names, work, values):
    """{name: facts} of one (side, header, configuration)."""
    cid, lang, cflags = cfg
    cxx = lang == "c++"
    ext = "cc" if cxx else "c"
    stem = os.path.join(work, f"{side.name}-{cid}-{header.replace('/', '_')}")
    # gcc 14 makes these errors by default; with gcc 13 they are warnings that -w would hide, and a macro that compiles
    # only through an implicit declaration of a function its header does not declare is not usable.
    # No -w: it silences a warning that -Werror= promotes. Warnings never fail a unit; only error lines are read.
    diag = ["-fdiagnostics-plain-output", "-fno-diagnostics-show-caret"] + (
        [] if cxx else ["-Wsystem-headers", "-Werror=implicit-function-declaration", "-Werror=implicit-int"])
    tests = [(i, n, fact) for i, n in enumerate(names) for fact in FACTS if fact not in (C_ONLY if cxx else CXX_ONLY)]
    passed, ok = compile_dropping(side.compiler(lang) + cflags + side.flags() + diag + ["-fsyntax-only"], f"{stem}.{ext}",
                                  lambda xs: tu_tests(header, xs, cxx), tests)
    passed = set(passed)
    facts = {}
    for i, n in enumerate(names):
        if not ok:
            facts[n] = {"def": "fatal"}
            continue
        f = {fact: (None if fact in (C_ONLY if cxx else CXX_ONLY) else (i, n, fact) in passed) for fact in FACTS}
        if not f["def"]:
            f = {"def": False}
        facts[n] = f
    usable = [(i, n) for i, n in enumerate(names) if facts[n].get("expr")]
    for i, n in usable:
        facts[n]["ty"] = facts[n]["sz"] = "?"
    if usable:
        cmd = side.compiler(lang) + cflags + side.flags() + diag + ["-c", "-o", f"{stem}-ty.o"]
        done, ok = compile_dropping(cmd, f"{stem}-ty.{ext}", lambda xs: tu_types(header, xs, cxx), usable)
        if done and ok:
            nm = subprocess.run(["nm", "-S", f"{stem}-ty.o"], capture_output=True, text=True).stdout
            sizes = {}
            for line in nm.splitlines():
                p = line.split()
                if len(p) == 4 and p[3].startswith(("niji_t_", "niji_s_", "niji_en_", "niji_ut_")):
                    sizes[p[3]] = int(p[1], 16)
            for i, n in done:
                facts[n]["ty"] = "enum" if sizes.get(f"niji_en_{i}") == 2 else TYPE_NAME.get(sizes.get(f"niji_t_{i}"), "?")
                facts[n]["sz"] = sizes.get(f"niji_s_{i}", "?")
                if facts[n]["ty"] == "enum":
                    facts[n]["ut"] = TYPE_NAME.get(sizes.get(f"niji_ut_{i}"), "?")
            # Each enumeration-typed macro against the name before it in the list (when that one is one too): one
            # enumeration or two. A one-enumerator enumeration per name has the right type kind and the wrong identity.
            if cxx:
                pairs = [(i, n, names[i - 1]) for i, n in enumerate(names)
                         if i and facts[n].get("ut") and facts[names[i - 1]].get("ut")]
                sp, ok2 = compile_dropping(cmd[:-1] + [f"{stem}-sp.o"], f"{stem}-sp.cc", lambda xs: tu_same(header, xs), pairs)
                if sp and ok2:
                    nm = subprocess.run(["nm", "-S", f"{stem}-sp.o"], capture_output=True, text=True).stdout
                    same = {p[3]: int(p[1], 16) == 2 for p in (x.split() for x in nm.splitlines()) if len(p) == 4}
                    for i, n, p in sp:
                        facts[n]["sp"] = f"{p}:{same.get(f'niji_sp_{i}', '?')}"
    if values and usable and (side.cc or not side.inc):
        exe = f"{stem}-v"
        cmd = side.compiler(lang, link=True) + cflags + side.flags() + diag + ["-o", exe]
        done, ok = compile_dropping(cmd, f"{stem}-v.{ext}", lambda xs: tu_values(header, xs, cxx), usable)
        got = {}
        if done and ok:
            try:
                ran = subprocess.run([exe], capture_output=True, text=True, timeout=30)
                for line in ran.stdout.splitlines():
                    k, _, v = line.partition("\t")
                    got[k] = v
            except subprocess.TimeoutExpired:
                pass
        for _, n in usable:
            facts[n]["val"] = got.get(n, "?")
    return facts


def form(f, cxx):
    """A short text of one side's facts."""
    if f.get("def") == "fatal":
        return "header-fails"
    if not f.get("def"):
        return "undefined"
    parts = [k for k in ("pp", "expr", "ice", "arr", "init") if f.get(k)] + (["constexpr"] if cxx and f.get("cx") else [])
    if f.get("expr"):
        parts.append(f"ty={f.get('ty')}")
        parts.append(f"sz={f.get('sz')}")
        if f.get("ut"):
            parts.append(f"ut={f.get('ut')}")
        if f.get("sp"):
            parts.append(f"same-as={f.get('sp')}")
    return " ".join(parts) if parts else "defined (no use compiles)"


def cause(a, b):
    """The cause group of a difference between libniji's facts `a` and the reference's `b`."""
    if a.get("def") == "fatal" or b.get("def") == "fatal":
        return "header fails on one side"
    if bool(a.get("def")) != bool(b.get("def")):
        return "defined on one side only"
    if not a.get("def"):
        return None
    if a.get("expr") != b.get("expr"):
        return "expression on one side only"
    if a.get("expr") and b.get("expr") and any(a.get(k) != b.get(k) for k in ("ice", "arr", "init", "cx")):
        return "constant on one side, run-time on the other"
    if a.get("pp") != b.get("pp"):
        return "#if usability differs"
    if (a.get("ty"), a.get("sz")) != (b.get("ty"), b.get("sz")):
        return "type differs"
    if (a.get("ut"), a.get("sp")) != (b.get("ut"), b.get("sp")):
        return "enumeration differs"
    va, vb = a.get("val"), b.get("val")  # compared only where both sides ran (--values with a libniji to link)
    if va and vb and "?" not in (va, vb) and va != vb:
        return "value differs"
    return None


def run(inc, work, jobs, values=False, niji_cc=None, niji_cxx=None, only=None, macros_tsv=None, skip=()):
    """(inventory, results {(name, cid): (niji facts, reference facts)}, problems)."""
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    kern = os.path.join(work, "kernel-uapi")
    os.makedirs(kern, exist_ok=True)
    for d in ("linux", "asm-generic", "asm"):
        for base_dir in ("/usr/include", "/usr/include/x86_64-linux-gnu"):
            src = os.path.join(base_dir, d)
            if os.path.isdir(src) and not os.path.lexists(os.path.join(kern, d)):
                os.symlink(src, os.path.join(kern, d))
    inv = inventory(inc, macros_tsv, skip)
    if only:
        inv = [x for x in inv if x[1] in only]
    by_header = {}
    for n, h, _ in inv:
        by_header.setdefault(h, []).append(n)
    sides = [Side("niji", inc, gccinc, kern, niji_cc, niji_cxx), Side("ref", None, gccinc, None)]
    tasks = [(s, h, c, ns) for h, ns in sorted(by_header.items()) for c in CONFIGS for s in sides]
    out = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
        futs = {ex.submit(probe_one, s, h, c, ns, work, values): (s.name, h, c[0]) for s, h, c, ns in tasks}
        for fut in concurrent.futures.as_completed(futs):
            sname, h, cid = futs[fut]
            for n, f in fut.result().items():
                out.setdefault((n, h, cid), {})[sname] = f
    return inv, out


def report(inv, out, emit=print):
    """Prints the FORM-DIFF and FORM-RUNTIME lines and the summary; returns (pairs matching everywhere, differing,
    {cause: rows})."""
    diffs, runtime = {}, {}
    for n, h, _ in inv:
        for cid, lang, _ in CONFIGS:
            a, b = out[(n, h, cid)]["niji"], out[(n, h, cid)]["ref"]
            cxx = lang == "c++"
            # A run-time expression of the reference: an expression that is no constant, of an integer type (a
            # floating or pointer macro is never an integer constant expression in C, so it is not counted).
            if b.get("expr") and not b.get("ice") and not b.get("arr") and b.get("ty") in INT_TYPES:
                runtime.setdefault((n, h), []).append(cid)
            c = cause(a, b)
            if c:
                diffs.setdefault(c, []).append((n, h, cid, form(a, cxx), form(b, cxx), a.get("val", "-"), b.get("val", "-")))
    bad = {(d[0], d[1]) for rows in diffs.values() for d in rows}
    for c in sorted(diffs):
        for n, h, cid, fa, fb, va, vb in sorted(diffs[c]):
            emit(f"FORM-DIFF [{c}] {n} <{h}> {cid}: libniji {fa} val={va} | reference {fb} val={vb}")
    for n, h in sorted(runtime):
        a_ok = all(cause(out[(n, h, cid)]["niji"], out[(n, h, cid)]["ref"]) is None for cid, _, _ in CONFIGS)
        emit(f"FORM-RUNTIME {n} <{h}>: the reference's is a run-time expression in {','.join(runtime[(n, h)])}; "
             f"libniji {'matches' if a_ok else 'differs'}")
    names = {n for n, _, _ in inv}
    emit(f"form: {len(names)} macros, {len(inv)} (macro, header) pairs x {len(CONFIGS)} configurations: "
         f"{len(inv) - len(bad)} pairs match in every configuration, {len(bad)} pairs ({len({n for n, _ in bad})} macros) "
         f"differ in at least one ({sum(len(v) for v in diffs.values())} rows; "
         + ", ".join(f"{c}: {len(v)}" for c, v in sorted(diffs.items())) + f"); {' and '.join(FAILING)} fail, the rest is report mode")
    return len(inv) - len(bad), len(bad), diffs


def reference_compiles(work):
    src = os.path.join(work, "ref.c")
    with open(src, "w") as f:
        f.write("#include <stdio.h>\nint niji_ref_tu;\n")
    return subprocess.run(["gcc", "-fsyntax-only", src], capture_output=True).returncode == 0


def units(ctx):
    return []


# The probe's own control (a FAIL, not report mode): a header of the probe's own with one macro of each form, and the
# facts the probe must find for it. A probe that stops telling a constant from a run-time call reports no difference
# at all, so the report alone would not show it.
CONTROL_H = ("#ifndef NIJI_FORM_CONTROL_H\n#define NIJI_FORM_CONTROL_H\n#ifdef __cplusplus\nextern \"C\" {\n#endif\n"
             "long niji_form_rt (int) __attribute__ ((__const__));\n#ifdef __cplusplus\n}\n#endif\nenum { NIJI_E0 = 3 };\nenum { NIJI_E1 = -1, NIJI_E2 = 4 };\n"
             "#define NIJI_EA NIJI_E1\n#define NIJI_EB NIJI_E2\n#define NIJI_K 5\n#define NIJI_RT (niji_form_rt (1))\n#define NIJI_EN NIJI_E0\n#define NIJI_CH 'x'\n"
             "#define NIJI_F 1.5f\n#define NIJI_CAST ((unsigned long) 7)\n#define NIJI_UNDECL niji_form_nowhere\n"
             "#define NIJI_CALL (niji_form_undeclared (1))\n#endif\n")
CONTROL = {  # name: ({fact: value} in C, the same in C++); ty and sz from the object unit
    "NIJI_K": ({"pp": True, "expr": True, "ice": True, "arr": True, "init": True, "ty": "int", "sz": 4},
               {"pp": True, "ice": True, "arr": True, "cx": True, "ty": "int"}),
    "NIJI_RT": ({"pp": False, "expr": True, "ice": False, "arr": False, "init": False, "ty": "long", "sz": 8},
                {"expr": True, "ice": False, "arr": False, "cx": False, "ty": "long"}),
    "NIJI_EN": ({"ice": True, "ty": "int"}, {"ice": True, "cx": True, "ty": "enum", "ut": "unsigned int", "sp": "NIJI_EB:False"}),
    # Two enumerators of one enumeration with a negative member: its underlying type is signed, and one type.
    "NIJI_EA": ({"ty": "int"}, {"ty": "enum", "ut": "int"}),
    "NIJI_EB": ({"ty": "int"}, {"ty": "enum", "ut": "int", "sp": "NIJI_EA:True"}),
    "NIJI_CH": ({"ty": "int", "sz": 4}, {"ty": "char", "sz": 1}),
    "NIJI_F": ({"pp": False, "expr": True, "ice": False, "arr": False, "init": True, "ty": "float"}, {"cx": True, "ty": "float"}),
    "NIJI_CAST": ({"pp": False, "ice": True, "ty": "unsigned long"}, {"pp": False, "cx": True, "ty": "unsigned long"}),
    "NIJI_UNDECL": ({"def": True, "expr": False}, {"def": True, "expr": False}),
    "NIJI_CALL": ({"def": True, "expr": False}, {"def": True, "expr": False}),  # C: no implicit declaration (gcc 14)
    "NIJI_NOT_DEFINED": ({"def": False}, {"def": False}),
}


def control(work, gccinc):
    """FAIL lines for each fact of CONTROL the probe gets wrong."""
    d = os.path.join(work, "control")
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "niji_form_control.h"), "w") as f:
        f.write(CONTROL_H)
    side = Side("control", d, gccinc, None)
    out = []
    for cfg in (CONFIGS[1], CONFIGS[-1]):
        got = probe_one(side, "niji_form_control.h", cfg, sorted(CONTROL), work, False)
        for n, want in sorted(CONTROL.items()):
            for k, v in want[cfg[1] == "c++"].items():
                if got[n].get(k, False if k == "def" else None) != v:
                    out.append(f"form control {n} {cfg[0]}: {k} is {got[n].get(k)!r}, not {v!r}")
        c = cause(got["NIJI_K"], got["NIJI_RT"])
        if c != "constant on one side, run-time on the other":
            out.append(f"form control cause {cfg[0]}: a constant against a run-time call is {c!r}")
    return out


def problems(ctx):
    """header_check.py entry: FAIL lines from the probe's own control only; the comparison is report mode (NOTE lines)."""
    if os.environ.get("NIJI_FORM_PROBE") == "0":
        print("NOTE form: skipped (NIJI_FORM_PROBE=0)")
        return []
    inc = getattr(ctx, "inc", os.path.join(ROOT, "libc-include"))
    work = tempfile.mkdtemp(prefix="form.", dir=getattr(ctx, "work", None))
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    fails = control(work, gccinc)
    if not reference_compiles(work):
        print("NOTE form: report skipped, the build host's own C library headers do not compile (no reference)")
        return fails
    jobs = int(os.environ.get("NIJI_FORM_JOBS", min(16, os.cpu_count() or 1)))
    skip = {x for _, mod in ctx.mods for x in getattr(mod, "must_fail", ())} if hasattr(ctx, "mods") else must_fail_headers()
    inv, out = run(inc, work, jobs, macros_tsv=ctx.macros, skip=skip)
    _, _, diffs = report(inv, out, emit=lambda s: print("NOTE " + s))
    return fails + failing(diffs) + enforced(diffs)


# The causes that fail the run (HEADERS-CXX-ENUM-TYPES): a macro's type in C++ (an enumerator's enumeration, a
# character constant's `char`) and which enumeration it belongs to. The other causes stay report mode until their
# follow-ups land, except for the ENFORCED pairs below.
FAILING = ("type differs", "enumeration differs")

# (macro, defining header) pairs whose form must match the reference (a FAIL, not report mode): the audit's groups A1
# (PTHREAD_STACK_MIN run-time where the reference's is), B1 (a macro naming a function the header declares), B2
# (sys/vlimit.h's int INFINITY) and B3 (MS_NOUSER a file-scope array size, C configurations: its C++ type is group C1).
# Every other group stays report mode. Value: the configurations enforced, None for all.
C_CONFIGS = frozenset(cid for cid, lang, _ in CONFIGS if lang == "c")
ENFORCED = {("PTHREAD_STACK_MIN", "pthread.h"): None, ("PTHREAD_STACK_MIN", "dirent.h"): None,
            ("PTHREAD_STACK_MIN", "limits.h"): None, ("basename", "libgen.h"): None, ("res_init", "resolv.h"): None,
            ("res_ninit", "resolv.h"): None, ("res_nclose", "resolv.h"): None,
            ("b64_ntop", "resolv.h"): None, ("b64_pton", "resolv.h"): None,("INFINITY", "sys/vlimit.h"): None,
            ("MS_NOUSER", "sys/mount.h"): C_CONFIGS}


def is_enforced(n, h, cid):
    """Whether (macro, header) in configuration `cid` is an ENFORCED pair."""
    return (n, h) in ENFORCED and (ENFORCED[(n, h)] is None or cid in ENFORCED[(n, h)])


def failing(diffs):
    """FAIL lines of the FAILING rows, except the rows of an ENFORCED pair (`enforced` reports those, once)."""
    out = []
    for c in FAILING:
        for n, h, cid, fa, fb, _, _ in sorted(diffs.get(c, [])):
            if is_enforced(n, h, cid):
                continue
            out.append(f"form [{c}] {n} <{h}> {cid}: libniji {fa} | reference {fb}")
    return out


def enforced(diffs):
    """FAIL lines (without the `FAIL ` prefix, which the caller prints) for the FORM-DIFF rows of the ENFORCED pairs,
    in their enforced configurations."""
    return [f"form {n} <{h}> {cid}: [{c}] libniji {fa} | reference {fb}"
            for c, rows in sorted(diffs.items()) for n, h, cid, fa, fb, _, _ in sorted(rows) if is_enforced(n, h, cid)]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--work", default="")
    ap.add_argument("--jobs", type=int, default=min(16, os.cpu_count() or 1))
    ap.add_argument("--tsv", default="", help="every fact, one row per (macro, configuration)")
    ap.add_argument("--diff", default="", help="the FORM lines, also printed")
    ap.add_argument("--values", action="store_true", help="also run each unit for the values (libniji needs --niji-cc)")
    ap.add_argument("--niji-cc", default="")
    ap.add_argument("--niji-cxx", default="")
    ap.add_argument("--headers", default="")
    args = ap.parse_args()
    work = args.work or tempfile.mkdtemp(prefix="form.")
    os.makedirs(work, exist_ok=True)
    macros = {}
    with open(os.path.join(ROOT, "spec/abi/macros.tsv")) as f:
        head = f.readline().rstrip("\n").split("\t")
        for line in f:
            r = dict(zip(head, line.rstrip("\n").split("\t")))
            macros[r["name"]] = r
    only = set(args.headers.split(",")) if args.headers else None
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    fails = control(work, gccinc)
    for x in fails:
        print(f"FAIL {x}")
    inv, out = run(os.path.abspath(args.include), work, args.jobs, args.values, args.niji_cc or None, args.niji_cxx or None,
                   only, macros, must_fail_headers())
    lines = []
    _, _, diffs = report(inv, out, emit=lambda s: (print(s), lines.append(s)))
    for x in failing(diffs) + enforced(diffs):
        print(f"FAIL {x}")
        fails.append(x)
    if args.diff:
        with open(args.diff, "w") as f:
            f.write("\n".join(lines) + "\n")
    if args.tsv:
        with open(args.tsv, "w") as f:
            f.write("name\theader\tdefined_in\tconfig\tside\t" + "\t".join(FACTS) + "\tty\tsz\tval\n")
            for n, h, hs in inv:
                for cid, _, _ in CONFIGS:
                    for s in ("niji", "ref"):
                        fa = out[(n, h, cid)][s]
                        f.write("\t".join([n, h, ",".join(hs), cid, s] + [str(fa.get(k, "")) for k in FACTS + ("ty", "sz", "val")]) + "\n")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
