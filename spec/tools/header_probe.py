#!/usr/bin/env python3
"""Probe which names each public header makes visible under each feature-test configuration.

    python3 spec/tools/header_probe.py [--jobs N] [--headers a.h,b.h] [--out DIR] [--include DIR]

`--include DIR` probes libniji's headers (P2 of plan/headers.md section 3): every compile gets `-nostdinc -isystem DIR`.

Run on the reference system (Appendix C.0), like abi_extract.py. Writes `ftm-configs.tsv` (the
configurations of plan/headers.md section 2.3) and `visibility.tsv` (Appendix C.12) into spec/abi/
(or --out). `--headers` limits the run to some headers, for trying the probe; a table for the spec
is always written by a full run.

Clean room (plan/headers.md section 2.2, ruling R-H0). The probe compiles translation units of its
own that include one public header and then test candidate names. It reads exactly two things back
from the compiler: the exit status, and the LINE NUMBERS of errors located in the probe's own file
(each test sits alone on its own line, so "line L has an error" is the result of test L). It never
reads diagnostic message text, `-E` or `-dM` output, or any header text.

Candidates are the names the existing tables already hold: prototypes.tsv functions, constants.tsv
macros, versions.tsv data objects, layouts.tsv types and tags, and the type and tag names spelled in
the prototypes.tsv declarations and layouts.tsv member types, and the identifiers the functional spec
writes in backticks. Per (header, configuration):

* macro       `#ifdef N` / `#error` / `#endif` errors (before any `#undef`);
* then every candidate is `#undef`ed and the ordinary name space is tested:
  - declared  C `__typeof__(N) *p;`, C++ `using ::N;` compiles;
  - typedef   C `N *p;` declares (C++: `typedef N t;` compiles and `struct`/`union`/`enum N *`
              all fail, so a class or enum name is not a typedef);
  - function  C: `(0, N)` has the type `__typeof__(N) *` (a function designator decays);
              C++: an ordinary name that is neither an enumerator nor a decay-stable object
              (an overload set lands here too);
  - enumerator `enum { e = (N) };` compiles (an integer constant expression);
  - object    a declared ordinary name that is none of the above;
* struct, union, enum  `sizeof(struct N)` (and `union`, `enum`) compiles: a complete tag.

A header whose bare `#include` fails in a configuration gets one row of kind `include-error`.
`configs` is `*` (every configuration), the ids, or `!` and the ids it is NOT visible in, whichever is
shorter. Rows of header `-` are what the empty translation unit already shows (compiler predefines and the
implicitly included stdc-predef.h); header rows leave those (name, kind, configuration) out.
"""
import argparse, csv, glob, importlib.util, os, re, shutil, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

# The family fragments (header_probe.d/*.py) import the shared machinery from this module by name, also when it runs
# as a script.
sys.modules.setdefault("header_probe", sys.modules[__name__])

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
ABI = os.path.join(ROOT, "spec", "abi")
INVENTORY = os.path.join(ROOT, "plan", "headers-inventory.tsv")

# id, language, flags, what it probes (plan/headers.md section 2.3). The order is the table order.
CONFIGS = [
    ("c89", "c", "-std=c89", "ISO C90, __STRICT_ANSI__"),
    ("c99", "c", "-std=c99", "ISO C99, __STRICT_ANSI__"),
    ("c11", "c", "-std=c11", "ISO C11, __STRICT_ANSI__"),
    ("c17", "c", "-std=c17", "ISO C17, __STRICT_ANSI__"),
    ("c2x", "c", "-std=c2x", "ISO C2X, __STRICT_ANSI__"),
    ("gnu17", "c", "-std=gnu17", "compiler default mode, no feature-test macro"),
    ("gnu2x", "c", "-std=gnu2x", "GNU C2X mode, no feature-test macro"),
    ("posix1", "c", "-std=c17 -D_POSIX_C_SOURCE=1", "POSIX.1-1990"),
    ("posix2", "c", "-std=c17 -D_POSIX_C_SOURCE=2", "POSIX.2-1992"),
    ("posix199309", "c", "-std=c17 -D_POSIX_C_SOURCE=199309L", "POSIX.1b"),
    ("posix199506", "c", "-std=c17 -D_POSIX_C_SOURCE=199506L", "POSIX.1c"),
    ("posix200112", "c", "-std=c17 -D_POSIX_C_SOURCE=200112L", "POSIX.1-2001"),
    ("posix200809", "c", "-std=c17 -D_POSIX_C_SOURCE=200809L", "POSIX.1-2008"),
    ("xopen", "c", "-std=c17 -D_XOPEN_SOURCE", "_XOPEN_SOURCE defined empty"),
    ("xopen500", "c", "-std=c17 -D_XOPEN_SOURCE=500", "SUSv2"),
    ("xopen600", "c", "-std=c17 -D_XOPEN_SOURCE=600", "SUSv3"),
    ("xopen700", "c", "-std=c17 -D_XOPEN_SOURCE=700", "SUSv4"),
    ("xopenext", "c", "-std=c17 -D_XOPEN_SOURCE -D_XOPEN_SOURCE_EXTENDED", "X/Open UNIX extended"),
    ("isoc99", "c", "-std=c89 -D_ISOC99_SOURCE", "C99 names in C90 mode"),
    ("isoc11", "c", "-std=c99 -D_ISOC11_SOURCE", "C11 names in C99 mode"),
    ("isoc2x", "c", "-std=c17 -D_ISOC2X_SOURCE", "C2X names in C17 mode"),
    ("default", "c", "-std=c17 -D_DEFAULT_SOURCE", "_DEFAULT_SOURCE under strict C17"),
    ("bsd", "c", "-std=c17 -D_BSD_SOURCE", "_BSD_SOURCE under strict C17"),
    ("svid", "c", "-std=c17 -D_SVID_SOURCE", "_SVID_SOURCE under strict C17"),
    ("gnu", "c", "-std=gnu17 -D_GNU_SOURCE", "_GNU_SOURCE"),
    ("lfs64", "c", "-std=gnu17 -D_LARGEFILE64_SOURCE", "the *64 names"),
    ("fob64", "c", "-std=gnu17 -D_FILE_OFFSET_BITS=64", "64-bit file offsets"),
    ("time64", "c", "-std=gnu17 -D_FILE_OFFSET_BITS=64 -D_TIME_BITS=64", "64-bit time"),
    ("time64only", "c", "-std=gnu17 -D_TIME_BITS=64", "_TIME_BITS=64 without _FILE_OFFSET_BITS=64"),
    ("dynstack", "c", "-std=gnu17 -D_DYNAMIC_STACK_SIZE_SOURCE", "run-time stack-size macros"),
    ("reentrant", "c", "-std=gnu17 -D_REENTRANT", "_REENTRANT"),
    ("fortify1", "c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=1", "fortify level 1 at -O2"),
    ("fortify2", "c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=2", "fortify level 2 at -O2"),
    ("fortify3", "c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=3", "fortify level 3 at -O2"),
    ("cxx98", "c++", "-std=c++98", "C++98 (g++ predefines _GNU_SOURCE)"),
    ("cxx11", "c++", "-std=c++11", "C++11"),
    ("cxx14", "c++", "-std=c++14", "C++14"),
    ("cxx17", "c++", "-std=c++17", "C++17"),
    ("cxx20", "c++", "-std=c++20", "C++20"),
    ("cxx23", "c++", "-std=c++23", "C++23"),
]
COMPILER = {"c": "gcc", "c++": "g++"}
BASE = ["-fsyntax-only", "-w", "-fno-builtin", "-fmax-errors=0", "-fno-diagnostics-show-caret", "-fdiagnostics-plain-output",
        "-fno-diagnostics-show-option", "-fdiagnostics-color=never"]
# Per-language flags after BASE. `--raw-cxx` puts `-nostdinc++` here for C++: the C library's headers alone, as the
# C++ library's own `#include_next` reaches them and its configure tests see them, without the C++ library's
# `<math.h>`/`<stdlib.h>`/... wrappers in front (which add and undefine names of their own).
LANG_FLAGS = {"c": [], "c++": []}
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
KINDS = ("macro", "typedef", "function", "object", "enumerator", "struct", "union", "enum", "include-error")


def tsv(name):
    with open(os.path.join(ABI, name), newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def c_integer_names():
    """The names C 7.20 (<stdint.h>) and C 7.8.1 (<inttypes.h>) give the integer types of each width: the
    exact-width, minimum-width and fastest types (7.20.1.1-3), the integer-constant macros (7.20.4) and the
    format macros (7.8.1). No other table spells them all (ruling #6047)."""
    out = ["INTMAX_C", "UINTMAX_C"]
    kinds = ["", "LEAST", "FAST"]
    for n in (8, 16, 32, 64):
        out += [f"int{n}_t", f"uint{n}_t", f"int_least{n}_t", f"uint_least{n}_t", f"int_fast{n}_t", f"uint_fast{n}_t",
                f"INT{n}_C", f"UINT{n}_C"]
    sizes = [f"{k}{n}" for k in kinds for n in (8, 16, 32, 64)] + ["MAX", "PTR"]
    # C23 7.8.1 adds the binary conversions `b` and `B` (PRIb*, PRIB*, SCNb*).
    out += [f"PRI{c}{s}" for c in "diouxXbB" for s in sizes]
    out += [f"SCN{c}{s}" for c in "diouxXb" if c != "X" for s in sizes]
    return out


def guard_candidates(header):
    """The include-guard names a program may test for `header` (HEADERS-STAGE2-GAPS-C; gmp's <gmp.h> declares its
    `FILE *` functions only when it sees a stdio guard): the conventional shapes, generated from the header's path
    (`sys/types.h` -> `_SYS_TYPES_H`, `__SYS_TYPES_H`, ...). A candidate is only a name to test: which headers define
    it, and its value, are probed. xtask/src/headers.rs `guard_candidates` spells the same list."""
    p = re.sub(r"[^A-Za-z0-9]", "_", header[:-2] if header.endswith(".h") else header).upper()
    return [f"_{p}_H", f"__{p}_H", f"_{p}_H_", f"__{p}_H__", f"{p}_H", f"_{p}_H_INCLUDED", f"_{p}_INCLUDED"]


FAMILIES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "header_probe.d")
_families = None


def families():
    """The candidate fragments header_probe.d/<family>.py, (family, module) in file-name order, loaded once. A header
    story adds its names as a new fragment (header_probe.d/README.md), so two stories never edit the same lines."""
    global _families
    if _families is None:
        _families = []
        for path in sorted(glob.glob(os.path.join(FAMILIES, "*.py"))):
            name = os.path.basename(path)[:-3]
            spec = importlib.util.spec_from_file_location(f"header_probe_d_{name.replace('-', '_')}", path)
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            if not callable(getattr(mod, "candidates", None)):
                raise SystemExit(f"{path}: no candidates() function")
            _families.append((name, mod))
    return _families


def family(name):
    """The fragment module of one family (header_checks/s07.py reads stage2-gaps-c's `_PATH_` names)."""
    return dict(families())[name]


def candidates():
    """(ordinary names, tag names) from the existing spec/abi tables and the family fragments."""
    names, tags = set(), set()
    names.update(r["name"] for r in tsv("prototypes.tsv"))
    names.update(r["macro"] for r in tsv("constants.tsv"))
    names.update(r["symbol"] for r in tsv("versions.tsv") if r["type"] == "OBJECT")
    spelled = [r["declaration"] for r in tsv("prototypes.tsv")]
    for r in tsv("layouts.tsv"):
        spelled.append(r["member_type"])
        t = r["type"]
        m = re.match(r"^(struct|union|enum)\s+(\w+)$", t)
        if m:
            tags.add(m.group(2))
        elif IDENT.match(t):
            names.add(t)
    for text in spelled:
        text = text or ""
        for m in re.finditer(r"\b(struct|union|enum)\s+([A-Za-z_]\w*)", text):
            tags.add(m.group(2))
        cleaned = re.sub(r"\b(struct|union|enum)\s+[A-Za-z_]\w*", " ", text)
        names.update(re.findall(r"[A-Za-z_]\w*", cleaned))
    # Identifiers the functional spec writes in backticks (`errno`, `HUGE_VAL`, `pid_t`, `S_ISDIR()`): names
    # the integer and prototype tables cannot hold.
    with open(os.path.join(ROOT, "glibc-functional-spec.md"), encoding="utf-8") as f:
        names.update(re.findall(r"`([A-Za-z_][A-Za-z0-9_]*)(?:\(\))?`", f.read()))
    names.update(c_integer_names())
    # The candidates of each header family (header_probe.d/<family>.py, `candidates()` -> (names, tags)).
    for _, mod in families():
        fam_names, fam_tags = mod.candidates()
        names.update(fam_names)
        tags.update(fam_tags)
    # The public type name of each reserved one (`__blksize_t` spells struct stat's member, `blksize_t` is the
    # name XBD <sys/types.h> gives programs): a header that shows the reserved type may show its public name.
    names.update(n[2:] for n in list(names) if n.startswith("__") and n.endswith("_t") and not n.startswith("___"))
    # The enumerators and enumeration tags of enumerators.tsv (HEADERS-ENUM-FOLLOWUPS): `_REG_NOMATCH`, the
    # enumerator the macro REG_NOMATCH names, its typedef `reg_errcode_t`, and `ns_t_a` of `enum __ns_type` are
    # spelled in no other table.
    for r in tsv("enumerators.tsv"):
        if r["tag"].startswith("enum "):
            tags.add(r["tag"][len("enum "):])
        elif r["tag"].startswith("typedef "):
            names.add(r["tag"][len("typedef "):])
        names.add(r["name"])
    # The reserved names headers-gen emits by policy (headers/reserved.tsv `emit`, rulings R-H2, R-H3): the generator
    # declares them only where this table shows them, so they are probed whether or not another table spells them
    # (slice 20: `__locale_t`, which the C++ library's locale support names, appears in no prototype).
    with open(os.path.join(ROOT, "headers", "reserved.tsv"), newline="") as f:
        names.update(r["name"] for r in csv.DictReader((l for l in f if not l.startswith("#")), delimiter="\t")
                     if r["policy"] in ("emit", "emit-once"))
    names = {n for n in names if IDENT.match(n) and not n.startswith("__builtin")}
    tags = {n for n in tags if IDENT.match(n)}
    return sorted(names), sorted(tags)


def _compile(lang, flags, path):
    """(exit status, line numbers of errors located in `path`, count of errors located elsewhere, command)."""
    # Without macro-expansion tracking an error inside a macro's expansion is located at the line that uses the macro
    # where the compiler can do so; errors outside macros are located as before.
    cmd = [COMPILER[lang]] + flags.split() + BASE + LANG_FLAGS[lang] + ["-ftrack-macro-expansion=0", "-x", "c" if lang == "c" else "c++", path]
    p = subprocess.run(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    # Only the location prefix of error lines is matched; the message after it is never looked at.
    pat = re.compile(r"^(.+?):(\d+):\d+: (?:fatal )?error:")
    lines, elsewhere = set(), 0
    for raw in p.stderr.splitlines():
        m = pat.match(raw.decode("utf-8", "replace"))
        if m and m.group(1) == path:
            lines.add(int(m.group(2)))
        elif m:
            elsewhere += 1
    return p.returncode, lines, elsewhere, cmd


def _located(rc, lines, elsewhere):
    """Whether a compile's outcome is known line by line: it passed, or every error is on a line of the unit."""
    return rc == 0 or (lines and not elsewhere)


def error_lines(lang, flags, path):
    """(exit status, set of line numbers of errors located in `path`). Nothing else is read."""
    rc, lines, elsewhere, cmd = _compile(lang, flags, path)
    if not _located(rc, lines, elsewhere):
        raise RuntimeError(f"{os.path.basename(path)}: the unit failed outside its own test lines ({' '.join(cmd[:2])} ...)")
    return rc, lines


class Unit:
    """A probe translation unit: each test on its own line, remembered by line number."""

    def __init__(self, header):
        self.lines, self.tests = [], {}
        if header:
            self.lines.append(f"#include <{header}>")

    def raw(self, text):
        self.lines.append(text)

    def test(self, key, text):
        self.lines.append(text)
        self.tests[len(self.lines)] = key

    def _write(self, path, active):
        """Write the unit with only the test lines in `active` (the others blank, so line numbers do not move)."""
        with open(path, "w") as f:
            f.write("\n".join("" if i + 1 in self.tests and i + 1 not in active else t
                              for i, t in enumerate(self.lines)) + "\n")

    def run(self, lang, flags, path):
        """(exit status, keys of the tests that fail to compile)."""
        every = set(self.tests)
        self._write(path, every)
        rc, errs, elsewhere, cmd = _compile(lang, flags, path)
        if not _located(rc, errs, elsewhere):
            # An error located outside the unit (an object-like alias of a function-like macro, `GETLONG` for
            # `NS_GET32`, used as a value is reported at the alias's definition in the header, whatever the
            # tracking flag): halve the set of tests until each such failure is pinned to one test. A unit that
            # fails with no test at all is the header's own failure, as before.
            self._write(path, set())
            if not _located(*_compile(lang, flags, path)[:3]):
                raise RuntimeError(f"{os.path.basename(path)}: the unit failed outside its own test lines "
                                   f"({' '.join(cmd[:2])} ...)")
            errs = self._bisect(lang, flags, path, sorted(every))
            rc = 1
        return rc, {key for line, key in self.tests.items() if line in errs}

    def _bisect(self, lang, flags, path, active):
        """The lines of the tests in `active` that fail, each test of an unlocated failure found by halving."""
        self._write(path, set(active))
        rc, errs, elsewhere, _ = _compile(lang, flags, path)
        if _located(rc, errs, elsewhere):
            return errs
        if len(active) == 1:
            return set(active)
        half = len(active) // 2
        return self._bisect(lang, flags, path, active[:half]) | self._bisect(lang, flags, path, active[half:])


def keywords(lang, names, work):
    """Candidates the compiler itself reserves (keywords) in the widest mode of `lang`: excluded there."""
    u = Unit(None)
    for n in names:
        u.test(n, f"struct vis_kw_{n} {{ int {n}; }};")
    # -undef drops the compiler's predefined macros and -nostdinc the implicitly included stdc-predef.h, whose
    # macros (`_STDC_PREDEF_H`, `__STDC_ISO_10646__`) would otherwise fail this test and be taken for keywords.
    flags = "-std=gnu2x -undef -nostdinc" if lang == "c" else "-std=c++23 -undef -nostdinc"
    _, failed = u.run(lang, flags, os.path.join(work, f"kw.{lang}"))
    return failed


def probe(header, cfg, names, tags, work, macro_only=()):
    """{(name, kind)} visible after `#include <header>` (header None: the empty unit) in `cfg`.

    `macro_only` (the keywords of the language) get the `#ifdef` test alone: `#ifdef` takes any identifier, and a
    header can define one as a macro in an older mode (`true` and `false` from stdbool.h before C23)."""
    cid, lang, flags, _ = cfg
    tag = (header or "-").replace("/", "_")
    path = os.path.join(work, f"{tag}.{cid}.{'c' if lang == 'c' else 'cc'}")
    if header:
        rc = subprocess.run([COMPILER[lang]] + flags.split() + BASE + LANG_FLAGS[lang] + ["-x", "c" if lang == "c" else "c++", "-"],
                            input=f"#include <{header}>\n".encode(), stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL).returncode
        if rc != 0:
            return {("-", "include-error")}
    cxx = lang == "c++"
    # Pass A: macros, then declared ordinary names and complete tags.
    a = Unit(header)
    for n in list(names) + list(macro_only):
        a.raw(f"#ifdef {n}")
        a.test(("macro", n), "#error vis")
        a.raw("#endif")
    for n in names:
        a.raw(f"#undef {n}")
    for n in names:
        # Conflicts with any ordinary declaration of n (function, object, typedef, enumerator): the error
        # marks a declared name, so the many undeclared candidates cost no diagnostic at all.
        a.test(("decl", n), f"struct vis_g_{n}; extern {'' if cxx else 'struct '}vis_g_{n} {n};")
    for n in tags:
        for kw in ("struct", "union", "enum"):
            a.test((kw, n), f"void vis_{kw[0]}_{n}(void) {{ (void)sizeof({kw} {n}); }}")
    _, fa = a.run(lang, flags, path)
    out = {(n, "macro") for n in list(names) + list(macro_only) if ("macro", n) in fa}
    for n in tags:
        for kw in ("struct", "union", "enum"):
            if (kw, n) not in fa:
                out.add((n, kw))
    declared = [n for n in names if ("decl", n) in fa]
    if not declared:
        return out
    # Pass B: classify the declared names.
    b = Unit(header)
    for n in names:
        b.raw(f"#undef {n}")
    if cxx:
        b.raw("template<class A, class B> struct vis_same { enum { vis_v = 1 }; };")
        b.raw("template<class A> struct vis_same<A, A> { enum { vis_v = -1 }; };")
        b.raw("template<class T> T vis_decay(T);")
    for n in declared:
        if cxx:
            b.test(("type", n), f"void vis_t_{n}() {{ typedef {n} vis_x_{n}; }}")
            for kw in ("struct", "union", "enum"):
                b.test(("ptr-" + kw, n), f"void vis_p{kw[0]}_{n}() {{ {kw} {n} *vis_y_{n} = 0; (void)vis_y_{n}; }}")
            b.test(("obj", n), f"extern char vis_o_{n}[vis_same<__typeof__(vis_decay({n})), __typeof__(&{n})>::vis_v];")
        else:
            b.test(("type", n), f"void vis_t_{n}(void) {{ (void)sizeof({n} *); }}")
            b.test(("fn", n), f"extern char vis_f_{n}[__builtin_types_compatible_p(__typeof__((0, {n})), __typeof__(&{n})) ? 1 : -1];")
        b.test(("ice", n), f"enum {{ vis_e_{n} = ({n}) }};")
    _, fb = b.run(lang, flags, path)
    for n in declared:
        is_type = ("type", n) not in fb
        if cxx:
            if is_type:
                if all(("ptr-" + kw, n) in fb for kw in ("struct", "union", "enum")):
                    out.add((n, "typedef"))
                continue
            if ("ice", n) not in fb:
                out.add((n, "enumerator"))
            elif ("obj", n) not in fb:
                out.add((n, "object"))
            else:
                out.add((n, "function"))
        else:
            if is_type:
                out.add((n, "typedef"))
            elif ("fn", n) not in fb:
                out.add((n, "function"))
            elif ("ice", n) not in fb:
                out.add((n, "enumerator"))
            else:
                out.add((n, "object"))
    return out


def public_headers():
    with open(INVENTORY, newline="") as f:
        rows = list(csv.DictReader(f, delimiter="\t"))
    return sorted(r["header"] for r in rows if r["kind"] == "public" and r["header"].endswith(".h"))


def uapi_flags():
    """`-isystem` for the kernel UAPI directories (linux-libc-dev) that slice 9 headers include: $NIJI_UAPI (the tree
    `cargo xtask so` stages at target/niji/uapi) or, when unset, the build host's multiarch and plain include directories,
    placed after libniji's tree so the host's libc headers stay out of reach."""
    d = os.environ.get("NIJI_UAPI")
    if d:
        return ["-isystem", d]
    return ["-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"] if os.path.isdir("/usr/include/linux") else []

def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--headers", default="")
    ap.add_argument("--out", default=ABI)
    ap.add_argument("--names", default="", help="FILE: probe only the candidates it lists (whitespace-separated)")
    ap.add_argument("--include", default="", help="probe the headers under DIR (libniji's libc-include/) instead of the system's")
    ap.add_argument("--raw-cxx", action="store_true", help="C++ compiles get -nostdinc++ (the C headers without the C++ library's wrappers)")
    ap.add_argument("--configs", default="", help="IDS: probe only these configurations (comma-separated; a partial table, for sweeps)")
    args = ap.parse_args()
    global CONFIGS
    if args.raw_cxx:
        LANG_FLAGS["c++"].append("-nostdinc++")
    if args.configs:
        want = args.configs.split(",")
        unknown = [c for c in want if c not in {c[0] for c in CONFIGS}]
        if unknown:
            ap.error(f"unknown configurations {unknown}")
        CONFIGS = [c for c in CONFIGS if c[0] in want]
    if args.include:
        gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        BASE.extend(["-nostdinc", "-isystem", os.path.abspath(args.include), "-isystem", gccinc, *uapi_flags()])
    headers = args.headers.split(",") if args.headers else public_headers()
    names, tags = candidates()
    if args.names:
        # A name's rows depend on that name's units only, so probing a subset gives exactly the full run's rows
        # for those names (rows to splice into the committed table).
        with open(args.names) as f:
            only = set(f.read().split())
        names, tags = [n for n in names if n in only], [t for t in tags if t in only]
    work = tempfile.mkdtemp(prefix="header_probe.")
    try:
        langs = sorted({c[1] for c in CONFIGS})
        kw = {lang: keywords(lang, sorted(set(names) | set(tags)), work) for lang in langs}
        per_lang = {lang: ([n for n in names if n not in kw[lang]], [t for t in tags if t not in kw[lang]]) for lang in langs}
        jobs = [(h, c) for h in [None] + headers for c in CONFIGS]
        with ThreadPoolExecutor(max_workers=args.jobs) as ex:
            results = list(ex.map(lambda hc: probe(hc[0], hc[1], *per_lang[hc[1][1]], work, sorted(kw[hc[1][1]])), jobs))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    ids = [c[0] for c in CONFIGS]
    seen = {}
    for (h, c), res in zip(jobs, results):
        for name, kind in res:
            seen.setdefault((h or "-", name, kind), set()).add(c[0])
    base = {(n, k): cs for (h, n, k), cs in seen.items() if h == "-"}
    rows = []
    for (h, n, k), cs in seen.items():
        if h != "-":
            cs = cs - base.get((n, k), set())
        if cs:
            listed = ",".join(i for i in ids if i in cs)
            rest = "!" + ",".join(i for i in ids if i not in cs)
            rows.append((h, n, k, "*" if len(cs) == len(ids) else min(listed, rest, key=len)))
    rows.sort(key=lambda r: (r[0] != "-", r[0], r[1], KINDS.index(r[2])))
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "ftm-configs.tsv"), "w", newline="\n") as f:
        f.write("id\tlanguage\tflags\tprobes\n")
        for c in CONFIGS:
            f.write("\t".join(c) + "\n")
    with open(os.path.join(args.out, "visibility.tsv"), "w", newline="\n") as f:
        f.write("header\tname\tkind\tconfigs\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    excluded = {lang: len(kw[lang]) for lang in langs}
    print(f"headers {len(headers)}  configurations {len(CONFIGS)}  candidates {len(names)} names + {len(tags)} tags"
          f"  keywords excluded {excluded}  rows {len(rows)}")


if __name__ == "__main__":
    sys.exit(main())
