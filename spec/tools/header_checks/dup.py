#!/usr/bin/env python3
"""DUP (HEADERS-FRAGMENTS): nothing is defined twice across libniji's installed headers.

For each configuration of spec/abi/ftm-configs.tsv that headers/levels.tsv does not reject, one translation unit
includes every header of the slice tables (except the `must_fail` ones), preprocessed with `-E -dD` against the
tree under check (`-nostdinc`, as every header_check unit). A FAIL names each of:

* macro       a `#define N` while N is still defined by an earlier `#define` (no `#undef N` between), identical
              text included (C accepts an identical redefinition silently; two stories that each add the name
              produce one);
* typedef     a second `typedef ... N` (C11 accepts an identical one silently);
* tag         a second `struct|union|enum N { ... }` body;
* enumerator  a second enumerator N.

Only definitions that both lie in the tree under check count (the compiler's own headers are P9's subject). The
typedef, tag and enumerator scan reads the C configurations' preprocessed text at file scope (function bodies are
skipped); in C++ a second definition of a tag or enumerator is an error the P5 units already see, and the C
configurations show the same header text. One FAIL per (kind, name), naming every file that defines it and the
configurations (the files of all configurations together, so the order of the includes does not matter).

Exceptions: only where the reference headers do the same, each in EXCEPTIONS with the black-box probe that shows it.
The duplicates on next when the check came (HEADERS-FRAGMENTS, 2026-11-26) are listed in headers/dup-listed.tsv
(kind, name, files): a listed name defined in exactly the listed files passes; one defined in other files, or a
listed row that no longer occurs, fails (the list only shrinks: remove a row when its duplicate is fixed).

    python3 spec/tools/header_checks/dup.py [--include DIR] [--rows]   # the check alone; --rows: every finding
                                                                       # as a dup-listed.tsv row on stdout
"""
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

# (kind, name) -> why the reference does the same (the probe that shows it). None yet.
EXCEPTIONS = {}
# The duplicates listed when the check came (see the module text), relative to the repository root.
LISTED = "headers/dup-listed.tsv"

TOKEN = re.compile(r"[A-Za-z_]\w*|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|\.?\d(?:[\w.]|[eEpP][+-])*|\S")
MARKER = re.compile(r'^# (\d+) "((?:\\.|[^"\\])*)"((?: \d)*)$')
DEFINE = re.compile(r"^#define ([A-Za-z_]\w*)")
UNDEF = re.compile(r"^#undef ([A-Za-z_]\w*)")
AGGREGATE = ("struct", "union", "enum")
# Groups dropped from a typedef's declarators: attributes, asm labels, typeof and _Atomic(type) specifiers.
GROUPS = {"__attribute__", "__attribute", "__asm__", "__asm", "asm", "__typeof__", "__typeof", "typeof", "_Atomic",
          "__declspec", "_Alignas", "alignas"}


def kernel_dir(work):
    """The kernel UAPI headers of the build host, as header_check.py links them (libniji ships none)."""
    kern = os.path.join(work, "dup-kernel-uapi")
    os.makedirs(kern, exist_ok=True)
    for d in ("linux", "asm-generic", "asm"):
        for base in ("/usr/include", "/usr/include/x86_64-linux-gnu"):
            src = os.path.join(base, d)
            if os.path.isdir(src) and not os.path.lexists(os.path.join(kern, d)):
                os.symlink(src, os.path.join(kern, d))
    return kern


def preprocess(cfg, inc, gccinc, kern, work, headers):
    cxx = cfg["language"] == "c++"
    src = os.path.join(work, f"dup-{cfg['id']}.{'cc' if cxx else 'c'}")
    with open(src, "w") as f:
        f.write("".join(f"#include <{h}>\n" for h in headers))
    cmd = ["g++" if cxx else "gcc", *cfg["flags"].split(), "-nostdinc", "-isystem", inc, "-isystem", gccinc,
           "-isystem", kern, "-E", "-dD", src]
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def skip_group(toks, i):
    """toks[i] names a group (`__attribute__ ((...))`): the index after its parenthesised operand."""
    j = i + 1
    if j < len(toks) and toks[j][0] == "(":
        depth = 0
        while j < len(toks):
            t = toks[j][0]
            depth += (t == "(") - (t == ")")
            j += 1
            if depth == 0:
                break
    return j


def declarator_name(part):
    """The name a declarator declares: inside a grouping `(` (`(*f)(int)`), else the last identifier before the
    first `(` or `[` (`int f(int)`, `char a[4]`, `unsigned long size_t`)."""
    end = len(part)
    for k, t in enumerate(part):
        if t == "(" and k + 1 < len(part) and part[k + 1] in ("*", "^", "("):
            for u in part[k + 1:]:
                if re.match(r"[A-Za-z_]", u) and u not in ("const", "volatile", "restrict", "__restrict", "__restrict__"):
                    return u
            return None
        if t in ("(", "["):
            end = k
            break
    names = [u for u in part[:end] if re.match(r"[A-Za-z_]", u)]
    return names[-1] if names else None


def typedef_names(decl):
    """The names a file-scope `typedef` declaration (its tokens, aggregate bodies removed) declares."""
    toks = [t for t in decl if t != "__extension__"]
    flat, i = [], 0
    while i < len(toks):
        if toks[i] in GROUPS:
            i = skip_group([(t,) for t in toks], i)
            continue
        flat.append(toks[i])
        i += 1
    parts, cur, depth = [], [], 0
    for t in flat[1:]:
        depth += (t in "([") - (t in ")]")
        if t == "," and depth == 0:
            parts.append(cur)
            cur = []
        else:
            cur.append(t)
    parts.append(cur)
    return [n for n in (declarator_name(p) for p in parts) if n]


def scan_c(toks):
    """Definitions in preprocessed C text: (kind, name, place) for typedef names, tag bodies and enumerators at file
    scope. toks: (token, place) in order."""
    out = []
    depth = 0
    frames = []           # one per open brace: "agg", "enum", "body" (function body), "init"
    decl = []             # file-scope tokens of the current declaration, aggregate bodies left out
    decl_place = None
    enum_expect = False
    paren = 0             # parenthesis depth inside the innermost enum body
    pending = None        # index of the `{` that opens a tag's body, and the tag
    i = 0
    n = len(toks)
    while i < n:
        t, place = toks[i]
        in_body = "body" in frames or "init" in frames
        if not in_body and t in AGGREGATE:
            j = i + 1
            while j < n and toks[j][0] in GROUPS | {"__extension__"}:
                j = skip_group(toks, j) if toks[j][0] in GROUPS else j + 1
            tag = None
            if j < n and re.match(r"[A-Za-z_]", toks[j][0]):
                tag, j = toks[j][0], j + 1
            if j < n and toks[j][0] == "{":
                pending = (j, t)
                if tag:
                    out.append(("tag", f"{t} {tag}", toks[j - 1][1]))
        if t == "{":
            if pending and pending[0] == i:
                kind = "enum" if pending[1] == "enum" else "agg"
                pending = None
            elif in_body:
                kind = "body"
            else:
                kind = "init" if (decl and decl[-1] == "=") else "body"
            frames.append(kind)
            depth += 1
            if kind == "enum":
                enum_expect, paren = True, 0
            i += 1
            continue
        if t == "}":
            closed = frames.pop() if frames else None
            depth -= 1
            if frames and frames[-1] == "enum":
                enum_expect, paren = False, 0
            if depth == 0 and closed == "body":
                decl, decl_place = [], None
            i += 1
            continue
        if frames and frames[-1] == "enum":
            if t == "(":
                paren += 1
            elif t == ")":
                paren -= 1
            elif t == "," and paren == 0:
                enum_expect = True
            elif enum_expect and re.match(r"[A-Za-z_]", t):
                out.append(("enumerator", t, place))
                enum_expect = False
            i += 1
            continue
        if depth == 0:
            if t == ";":
                body = [x for x in decl if x != "__extension__"]
                if body and body[0] == "typedef":
                    for name in typedef_names(decl):
                        out.append(("typedef", name, decl_place))
                decl, decl_place = [], None
            else:
                if not decl:
                    decl_place = place
                decl.append(t)
        i += 1
    return out


def analyse(text, inc, scan):
    """Duplicates in one configuration's `-E -dD` output: {(kind, name): {files, relative to inc}}."""
    prefix = inc.rstrip("/") + "/"
    pairs = set()
    macros = {}
    toks = []
    file, line = "", 0
    for raw in text.split("\n"):
        m = MARKER.match(raw)
        if m:
            file, line = m.group(2), int(m.group(1))
            continue
        here = (file, line)
        line += 1
        if raw.startswith("#"):
            d = DEFINE.match(raw)
            if d:
                name = d.group(1)
                if name in macros and macros[name][0].startswith(prefix) and file.startswith(prefix):
                    pairs.add(("macro", name, macros[name], here))
                macros[name] = here
                continue
            u = UNDEF.match(raw)
            if u:
                macros.pop(u.group(1), None)
            continue
        if scan and file.startswith(prefix):
            toks.extend((t, here) for t in TOKEN.findall(raw))
        elif scan and raw.strip():
            # The compiler's own headers: their definitions are no duplicate here, but their braces and
            # semicolons still delimit the declarations around them.
            toks.extend((t, None) for t in TOKEN.findall(raw))
    if scan:
        seen = {}
        for kind, name, place in scan_c(toks):
            if place is None:
                continue
            key = (kind, name)
            if key in seen:
                pairs.add((kind, name, seen[key], place))
            else:
                seen[key] = place
    dups = {}
    for kind, name, a, b in pairs:
        dups.setdefault((kind, name), set()).update(f[len(prefix):] for f, _ in (a, b))
    return dups


def find(inc, gccinc, work, cfgs, rejected, headers):
    """{(kind, name): (files, [config ids])} over every configuration (the files of every configuration's
    duplicate, so the result does not depend on the order of the includes), and the configurations whose unit did
    not preprocess (each a problem)."""
    kern = kernel_dir(work)
    run = [c for c in cfgs if c["id"] not in rejected]
    with ThreadPoolExecutor(max_workers=min(16, os.cpu_count() or 1)) as pool:
        outs = list(pool.map(lambda c: (c, preprocess(c, inc, gccinc, kern, work, headers)), run))
    found, broken = {}, []
    for c, (rc, text, err) in outs:
        if rc != 0:
            broken.append(f"DUP {c['id']}: the all-headers unit does not preprocess: rc {rc}: {' | '.join(err.strip().splitlines()[:3])[:300]}")
            continue
        for key, files in analyse(text, inc, c["language"] == "c").items():
            fs, ids = found.setdefault(key, (set(), []))
            fs.update(files)
            ids.append(c["id"])
    return found, broken


def listed(rows):
    """headers/dup-listed.tsv: {(kind, name): files} (the files comma-separated, sorted)."""
    return {(r["kind"], r["name"]): r["files"] for r in rows(LISTED)}


def report(found, known, all_ids):
    """One FAIL per duplicate not in EXCEPTIONS or listed with exactly its files, and per listed row that no longer
    occurs."""
    lines = []
    for (kind, name), (files, ids) in sorted(found.items()):
        have = ",".join(sorted(files))
        if (kind, name) in EXCEPTIONS or known.get((kind, name)) == have:
            continue
        cfgs = "every configuration" if len(ids) == len(all_ids) else ", ".join(ids)
        was = f"; {LISTED} lists it in {known[(kind, name)]}" if (kind, name) in known else ""
        lines.append(f"DUP {kind} {name}: defined in {', '.join(sorted(files))} ({cfgs}){was}")
    for (kind, name), files in sorted(known.items()):
        if (kind, name) not in found:
            lines.append(f"DUP {kind} {name}: {LISTED} lists it in {files}, but it is defined once now: remove the row")
    return lines


def problems(ctx):
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    levels = {r["id"]: r for r in ctx.rows("headers/levels.tsv")}
    rejected = {i for i, r in levels.items() if r["reject"] == "1"}
    headers = [r["header"] for r in ctx.slice_rows if r["header"] not in ctx.must_fail]
    found, broken = find(ctx.inc, gccinc, ctx.work, ctx.cfgs, rejected, headers)
    ids = [c["id"] for c in ctx.cfgs if c["id"] not in rejected]
    if getattr(ctx, "dump_rows", False):
        for (kind, name), (files, _) in sorted(found.items()):
            print(f"{kind}\t{name}\t{','.join(sorted(files))}")
    return broken + report(found, listed(ctx.rows), ids)


def main():
    import argparse
    import tempfile
    import types
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    import header_check as hc
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--include", default=os.path.join(hc.ROOT, "libc-include"))
    ap.add_argument("--rows", action="store_true")
    args = ap.parse_args()
    srows, _ = hc.slice_rows()
    must_fail = {x for _, mod in hc.slice_checks() for x in getattr(mod, "must_fail", ())}
    ctx = types.SimpleNamespace(rows=hc.rows, cfgs=hc.rows("spec/abi/ftm-configs.tsv"), slice_rows=srows,
                                must_fail=must_fail, inc=os.path.abspath(args.include),
                                work=tempfile.mkdtemp(prefix="dup."), dump_rows=args.rows)
    lines = problems(ctx)
    for p in lines:
        print(f"FAIL {p}", file=sys.stderr if args.rows else sys.stdout)
    print(f"dup: {len(lines)} failed", file=sys.stderr)
    return 1 if lines else 0


if __name__ == "__main__":
    sys.exit(main())
