#!/usr/bin/env python3
"""Probe which symbol a compiled program references for each function, and each function's C++ exception
specification (plan/headers.md slice 4, Appendix C.13).

    python3 spec/tools/callsite_probe.py [--jobs N] [--names a,b] [--out DIR]

Run on the reference system (Appendix C.0), like header_probe.py, after it: the function list is
prototypes.tsv, and the header and configurations each function is probed in come from visibility.tsv
(the header where the name is a function in the most configurations; ties go to the first in sort
order). Writes `callsites.tsv` and `noexcept.tsv` into spec/abi/ (or --out). `--names` limits the run to
some functions, for trying the probe; a table for the spec is always written by a full run.

Clean room (plan/headers.md section 2.2, ruling R-H0). The probe compiles translation units of its own
that include one public header and then hold one test per line. It reads back only: the LINE NUMBERS of
errors located in its own file (a test that does not compile is dropped and recorded as `!error`), the
relocations of its own sections (`readelf -rW`), and the sizes and demangled names of its own symbols
(`nm`). It never reads diagnostic text, `-E` or `-dM` output, disassembly, or any header text.

callsites.tsv (`name header mode configs symbol`), one row per (function, mode, symbol):
* addr  `void *lp_a_F = (void *)&F;` at -O0 in its own data section, in every configuration of
        ftm-configs.tsv except the fortify ones (C and C++): the symbol the address relocation names.
* call  a call `F(args)` inside a function of its own text section, at -O2, in the fortify
        configurations, each also with `+gnu` (-D_GNU_SOURCE) and `+gnu64` (-D_GNU_SOURCE
        -D_FILE_OFFSET_BITS=64) added, so the GNU-only functions are called too. Every pointer argument is
        a distinct `extern char[8]` cast to the parameter type (a known-size buffer), every other argument
        is read from an opaque extern object (a run-time value); variadic functions get no variadic
        arguments; a result is stored through a volatile pointer so the call stays.
        The symbols are the call's relocation targets other than the probe's own `lp_` objects, sorted
        and comma-joined; `-` when the call references none (expanded inline).
* macro the same as call for the C.9 macros that reach a `_chk` export (FD_SET, FD_CLR, FD_ISSET).
`symbol` is `!error` when the test does not compile in a configuration where the name is a function,
and `local` when the relocation names a symbol the probe's object defines itself. `configs` is `*` (every
configuration of the mode), the ids, or `!` and the ids it lacks, whichever is shorter (visibility.tsv's
form, over the mode's own configuration list).

noexcept.tsv (`name header cxx17 variadic cxx98`), one row per prototypes.tsv function:
* cxx17, variadic  at -std=c++17 an overload set `R(*)(A...)`, `R(*)(A...) noexcept`, `R(*)(A..., ...)`,
        `R(*)(A..., ...) noexcept`, each returning a different array size, is called on `&F`; the size
        of `char lp_n_F[sizeof(lp_k(&F))]` gives `noexcept`/`none` and `yes`/`no`. `error` when `&F`
        does not resolve (a C++ overload set), `-` when F is no function in C++17.
* cxx98  at -std=c++98 the type of `&F` is read back from the demangled name of
        `void lp_t_F(__typeof__(&F))`; F is then redeclared `R (F)(A...)` with that type (no linkage
        specification, so it keeps the header's; `(F)` so a function-like macro F does not expand), once
        plain and once with `throw()`, each in its own unit compiled with the headers as ordinary (-I, not
        system) include directories, where a different exception specifier is an error (a unit that fails
        outside its own lines is split into one unit per name; a tag hidden by a same-named function,
        demangled `stat*`, is written `struct stat*`). `throw()` or `none` is
        the one redeclaration that compiles; `unknown` when both do (the redeclaration
        check cannot tell), `error` when neither does or the type cannot be read back, `-` when F is no function in C++98.
"""
import argparse, csv, os, re, shutil, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
ABI = os.path.join(ROOT, "spec", "abi")
COMPILER = {"c": "gcc", "c++": "g++"}
DIAG = ["-fmax-errors=0", "-fno-diagnostics-show-caret", "-fdiagnostics-plain-output", "-fno-diagnostics-show-option",
        "-fdiagnostics-color=never", "-w"]
OBJ = ["-c", "-fno-asynchronous-unwind-tables", "-fno-stack-protector", "-fno-ipa-icf", "-fcf-protection=none"]
OVERLAYS = [("", ""), ("+gnu", "-D_GNU_SOURCE"), ("+gnu64", "-D_GNU_SOURCE -D_FILE_OFFSET_BITS=64")]
# The C.9 macros that expand to a `_chk` call: (macro, header, expression).
MACROS = [
    ("FD_CLR", "sys/select.h", "FD_CLR(*(int *)lp_v, (fd_set *)lp_b0)"),
    ("FD_ISSET", "sys/select.h", "*(volatile int *)lp_out = FD_ISSET(*(int *)lp_v, (fd_set *)lp_b0)"),
    ("FD_SET", "sys/select.h", "FD_SET(*(int *)lp_v, (fd_set *)lp_b0)"),
]
NBUF = 16
ERR = re.compile(r"^(.+?):(\d+):\d+: (?:fatal )?error:")
RELOC_SECTION = re.compile(r"^Relocation section '\.rela(\S+)'")
RELOC = re.compile(r"^\s*[0-9a-f]+\s+[0-9a-f]+\s+R_X86_64_\w+\s+[0-9a-f]+\s+(\S+)\s*[+-]\s*[0-9a-f]+\s*$")


def tsv(name):
    with open(os.path.join(ABI, name), newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def configs():
    """(all ftm ids in order, {id: (language, flags)})."""
    rows = tsv("ftm-configs.tsv")
    return [r["id"] for r in rows], {r["id"]: (r["language"], r["flags"]) for r in rows}


def decode(form, ids):
    if form == "*":
        return set(ids)
    if form.startswith("!"):
        return set(ids) - set(form[1:].split(","))
    return set(form.split(","))


def encode(cs, ids):
    """visibility.tsv's spelling of a configuration set over `ids` (`None` when empty)."""
    if not cs:
        return None
    if len(cs) == len(ids):
        return "*"
    listed = ",".join(i for i in ids if i in cs)
    rest = "!" + ",".join(i for i in ids if i not in cs)
    return rest if len(rest) < len(listed) else listed


def functions(all_ids, names_filter, proto_header):
    """{name: (header, {config ids where it is a function there})} from visibility.tsv. The header is the one
    where the name is a function in the most configurations; on a tie prototypes.tsv's header, then the header
    that shows the fewest names (the most specific one), then the first in sort order."""
    per, size = {}, {}
    for r in tsv("visibility.tsv"):
        size[r["header"]] = size.get(r["header"], 0) + 1
        if r["kind"] == "function" and r["header"] != "-":
            per.setdefault(r["name"], []).append((r["header"], decode(r["configs"], all_ids)))
    out = {}
    for name, hs in per.items():
        if names_filter and name not in names_filter:
            continue
        h, cs = min(hs, key=lambda x: (-len(x[1]), x[0] != proto_header.get(name), size[x[0]], x[0]))
        out[name] = (h, cs)
    return out


def compile_unit(lang, flags, path, lines, tests, extra, obj=None):
    """Compile, dropping tests whose line has an error until the unit compiles. Returns (kept keys, failed keys)."""
    failed = set()
    while True:
        text, at = [], {}
        for kind, payload in lines:
            if kind == "test":
                key, code = payload
                if key in failed:
                    continue
                at[len(text) + 1] = key
                text.append(code)
            else:
                text.append(payload)
        with open(path, "w") as f:
            f.write("\n".join(text) + "\n")
        cmd = [COMPILER[lang]] + flags.split() + extra + DIAG + ["-x", "c" if lang == "c" else "c++", path]
        cmd += ["-o", obj] if obj else ["-fsyntax-only"]
        p = subprocess.run(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        if p.returncode == 0:
            return [k for k in at.values()], failed
        mine, elsewhere = set(), 0
        for raw in p.stderr.splitlines():
            m = ERR.match(raw.decode("utf-8", "replace"))
            if m and m.group(1) == path:
                mine.add(int(m.group(2)))
            elif m:
                elsewhere += 1
        new = {at[n] for n in mine if n in at}
        if elsewhere or not new:
            raise RuntimeError(f"{os.path.basename(path)}: the unit failed outside its own test lines")
        failed |= new


def relocs(obj, prefix):
    """{section suffix after `prefix`: sorted symbol list} from the probe object's own relocations."""
    defined = set(subprocess.run(["nm", "--defined-only", "-j", obj], capture_output=True, text=True, check=True).stdout.split())
    out, cur = {}, None
    for line in subprocess.run(["readelf", "-rW", obj], capture_output=True, text=True, check=True).stdout.splitlines():
        m = RELOC_SECTION.match(line)
        if m:
            cur = m.group(1)[len(prefix):] if m.group(1).startswith(prefix) else None
            if cur is not None:
                out.setdefault(cur, set())
            continue
        m = RELOC.match(line)
        if m and cur is not None:
            sym = m.group(1)
            if sym.startswith("lp_"):
                continue
            out[cur].add("local" if sym in defined or sym.startswith(".") else sym)
    return {k: sorted(v) for k, v in out.items()}


def split_params(decl, name):
    """(return type, [parameter types], variadic) of a prototypes.tsv declaration."""
    m = re.search(r"(?<![\w])" + re.escape(name) + r" \(", decl)
    ret = decl[len("extern "):m.start()].strip() if decl.startswith("extern ") else decl[:m.start()].strip()
    i, depth, start, params = m.end(), 1, m.end(), []
    while depth:
        c = decl[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                params.append(decl[start:i].strip())
        elif c == "," and depth == 1:
            params.append(decl[start:i].strip())
            start = i + 1
        i += 1
    variadic = params and params[-1] == "..."
    params = [p for p in params if p not in ("...", "void", "")]
    return ret, params, bool(variadic)


def call_exprs(name, decl):
    """The call test, then its fallback for a unit where the first does not compile: a pointer argument is
    `(void *)` on a buffer (no type spelled); a value argument is read from `lp_v` through its spelled type,
    and in the fallback is `0` (a constant: only used when the spelled type is not in scope)."""
    ret, params, _ = split_params(decl, name)
    out = []
    for value in (lambda p: f"*(__typeof__({p}) *)lp_v", lambda p: "0"):
        args = [f"(void *)lp_b{k}" if "*" in p or "(" in p else value(p) for k, p in enumerate(params)]
        call = f"{name}({', '.join(args)})"
        out.append(call if ret == "void" else f"*(volatile __typeof__({call}) *)lp_out = {call}")
    return out


CALL_HEAD = [f"extern char lp_b{k}[8] __attribute__((aligned(16)));" for k in range(NBUF)] + [
    "extern char lp_v[512] __attribute__((aligned(64)));", "extern char lp_out[512] __attribute__((aligned(64)));"]


def probe_sites(header, cid, mode, flags_lang, items, work):
    """{key: symbol string} for one (header, configuration, mode)."""
    lang, flags = flags_lang
    tag = f"{header.replace('/', '_')}.{cid.replace('+', '_')}.{mode}"
    src = os.path.join(work, tag + (".c" if lang == "c" else ".cc"))
    obj = os.path.join(work, tag + ".o")
    if mode == "addr":
        lines = [("raw", f"#include <{header}>")]
        for n in items:
            lines.append(("test", (n, f'void *lp_a_{n} __attribute__((section(".lpa.{n}"))) = (void *)&{n};')))
        kept, failed = compile_unit(lang, flags, src, lines, None, OBJ, obj)
        got = relocs(obj, ".lpa.") if kept else {}
        out = {n: "!error" for n in failed}
        for n in kept:
            out[n] = ",".join(got.get(n, [])) or "-"
        return out
    out, todo = {}, dict(items)
    for alt in range(2):
        lines = [("raw", f"#include <{header}>")] + [("raw", x) for x in CALL_HEAD]
        for n, exprs in todo.items():
            if alt < len(exprs):
                lines.append(("test", (n, f'void lp_c_{n}(void) __attribute__((section(".lpc.{n}"))); void lp_c_{n}(void) {{ {exprs[alt]}; }}')))
        kept, failed = compile_unit(lang, flags, f"{src}.{alt}", lines, None, OBJ, f"{obj}.{alt}")
        got = relocs(f"{obj}.{alt}", ".lpc.") if kept else {}
        for n in kept:
            out[n] = ",".join(got.get(n, [])) or "-"
        todo = {n: e for n, e in todo.items() if n in failed}
    out.update({n: "!error" for n in todo})
    return out


NX_HEAD = [
    "template<class R, class... A> char (&lp_k(R(*)(A...)))[1];",
    "template<class R, class... A> char (&lp_k(R(*)(A...) noexcept))[2];",
    "template<class R, class... A> char (&lp_k(R(*)(A..., ...)))[3];",
    "template<class R, class... A> char (&lp_k(R(*)(A..., ...) noexcept))[4];",
]
NX_SIZE = {1: ("none", "no"), 2: ("noexcept", "no"), 3: ("none", "yes"), 4: ("noexcept", "yes")}


def probe_cxx17(header, names, flags_lang, work):
    lang, flags = flags_lang
    src = os.path.join(work, header.replace("/", "_") + ".nx17.cc")
    obj = src[:-3] + ".o"
    lines = [("raw", f"#include <{header}>")] + [("raw", x) for x in NX_HEAD]
    for n in names:
        lines.append(("test", (n, f"char lp_n_{n}[sizeof(lp_k(&{n}))];")))
    kept, failed = compile_unit(lang, flags, src, lines, None, ["-c"], obj)
    out = {n: ("error", "-") for n in failed}
    if kept:
        for line in subprocess.run(["nm", "-S", "--defined-only", obj], capture_output=True, text=True, check=True).stdout.splitlines():
            f = line.split()
            if len(f) == 4 and f[3].startswith("lp_n_"):
                out[f[3][len("lp_n_"):]] = NX_SIZE[int(f[1], 16)]
    return out


def redeclaration(name, demangled, functions=frozenset()):
    """Redeclaration text of `name` from the demangled type of `&name`, as (declarator, rest), or None. The name
    is written `(name)` so that a function-like macro of the same name does not expand; no linkage
    specification, so the redeclaration keeps whatever language linkage the header gave the name."""
    # A va_list parameter decays to `__va_list_tag*`, which is not a name in scope: write its own type.
    demangled = demangled.replace("__va_list_tag*", "__builtin_va_list")
    # A tag that shares its name with a function of the header (`stat`, `statfs`) is hidden by that function:
    # the demangled `stat*` must be written `struct stat*`.
    demangled = re.sub(r"(?<![\w:])(\w+)(?=[\s*&,)]|$)", lambda m: "struct " + m.group(1) if m.group(1) in functions else m.group(1), demangled)
    i = demangled.find("(*)")
    if i < 0:
        return None
    text = demangled[:i] + "(" + name + ")" + demangled[i + 3:]
    j, depth = i + len(name) + 2, 0
    if j >= len(text) or text[j] != "(":
        return None
    while True:
        if text[j] == "(":
            depth += 1
        elif text[j] == ")":
            depth -= 1
            if depth == 0:
                break
        j += 1
    return text[:j + 1], text[j + 1:]


def probe_cxx98(header, names, flags_lang, plain_inc, functions, work):
    lang, flags = flags_lang
    base = os.path.join(work, header.replace("/", "_") + ".nx98")
    lines = [("raw", f"#include <{header}>")]
    for n in names:
        lines.append(("test", (n, f"void lp_t_{n}(__typeof__(&{n})) {{}}")))
    kept, failed = compile_unit(lang, flags, base + "t.cc", lines, None, ["-c"], base + "t.o")
    out = {n: "error" for n in failed}
    types = {}
    if kept:
        for line in subprocess.run(["nm", "-C", "--defined-only", base + "t.o"], capture_output=True, text=True, check=True).stdout.splitlines():
            m = re.match(r"^[0-9a-f]+ T lp_t_(\w+)\((.*)\)$", line)
            if m:
                types[m.group(1)] = m.group(2)
    decls = {}
    for n in kept:
        d = redeclaration(n, types.get(n, ""), functions)
        if d is None:
            out[n] = "error"
        else:
            decls[n] = d
    verdict = {}
    for spec in ("", " throw()"):
        unit = base + ("x" if spec else "p")
        lines = [("raw", f"#include <{header}>")]
        for n, (head, tail) in decls.items():
            lines.append(("test", (n, f"{head}{spec}{tail};")))
        try:
            ok, _ = compile_unit(lang, flags, unit + ".cc", lines, None, plain_inc)
        except RuntimeError:
            # A redeclaration can make the unit fail outside its own lines; then each name gets a unit of its
            # own, and only a name whose own unit fails that way counts as not compiling.
            ok = []
            for n, (head, tail) in decls.items():
                try:
                    one, _ = compile_unit(lang, flags, f"{unit}_{n}.cc", [lines[0], ("test", (n, f"{head}{spec}{tail};"))], None, plain_inc)
                except RuntimeError:
                    one = []
                ok += one
        for n in ok:
            verdict.setdefault(n, []).append(spec.strip() or "none")
    for n in decls:
        v = verdict.get(n, [])
        out[n] = v[0] if len(v) == 1 else "unknown" if v else "error"
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--names", default="")
    ap.add_argument("--out", default=ABI)
    args = ap.parse_args()
    names_filter = set(args.names.split(",")) if args.names else set()
    all_ids, cfg = configs()
    protos = {r["name"]: r["declaration"] for r in tsv("prototypes.tsv") if not names_filter or r["name"] in names_filter}
    fns = functions(all_ids, names_filter, {r["name"]: r["header"] for r in tsv("prototypes.tsv")})
    addr_ids = [i for i in all_ids if not i.startswith("fortify")]
    call_ids, call_cfg, call_vis = [], {}, {}
    for i in all_ids:
        if i.startswith("fortify"):
            for suffix, extra in OVERLAYS:
                call_ids.append(i + suffix)
                call_cfg[i + suffix] = (cfg[i][0], (cfg[i][1] + " " + extra).strip())
                call_vis[i + suffix] = "gnu" if suffix else i
    gccinc = subprocess.run(["g++", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    # -fno-builtin: GCC's own implicit declarations of library builtins (isalnum, abs) accept either specifier.
    plain_inc = ["-fno-builtin", "-nostdinc","-I" + gccinc, "-I/usr/include/x86_64-linux-gnu", "-I/usr/include"]
    # Every prototypes.tsv function: a type token with one of these names is a tag the function hides.
    all_functions = frozenset(r["name"] for r in tsv("prototypes.tsv"))
    jobs = []
    by_header = {}
    for n, (h, cs) in sorted(fns.items()):
        if n in protos:
            by_header.setdefault(h, []).append((n, cs))
    for h, items in sorted(by_header.items()):
        for c in addr_ids:
            names = [n for n, cs in items if c in cs]
            if names:
                jobs.append(("addr", h, c, cfg[c], names))
        for c in call_ids:
            names = [(n, call_exprs(n, protos[n])) for n, cs in items if call_vis[c] in cs]
            if names:
                jobs.append(("call", h, c, call_cfg[c], names))
        for std, label in (("cxx17", "nx17"), ("cxx98", "nx98")):
            names = [n for n, cs in items if std in cs]
            if names:
                jobs.append((label, h, std, cfg[std], names))
    # One unit per (header, configuration) for the macros too: every job writes its own file names.
    for h in sorted({m[1] for m in MACROS}):
        items = [(m, [e]) for m, mh, e in MACROS if mh == h and (not names_filter or m in names_filter)]
        if items:
            for c in call_ids:
                jobs.append(("macro", h, c, call_cfg[c], items))
    # A unit's file names come from (mode, header, configuration): two jobs sharing them would race.
    if len({j[:3] for j in jobs}) != len(jobs):
        raise RuntimeError("two probe jobs share a (mode, header, configuration)")
    work = tempfile.mkdtemp(prefix="callsite_probe.")

    def run(job):
        mode, h, c, fl, names = job
        if mode in ("addr", "call", "macro"):
            return probe_sites(h, c, "addr" if mode == "addr" else "call", fl, names, os.path.join(work, mode))
        if mode == "nx17":
            return probe_cxx17(h, names, fl, work)
        return probe_cxx98(h, names, fl, plain_inc, all_functions, work)

    for d in ("addr", "call", "macro"):
        os.makedirs(os.path.join(work, d))
    try:
        with ThreadPoolExecutor(max_workers=args.jobs) as ex:
            results = list(ex.map(run, jobs))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    sites, nx17, nx98 = {}, {}, {}
    for (mode, h, c, _, _), res in zip(jobs, results):
        for n, val in res.items():
            if mode in ("addr", "call", "macro"):
                hdr = h
                sites.setdefault((n, hdr, mode, val), set()).add(c)
            elif mode == "nx17":
                nx17[n] = val
            else:
                nx98[n] = val
    rows = []
    for (n, h, mode, sym), cs in sites.items():
        rows.append((n, h, mode, encode(cs, addr_ids if mode == "addr" else call_ids), sym))
    rows.sort(key=lambda r: (r[0], r[2], r[4]))
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "callsites.tsv"), "w", newline="\n") as f:
        f.write("name\theader\tmode\tconfigs\tsymbol\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    with open(os.path.join(args.out, "noexcept.tsv"), "w", newline="\n") as f:
        f.write("name\theader\tcxx17\tvariadic\tcxx98\n")
        for n in sorted(protos):
            h = fns[n][0] if n in fns else "-"
            c17, var = nx17.get(n, ("-", "-"))
            f.write(f"{n}\t{h}\t{c17}\t{var}\t{nx98.get(n, '-')}\n")
    errors = sum(1 for r in rows if r[4] == "!error")
    print(f"functions {len(protos)}  probed {sum(1 for n in protos if n in fns)}  jobs {len(jobs)}  callsite rows {len(rows)}"
          f"  error rows {errors}  noexcept: cxx17 {sorted(set(v[0] for v in nx17.values()))} cxx98 errors"
          f" {sum(1 for v in nx98.values() if v == 'error')}")


if __name__ == "__main__":
    sys.exit(main())
